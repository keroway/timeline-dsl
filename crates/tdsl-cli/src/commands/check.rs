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
        .map_err(|errs| render_lowering_errors(&errs, &source, &filename))?;

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

/// lowering エラー（E101〜）を miette のキャレット付きスニペットで表示する。
///
/// パースエラーは v1.14 でキャレット表示になったが、lowering エラーは
/// `to_string()` を join するだけで、大きいファイルでは該当行を探す手段が
/// 無かった（#760）。`SpannedLoweringError` が持つ位置を使って同じ体裁で出す。
///
/// **位置が無いエラー（`NoTimeline` 等、ファイル全体に対するもの）は
/// スニペット無しで出す。** 位置不明を先頭行と偽らない。
///
/// `Display` / `Error` は手で実装する。miette の `Diagnostic` derive は
/// `std::error::Error` を要求するが、そのためだけに tdsl-cli へ
/// thiserror を新規依存として足さない。
#[derive(Debug, miette::Diagnostic)]
#[diagnostic(
    code(tdsl::lowering_error),
    help("エラーカタログ docs/error-catalog.md を確認してください")
)]
pub(crate) struct LoweringDiagnostic {
    message: String,
    #[source_code]
    src: miette::NamedSource<String>,
    #[label("ここに問題があります")]
    span: Option<miette::SourceSpan>,
}

impl std::fmt::Display for LoweringDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LoweringDiagnostic {}

impl LoweringDiagnostic {
    pub(crate) fn new(
        err: &tdsl_core::error::SpannedLoweringError,
        src: &str,
        filename: &str,
    ) -> Self {
        Self {
            message: err.to_string(),
            src: miette::NamedSource::new(filename, src.to_string()),
            span: err
                .span
                .map(|s| miette::SourceSpan::from(s.start..s.end.max(s.start))),
        }
    }
}

/// lowering エラー列を miette 表示にまとめる。
///
/// エラーは複数まとめて返るため、1 件ずつスニペットを出して連結する。
pub(crate) fn render_lowering_errors(
    errs: &[tdsl_core::error::SpannedLoweringError],
    src: &str,
    filename: &str,
) -> String {
    errs.iter()
        .map(|e| {
            let report = miette::Report::new(LoweringDiagnostic::new(e, src, filename));
            format!("{report:?}")
        })
        .collect::<Vec<_>>()
        .join("\n")
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
