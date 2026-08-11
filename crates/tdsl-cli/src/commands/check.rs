use crate::CheckOutputFormat;
/// `check --format json` の 1 ファイル分のレポート。
///
/// `lint --format json` の `LintReportOutput` と同じ形（`file` / `ok` /
/// カウント + 診断の配列）に寄せてある（#748）。
#[derive(Debug, serde::Serialize)]
pub(crate) struct CheckReportOutput {
    pub file: String,
    pub lanes: usize,
    pub items: usize,
    pub unresolved_blocks: usize,
    pub warning_count: usize,
    pub ok: bool,
    pub diagnostics: Vec<CheckDiagnostic>,
}

/// 1 件の診断。`code` は `docs/error-catalog.md` に対応する安定した識別子。
#[derive(Debug, serde::Serialize)]
pub(crate) struct CheckDiagnostic {
    pub code: &'static str,
    pub severity: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub message: String,
}

/// .tdsl ファイルの構文・意味エラーをチェックする。
///
/// **`check` は常に offline（Pass 1/2 のみ）で動く。** import 解決（Pass 3）と
/// map 適用（Pass 4）は走らないため、`import` / `map` / `apply` を含む
/// ファイルではアイテムが生成されない。その旨を警告と完了行の両方で出す（#751）。
///
/// `offline` は現時点で唯一の動作なので受け取っても分岐しないが、
/// **フラグとして明示しておくことで「なぜアイテムが 0 件なのか」が
/// コマンドラインからも読める**（将来オンライン `check` を足す余地も残る）。
pub(crate) fn cmd_check(
    input: &std::path::Path,
    offline: bool,
    format: CheckOutputFormat,
    deny_warnings: bool,
    json_sink: Option<&mut Vec<CheckReportOutput>>,
) -> Result<(), String> {
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
    // `source` を渡すと各アイテムに source_span（行番号）が付き、
    // 診断に行番号を載せられる（#748）。以前は None を渡していたため
    // JSON の `line` が常に欠落していた。
    let (ir, lower_warnings) =
        tdsl_core::lower::lower_static_with_diagnostics(&file, Some(&source))
            .map_err(|errs| render_lowering_errors(&errs, &source, &filename))?;

    // 診断コードを添えて出す（#748）。CI で特定の警告だけを許容/禁止できる
    // ようにするため、機械可読な識別子を出力に含める。
    let mut diagnostics: Vec<CheckDiagnostic> = Vec::new();
    for w in &lower_warnings {
        // lowering の非致命的警告。offline の未解決ブロック（W211）が該当する。
        diagnostics.push(CheckDiagnostic {
            code: "W211",
            severity: "warning",
            line: None,
            message: w.clone(),
        });
    }
    for d in tdsl_core::validate::validate_with_spans(&ir) {
        diagnostics.push(CheckDiagnostic {
            code: d.code,
            severity: "warning",
            line: d.span.as_ref().map(|s| s.line as usize),
            message: d.message,
        });
    }

    if matches!(format, CheckOutputFormat::Text) {
        for d in &diagnostics {
            match d.line {
                Some(line) => eprintln!("Warning [{}] line {line}: {}", d.code, d.message),
                None => eprintln!("Warning [{}]: {}", d.code, d.message),
            }
        }
    }

    // 未解決ブロックがあるときは完了行にも出す。警告は他の警告に埋もれるが、
    // 完了行は必ず最後に出るため見落としにくい。
    let unresolved = count_unresolved_blocks(&file);
    let warning_count = diagnostics.len();

    if matches!(format, CheckOutputFormat::Json) {
        let report = CheckReportOutput {
            file: input.display().to_string(),
            lanes: ir.lanes.len(),
            items: ir.items.len(),
            unresolved_blocks: unresolved,
            warning_count: diagnostics.len(),
            ok: diagnostics.is_empty(),
            diagnostics,
        };
        match json_sink {
            Some(sink) => sink.push(report),
            None => println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            ),
        }
    } else if unresolved > 0 {
        eprintln!(
            "OK: {} lanes, {} items ({} block(s) unresolved: offline lowering does not run import/map)",
            ir.lanes.len(),
            ir.items.len(),
            unresolved
        );
    } else {
        eprintln!("OK: {} lanes, {} items", ir.lanes.len(), ir.items.len());
    }

    // `--deny-warnings` は警告があれば非ゼロ終了する。既定では従来どおり
    // 警告のみなら成功（`lint` の ERROR/WARN の扱いと揃えている、#766）。
    if deny_warnings && warning_count > 0 {
        return Err(format!(
            "{warning_count} warning(s) in {} (--deny-warnings)",
            input.display()
        ));
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
