/// JSON IR を .tdsl ソースファイルへ逆変換する。
pub(crate) fn cmd_decompile(
    input: Option<&std::path::Path>,
    output: Option<&std::path::Path>,
) -> Result<(), String> {
    let json_str = match input {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?,
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("Failed to read stdin: {e}"))?;
            buf
        }
    };

    let ir: tdsl_core::ir::TimelineIr =
        serde_json::from_str(&json_str).map_err(|e| format!("Invalid IR JSON: {e}"))?;
    // decompile 自体は IR を検証しない。無効な lane ID など、出力が再パースできなくなる
    // 原因を利用者が気付けるよう、build と同じ検証警告を stderr に出す（#961）。
    for w in tdsl_core::validate::validate(&ir) {
        eprintln!("Warning: {w}");
    }
    let tdsl = tdsl_core::decompile::decompile(&ir);

    if let Some(out_path) = output {
        std::fs::write(out_path, &tdsl)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        print!("{tdsl}");
    }

    Ok(())
}
