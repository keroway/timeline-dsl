/// .tdsl ファイルを正準フォーマットする。
///
/// フォーマットには `tdsl_parser::format_source` を使用する。
/// これは WebUI の Format ボタン / `tdsl lint --fix` と同一の emitter であり、
/// 2 スペースインデント・ブロック間空行 1 行を出力する。
///
/// **コメントの扱い**: トップレベルのコメント（`//`・`/* */`）は位置を保ったまま保持される。
/// ブロック内部のコメントは内容を保持しつつブロック境界に移動する（#473）。
/// なお `tdsl decompile` は IR 起点のためコメントを復元できない（#474）。
pub(crate) fn cmd_fmt(input: &std::path::Path, check: bool, write: bool) -> Result<(), String> {
    // --check と --write は排他（clap の conflicts_with で保証済みだが念のため確認）
    if check && write {
        return Err("--check and --write cannot be used together".to_string());
    }

    let source = super::read_source(input)?;
    let formatted = tdsl_parser::format_source(&source).map_err(|e| e.to_string())?;

    if check {
        if source == formatted {
            Ok(())
        } else {
            Err(format!("File is not formatted: {}", input.display()))
        }
    } else if write {
        if source != formatted {
            std::fs::write(input, &formatted)
                .map_err(|e| format!("Failed to write {}: {e}", input.display()))?;
        }
        Ok(())
    } else {
        print!("{formatted}");
        Ok(())
    }
}
