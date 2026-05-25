//! `tdsl-lsp` — Timeline DSL Language Server Protocol implementation.
//!
//! このクレートは `tower-lsp` を使って LSP サーバを提供する。
//!
//! 現バージョン（v1.11.x）の機能範囲: Diagnostics + Completion。
//!
//! - `textDocument/didOpen` / `didChange`: パースエラー・バリデーション警告を `publishDiagnostics` で返す
//! - `textDocument/didClose`: 診断をクリア
//! - `textDocument/completion`: DSL キーワード補完候補を返す（文脈非依存・全キーワード）
//!
//! Hover / Goto / Code Action は将来の別 issue で実装する。

pub mod backend;
pub mod completion;
pub mod diagnostics;
pub mod keywords;

pub use backend::run_server;
