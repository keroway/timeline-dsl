/// .tdsl ファイルを正準フォーマットする。
///
/// フォーマットには `tdsl_parser::format_source` を使用する。
/// これは WebUI の Format ボタン / `tdsl lint --fix` と同一の emitter であり、
/// 2 スペースインデント・ブロック間空行 1 行を出力する。
///
/// **制約**: 現状フォーマットするとコメント（`//`・`/* */`）は失われます（grammar で
/// COMMENT が silent のため）。根治は別 issue で対応予定。
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
