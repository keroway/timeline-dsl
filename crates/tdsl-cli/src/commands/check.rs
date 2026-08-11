/// .tdsl ファイルの構文・意味エラーをチェックする。
///
/// **`check` は常に offline（Pass 1/2 のみ）で動く。** import 解決（Pass 3）と
/// map 適用（Pass 4）は走らないため、`import` / `map` / `apply` を含む
/// ファイルではアイテムが生成されない。その旨を警告と完了行の両方で出す（#751）。
///
/// `offline` は現時点で唯一の動作なので受け取っても分岐しないが、
/// **フラグとして明示しておくことで「なぜアイテムが 0 件なのか」が
/// コマンドラインからも読める**（将来オンライン `check` を足す余地も残る）。
pub(crate) fn cmd_check(input: &std::path::Path, offline: bool) -> Result<(), String> {
    // 現状 check はオンライン経路を持たないため、`--offline` の有無で
    // 挙動は変わらない。指定されていないときに黙って offline 扱いするのではなく、
    // 下の完了行で毎回「offline」と明示する。
    let _ = offline;
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

    // 未解決ブロックがあるときは完了行にも出す。警告は他の警告に埋もれるが、
    // 完了行は必ず最後に出るため見落としにくい。
    let unresolved = count_unresolved_blocks(&file);
    if unresolved > 0 {
        eprintln!(
            "OK: {} lanes, {} items ({} block(s) unresolved: offline lowering does not run import/map)",
            ir.lanes.len(),
            ir.items.len(),
            unresolved
        );
    } else {
        eprintln!("OK: {} lanes, {} items", ir.lanes.len(), ir.items.len());
    }
    Ok(())
}

/// offline lowering で処理されないブロック（`import` / `map` / `apply`）の総数。
fn count_unresolved_blocks(file: &tdsl_parser::ast::File) -> usize {
    file.statements
        .iter()
        .filter(|stmt| {
            matches!(
                stmt.node,
                tdsl_parser::ast::Statement::Import(_)
                    | tdsl_parser::ast::Statement::Map(_)
                    | tdsl_parser::ast::Statement::Apply(_)
            )
        })
        .count()
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
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("{message}")]
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
