//! `tdsl-lsp` — Timeline DSL Language Server Protocol implementation.
//!
//! このクレートは `tower-lsp` を使って LSP サーバを提供する。
//!
//! 現バージョンの機能範囲: Diagnostics + Completion + Hover + Goto Definition + Code Action。
//!
//! - `textDocument/didOpen` / `didChange`: パースエラー・バリデーション警告を `publishDiagnostics` で返す
//! - `textDocument/didClose`: 診断をクリア
//! - `textDocument/completion`: DSL キーワード補完候補を返す（文脈非依存・全キーワード）
//! - `textDocument/hover`: lane ID → lane 情報、QID → キャッシュ済みエンティティ情報を返す
//! - `textDocument/definition`: lane 参照位置 → lane 宣言位置へのジャンプ
//! - `textDocument/codeAction`: `tdsl lint --fix` 相当の自動修正を quick fix として提供

pub mod backend;
pub mod code_action;
pub mod completion;
pub mod diagnostics;
pub mod goto_definition;
pub mod hover;
pub mod keywords;

pub use backend::run_server;
