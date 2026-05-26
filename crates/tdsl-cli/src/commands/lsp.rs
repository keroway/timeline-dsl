/// LSP サーバを stdio 経由で起動する。
///
/// tokio ランタイムを構築し、`tdsl_lsp::run_server()` を呼ぶ薄いラッパ。
/// `tokio::spawn` は tower-lsp の Server 内部で使われるため CLI 層で扱う。
pub(crate) fn cmd_lsp() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(tdsl_lsp::run_server());
    Ok(())
}
