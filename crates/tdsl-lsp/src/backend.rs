//! LSP サーバの Backend 実装。
//!
//! `tower-lsp` の `LanguageServer` trait を実装し、stdio 経由で LSP クライアントと通信する。
//! 現バージョンで実装している機能: Diagnostics + Completion + Hover + Goto Definition + Code Action。

use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    CodeActionParams, CodeActionProviderCapability, CodeActionResponse, CompletionOptions,
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams, MessageType,
    OneOf, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::code_action::compute_code_actions;
use crate::completion::keyword_completions;
use crate::diagnostics::compute_diagnostics;
use crate::goto_definition::compute_goto_definition;
use crate::hover::compute_hover;

/// LSP サーバの Backend 状態。
///
/// FULL sync モードのため毎回全文が送られてくる。
/// ドキュメントごとのバージョン管理は不要だが、URI ごとにテキストを保持する。
struct Backend {
    client: Client,
    /// URI → 現在のドキュメント全文のマップ。
    /// `Mutex` で保護し、各通知ハンドラで排他的に更新する。
    documents: Mutex<HashMap<String, String>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            // Mutex::new は常に成功するため、初期化時の unwrap は安全
            documents: Mutex::new(HashMap::new()),
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
    async fn initialize(&self, _params: InitializeParams) -> LspResult<InitializeResult> {
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
                "tdsl LSP server initialized (Diagnostics + Completion + Hover + Goto Definition + Code Action)",
            )
            .await;
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).cloned()
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
            docs.get(uri.as_str()).cloned()
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
        let source = {
            // LSP サーバの単一スレッド文脈では panic しない
            let docs = self.documents.lock().expect("documents lock poisoned");
            docs.get(uri.as_str()).cloned()
        };
        match source {
            Some(src) => Ok(Some(compute_code_actions(&src, &uri, range))),
            None => Ok(Some(Vec::new())),
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
            docs.insert(uri.to_string(), text.clone());
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
            docs.insert(uri.to_string(), text.clone());
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
