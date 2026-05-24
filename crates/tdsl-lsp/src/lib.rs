//! `tdsl-lsp` — Timeline DSL Language Server Protocol implementation.
//!
//! このクレートは `tower-lsp` を使って LSP サーバを提供する。
//!
//! 現バージョン（v1.10.x）の機能範囲: Diagnostics のみ。
//!
//! - `textDocument/didOpen` / `didChange`: パースエラー・バリデーション警告を `publishDiagnostics` で返す
//! - `textDocument/didClose`: 診断をクリア
//!
//! Completion / Hover / Goto / Code Action は将来の別 issue で実装する。

pub mod backend;
pub mod diagnostics;

pub use backend::run_server;
