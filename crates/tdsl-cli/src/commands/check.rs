/// .tdsl ファイルの構文・意味エラーをチェックする。
pub(crate) fn cmd_check(input: &std::path::Path) -> Result<(), String> {
    let source = super::read_source(input)?;
    let file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;
    let ir = tdsl_core::lower::lower_static(&file).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let warnings = tdsl_core::validate::validate(&ir);
    for w in &warnings {
        eprintln!("Warning: {w}");
    }

    eprintln!("OK: {} lanes, {} items", ir.lanes.len(), ir.items.len());
    Ok(())
}

/// パースした AST をデバッグ形式で表示する。
pub(crate) fn cmd_ast(input: &std::path::Path) -> Result<(), String> {
    let source = super::read_source(input)?;
    let file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;
    println!("{file:#?}");
    Ok(())
}
