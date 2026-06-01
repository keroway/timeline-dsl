//! LSP サーバの Backend 実装。
//!
//! `tower-lsp` の `LanguageServer` trait を実装し、stdio 経由で LSP クライアントと通信する。
//! 現バージョンで実装している機能: Diagnostics + Completion + Hover + Goto Definition + Code Action + Document Symbols + Find References + Rename + Formatting。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tower_lsp::jsonrpc::{self, Result as LspResult};
use tower_lsp::lsp_types::{
    CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionOptions,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
    DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, Location,
    MessageType, OneOf, PrepareRenameResponse, ReferenceParams, RenameOptions, RenameParams,
    ServerCapabilities, TextDocumentPositionParams, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit, Url, WorkspaceEdit,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::code_action::compute_code_actions;
use crate::completion::keyword_completions;
use crate::diagnostics::compute_diagnostics;
use crate::document_symbols::compute_document_symbols;
use crate::find_references::compute_references;
use crate::formatting::compute_formatting;
use crate::goto_definition::compute_goto_definition;
use crate::hover::compute_hover;
use crate::rename::{compute_prepare_rename, compute_rename};

/// 1 ドキュメントの保持状態（全文 + LSP バージョン）。
///
/// バージョンは Code Action のバージョン付き `documentChanges` で使い、
/// 計算後に編集されたドキュメントへの stale な全文置換適用を client に拒否させる。
#[derive(Clone)]
struct DocumentState {
    text: String,
    version: i32,
}

/// LSP サーバの Backend 状態。
///
/// FULL sync モードのため毎回全文が送られてくる。URI ごとに全文とバージョンを保持する。
struct Backend {
    client: Client,
    /// URI → 現在のドキュメント状態（全文 + バージョン）のマップ。
    /// `Mutex` で保護し、各通知ハンドラで排他的に更新する。
    documents: Mutex<HashMap<String, DocumentState>>,
    /// client が `workspace.workspaceEdit.documentChanges` をサポートするか。
    /// `initialize` で受け取った capability から設定し、Code Action の `WorkspaceEdit`
    /// 構築方法（versioned documentChanges / 非バージョンの changes）を切り替える。
    supports_document_changes: AtomicBool,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            // Mutex::new は常に成功するため、初期化時の unwrap は安全
            documents: Mutex::new(HashMap::new()),
            // initialize で client capability に基づき更新する（既定は安全側の false）
            supports_document_changes: AtomicBool::new(false),
        }
    }

    /// 指定 URI のドキュメントを診断して `publishDiagnostics` を送信する。
    async fn run_diagnostics(&self, uri: Url, source: String, version: Option<i32>) {
        let diags = compute_diagnostics(&source);
        self.client.publish_diagnostics(uri, diags, version).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        // client が WorkspaceEdit の documentChanges をサポートするか記録する。
        // 非対応なら Code Action は changes フォールバックで返す（versioned 不可）。
        let supports_document_changes = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.workspace_edit.as_ref())
            .and_then(|we| we.document_changes)
            .unwrap_or(false);
        self.supports_document_changes
            .store(supports_document_changes, Ordering::Relaxed);

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // FULL sync: 毎回全文を受け取る（シンプルで確実）
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // キーワード補完（文脈非依存・全キーワード返却）
                completion_provider: Some(CompletionOptions::default()),
                // lane ID / QID のホバー情報表示
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                // lane 参照 → lane 宣言位置へのジャンプ
                definition_provider: Some(OneOf::Left(true)),
                // lint auto-fix の quick fix 提供
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                // アウトライン / ブレッドクラム / シンボル検索のためのドキュメントシンボル
                document_symbol_provider: Some(OneOf::Left(true)),
                // lane ID の全参照位置を返す
                references_provider: Some(OneOf::Left(true)),
                // lane ID のリネーム（宣言と全参照を一括置換）
                // prepare_provider: true を指定して prepareRename を有効化する
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                // ドキュメント全体のフォーマット（2 スペースインデント・ブロック間空行 1 行）
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        // 初期化完了ログ
        self.client
            .log_message(
                MessageType::INFO,
                "tdsl LSP server initialized (Diagnostics + Completion + Hover + Goto Definition + Code Action + Document Symbols + Find References + Rename + Formatting)",
            )
            .await;
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => Ok(compute_hover(&src, position)),
            None => Ok(None),
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => Ok(compute_goto_definition(&src, position, &uri)),
            None => Ok(None),
        }
    }

    async fn completion(&self, _params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(keyword_completions())))
    }

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let range = params.range;
        let doc = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).cloned()
        };
        let supports_document_changes = self.supports_document_changes.load(Ordering::Relaxed);
        match doc {
            Some(d) => Ok(Some(compute_code_actions(
                &d.text,
                &uri,
                d.version,
                supports_document_changes,
                range,
            ))),
            None => Ok(Some(Vec::new())),
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => {
                let syms = compute_document_symbols(&src);
                Ok(Some(DocumentSymbolResponse::Nested(syms)))
            }
            None => Ok(None),
        }
    }

    async fn references(&self, params: ReferenceParams) -> LspResult<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => Ok(compute_references(
                &src,
                position,
                include_declaration,
                &uri,
            )),
            None => Ok(None),
        }
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> LspResult<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => {
                Ok(compute_prepare_rename(&src, position).map(PrepareRenameResponse::Range))
            }
            None => Ok(None),
        }
    }

    async fn rename(&self, params: RenameParams) -> LspResult<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let new_name = params.new_name.clone();
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => match compute_rename(&src, position, &new_name, &uri) {
                Ok(edit) => Ok(Some(edit)),
                Err(msg) => Err(jsonrpc::Error {
                    // LSP spec §3.16: rename 拒否には ServerError(-32803) を使う
                    code: jsonrpc::ErrorCode::ServerError(-32803),
                    message: msg.into(),
                    data: None,
                }),
            },
            None => Ok(None),
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).map(|d| d.text.clone())
        };
        match source {
            Some(src) => Ok(compute_formatting(&src)),
            None => Ok(None),
        }
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        // documents マップを更新し、診断を実行する
        {
            // Mutex のロック取得。LSP サーバの単一スレッド文脈では panic しない
            let mut docs = self.documents.lock().expect("documents lock poisoned");
            docs.insert(
                uri.to_string(),
                DocumentState {
                    text: text.clone(),
                    version,
                },
            );
        }

        self.run_diagnostics(uri, text, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // FULL sync のため content_changes は常に 1 件（全文）
        let text = match params.content_changes.into_iter().next() {
            Some(change) => change.text,
            // 変更が空の場合はスキップ（プロトコル違反だが silent fallback より明示ログを出す）
            None => {
                self.client
                    .log_message(MessageType::WARNING, "did_change: empty content_changes")
                    .await;
                return;
            }
        };

        {
            let mut docs = self.documents.lock().expect("documents lock poisoned");
            docs.insert(
                uri.to_string(),
                DocumentState {
                    text: text.clone(),
                    version,
                },
            );
        }

        self.run_diagnostics(uri, text, Some(version)).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        {
            let mut docs = self.documents.lock().expect("documents lock poisoned");
            docs.remove(&uri.to_string());
        }

        // ドキュメントを閉じたら診断をクリア（空 vec を publish）
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}

/// stdio 経由で LSP サーバを起動する。
///
/// `tdsl lsp` サブコマンドから呼ばれる。
/// tokio runtime は呼び出し元（CLI 層）で構築すること。
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
