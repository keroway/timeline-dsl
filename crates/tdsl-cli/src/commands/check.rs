/// .tdsl ファイルの構文・意味エラーをチェックする。
pub(crate) fn cmd_check(input: &std::path::Path) -> Result<(), String> {
    let source = super::read_source(input)?;
    let filename = input.display().to_string();
    let file = tdsl_parser::parse(&source).map_err(|e| {
        print_parse_error(&e, &source, &filename);
        // miette 出力済みのためメッセージは空にして重複を避ける
        String::new()
    })?;
    let (ir, lower_warnings) = tdsl_core::lower::lower_static_with_diagnostics(&file, None)
        .map_err(|errs| {
            errs.iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        })?;

    for w in &lower_warnings {
        eprintln!("Warning: {w}");
    }

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
    let filename = input.display().to_string();
    let file = tdsl_parser::parse(&source).map_err(|e| {
        print_parse_error(&e, &source, &filename);
        String::new()
    })?;
    println!("{file:#?}");
    Ok(())
}

/// `ParseError` から miette のキャレット付き診断文字列を生成する。
pub(crate) fn render_parse_error(
    err: &tdsl_parser::error::ParseError,
    src: &str,
    filename: &str,
) -> String {
    let diag = tdsl_parser::ParseDiagnostic::from_parse_error(err, src, filename);
    let report = miette::Report::new(diag);
    format!("{report:?}")
}

/// `ParseError` を miette のキャレット付きスニペットで標準エラー出力に書き出す。
pub(crate) fn print_parse_error(err: &tdsl_parser::error::ParseError, src: &str, filename: &str) {
    eprintln!("{}", render_parse_error(err, src, filename));
}
