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
use crate::completion::{contextual_completions, keyword_completions};
use crate::diagnostics::compute_diagnostics;
use crate::document_symbols::compute_document_symbols;
use crate::find_references::compute_references;
use crate::formatting::compute_formatting;
use crate::goto_definition::compute_goto_definition;
use crate::hover::compute_hover_with;
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
    /// QID → キャッシュ済みエンティティ（未取得なら `None`）のメモ。
    ///
    /// hover は QID にカーソルを置くたびに呼ばれ、その都度キャッシュ
    /// ディレクトリを読みに行っていた（#770）。**見つからなかったことも
    /// 記憶する**（`Option` を値に持つ）— キャッシュ未取得の QID こそ
    /// フォールバックの全走査に落ちて最も高くつくため。
    ///
    /// 有効期間はセッション中。`read_cached_entity` は元々 TTL を無視して
    /// 「古くても取得済み情報を見せる」設計なので、セッション内で固定しても
    /// 表示の性質は変わらない。エディタを開き直せば読み直される。
    entity_cache: Mutex<HashMap<String, Option<tdsl_wikidata::WikidataEntity>>>,
}

/// メモ経由で引き当てる。**取得できなかったことも記憶する。**
///
/// `Backend::cached_entity` の中身をここへ出しているのは、`Backend` の構築に
/// `Client`（稼働中の LSP サービス）が要り、メモ化の挙動を単体で検証できない
/// ため。fetch を注入可能にして「2 回目は fetch が呼ばれない」を固定する。
fn lookup_memoized(
    cache: &Mutex<HashMap<String, Option<tdsl_wikidata::WikidataEntity>>>,
    qid: &str,
    fetch: impl FnOnce(&str) -> Option<tdsl_wikidata::WikidataEntity>,
) -> Option<tdsl_wikidata::WikidataEntity> {
    if let Some(hit) = cache.lock().unwrap_or_else(|p| p.into_inner()).get(qid) {
        return hit.clone();
    }
    let fetched = fetch(qid);
    cache
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(qid.to_string(), fetched.clone());
    fetched
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            // Mutex::new は常に成功するため、初期化時の unwrap は安全
            documents: Mutex::new(HashMap::new()),
            entity_cache: Mutex::new(HashMap::new()),
            // initialize で client capability に基づき更新する（既定は安全側の false）
            supports_document_changes: AtomicBool::new(false),
        }
    }

    /// `documents` のロックを取得する。**poison していても回復して続行する。**
    ///
    /// 以前は 12 箇所で `.expect("documents lock poisoned")` していた（#771）。
    /// コメントには「単一スレッド文脈では panic しない」とあったが不正確で、
    /// `LspService` はマルチスレッド runtime 上で動く。どれか 1 ハンドラが
    /// ロック保持中に panic すると Mutex が poison し、以後**すべての**リクエストが
    /// `expect` で panic し続けてサーバが実質死ぬ。
    ///
    /// ここで扱うのはエディタが送ってくるドキュメント本文のキャッシュであり、
    /// 途中まで書き込まれた不整合状態が残っても次の `didChange` で上書きされる。
    /// **サーバごと止めるより、poison を無視して動き続ける方が損害が小さい。**
    fn documents_lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, DocumentState>> {
        self.documents.lock().unwrap_or_else(|poisoned| {
            // 復旧したことを観測できるようにログへ残す（silent に握り潰さない）。
            eprintln!(
                "warning: documents mutex was poisoned; recovering and continuing (see #771)"
            );
            poisoned.into_inner()
        })
    }

    /// 指定 URI のドキュメント本文を取得する。
    ///
    /// 12 箇所に散っていた「ロックを取って `get` して `clone`」を 1 箇所に集約する。
    /// QID のエンティティを、セッション内メモ経由で取得する（#770）。
    ///
    /// 未取得（`None`）もそのまま記憶する。キャッシュに無い QID は
    /// `read_cached_entity` のフォールバック全走査に落ちて最も高くつくため、
    /// そこを毎回払わないことがこの変更の主目的。
    fn cached_entity(&self, qid: &str) -> Option<tdsl_wikidata::WikidataEntity> {
        lookup_memoized(&self.entity_cache, qid, |q| {
            tdsl_wikidata::read_cached_entity(&tdsl_wikidata::default_cache_dir(), q)
        })
    }

    fn document_text(&self, uri: &str) -> Option<String> {
        self.documents_lock().get(uri).map(|d| d.text.clone())
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
        // silent な unwrap_or(false) を避け、false に倒れた理由を観測可能にする。
        let support = resolve_document_changes_support(&params);
        let supports_document_changes = support.is_supported();
        self.supports_document_changes
            .store(supports_document_changes, Ordering::Relaxed);
        if !supports_document_changes {
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "client does not support WorkspaceEdit.documentChanges ({support:?}); \
                         code actions will use the non-versioned `changes` fallback"
                    ),
                )
                .await;
        }

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
        let source = self.document_text(uri.as_str());
        match source {
            Some(src) => Ok(compute_hover_with(&src, position, |qid| {
                self.cached_entity(qid)
            })),
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
        let source = self.document_text(uri.as_str());
        match source {
            Some(src) => Ok(compute_goto_definition(&src, position, &uri)),
            None => Ok(None),
        }
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let text = self.document_text(uri.as_str());
        let completions = match text {
            Some(src) => contextual_completions(&src, position.line, position.character),
            None => keyword_completions(),
        };
        Ok(Some(CompletionResponse::Array(completions)))
    }

    async fn code_action(&self, params: CodeActionParams) -> LspResult<Option<CodeActionResponse>> {
        let uri = params.text_document.uri.clone();
        let range = params.range;
        let doc = self.documents_lock().get(uri.as_str()).cloned();
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
        let source = self.document_text(uri.as_str());
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
        let source = self.document_text(uri.as_str());
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
        let source = self.document_text(uri.as_str());
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
        let source = self.document_text(uri.as_str());
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
        let source = self.document_text(uri.as_str());
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
            let mut docs = self.documents_lock();
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
            let mut docs = self.documents_lock();
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
            let mut docs = self.documents_lock();
            docs.remove(&uri.to_string());
        }

        // ドキュメントを閉じたら診断をクリア（空 vec を publish）
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}

/// クライアントの `WorkspaceEdit.documentChanges` サポート状況の解決結果。
///
/// silent な `unwrap_or(false)` を避け、`false` に倒れた理由を区別して観測可能にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentChangesSupport {
    /// クライアントが明示的に `true` を申告。versioned な `documentChanges` を返せる。
    Supported,
    /// クライアントが明示的に `false` を申告。`changes` フォールバックを使う。
    ExplicitlyUnsupported,
    /// capability が未申告（`workspace` / `workspace_edit` / `document_changes` の
    /// いずれかが `None`）。LSP 仕様上は非対応扱いとし `changes` フォールバックを使う。
    Unspecified,
}

impl DocumentChangesSupport {
    fn is_supported(self) -> bool {
        matches!(self, DocumentChangesSupport::Supported)
    }
}

/// `InitializeParams` から `documentChanges` サポート状況を解決する。
///
/// `client.capabilities.workspace.workspace_edit.document_changes` の有無と値を
/// 明示的に場合分けし、握り潰しを避ける。
fn resolve_document_changes_support(params: &InitializeParams) -> DocumentChangesSupport {
    match params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.workspace_edit.as_ref())
        .map(|we| we.document_changes)
    {
        Some(Some(true)) => DocumentChangesSupport::Supported,
        Some(Some(false)) => DocumentChangesSupport::ExplicitlyUnsupported,
        // workspace_edit はあるが document_changes が None、
        // または workspace / workspace_edit 自体が None。
        Some(None) | None => DocumentChangesSupport::Unspecified,
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

#[cfg(test)]
mod tests {

    // ─── QID 引き当てのメモ化（#770）────────────────────────────────────

    fn dummy_entity(id: &str) -> tdsl_wikidata::WikidataEntity {
        tdsl_wikidata::WikidataEntity {
            id: id.to_string(),
            labels: Default::default(),
            claims: Default::default(),
        }
    }

    /// 2 回目以降は fetch を呼ばない。hover は QID にカーソルを置くたびに
    /// 呼ばれるため、ここが効かないとキャッシュディレクトリを毎回読みに行く。
    #[test]
    fn memoized_lookup_fetches_only_once_per_qid() {
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let calls = std::cell::Cell::new(0);

        for _ in 0..5 {
            let got = super::lookup_memoized(&cache, "Q42", |q| {
                calls.set(calls.get() + 1);
                Some(dummy_entity(q))
            });
            assert_eq!(got.expect("見つかるべき").id, "Q42");
        }
        assert_eq!(calls.get(), 1, "fetch が複数回呼ばれた");
    }

    /// **見つからなかったことも記憶する。** キャッシュ未取得の QID は
    /// フォールバックの全走査に落ちて最も高くつくため、ここを毎回払わない
    /// ことがこの変更の主目的。
    #[test]
    fn memoized_lookup_remembers_misses() {
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let calls = std::cell::Cell::new(0);

        for _ in 0..5 {
            let got = super::lookup_memoized(&cache, "Q999999", |_| {
                calls.set(calls.get() + 1);
                None
            });
            assert!(got.is_none());
        }
        assert_eq!(calls.get(), 1, "miss が記憶されていない");
    }

    /// QID ごとに独立して記憶する（取り違えない）。
    #[test]
    fn memoized_lookup_is_keyed_by_qid() {
        let cache = std::sync::Mutex::new(std::collections::HashMap::new());
        let a = super::lookup_memoized(&cache, "Q1", |q| Some(dummy_entity(q)));
        let b = super::lookup_memoized(&cache, "Q2", |q| Some(dummy_entity(q)));
        assert_eq!(a.unwrap().id, "Q1");
        assert_eq!(b.unwrap().id, "Q2");
    }

    use super::{DocumentChangesSupport, resolve_document_changes_support};
    use tower_lsp::lsp_types::{
        ClientCapabilities, InitializeParams, WorkspaceClientCapabilities,
        WorkspaceEditClientCapabilities,
    };

    /// `document_changes` を指定した `InitializeParams` を組み立てる。
    /// `workspace` / `workspace_edit` の有無も併せて制御する。
    fn params(
        with_workspace: bool,
        with_workspace_edit: bool,
        document_changes: Option<bool>,
    ) -> InitializeParams {
        let workspace = with_workspace.then(|| WorkspaceClientCapabilities {
            workspace_edit: with_workspace_edit.then(|| WorkspaceEditClientCapabilities {
                document_changes,
                ..Default::default()
            }),
            ..Default::default()
        });
        InitializeParams {
            capabilities: ClientCapabilities {
                workspace,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn explicit_true_is_supported() {
        let p = params(true, true, Some(true));
        assert_eq!(
            resolve_document_changes_support(&p),
            DocumentChangesSupport::Supported
        );
        assert!(resolve_document_changes_support(&p).is_supported());
    }

    #[test]
    fn explicit_false_is_explicitly_unsupported() {
        let p = params(true, true, Some(false));
        assert_eq!(
            resolve_document_changes_support(&p),
            DocumentChangesSupport::ExplicitlyUnsupported
        );
        assert!(!resolve_document_changes_support(&p).is_supported());
    }

    #[test]
    fn missing_document_changes_is_unspecified() {
        let p = params(true, true, None);
        assert_eq!(
            resolve_document_changes_support(&p),
            DocumentChangesSupport::Unspecified
        );
    }

    #[test]
    fn missing_workspace_edit_is_unspecified() {
        let p = params(true, false, None);
        assert_eq!(
            resolve_document_changes_support(&p),
            DocumentChangesSupport::Unspecified
        );
    }

    #[test]
    fn missing_workspace_is_unspecified() {
        let p = params(false, false, None);
        assert_eq!(
            resolve_document_changes_support(&p),
            DocumentChangesSupport::Unspecified
        );
        assert!(!resolve_document_changes_support(&p).is_supported());
    }
}
