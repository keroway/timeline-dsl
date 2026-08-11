use serde::Serialize;
use tdsl_core::lint::{LintIssue, LintSeverity, apply_lint_fixes, lint_issues};

use crate::LintOutputFormat;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LintReportOutput {
    file: String,
    fix_applied: usize,
    issue_count: usize,
    ok: bool,
    issues: Vec<LintIssue>,
}

/// 1 ファイルを lint する。
///
/// `json_sink` が `Some` のとき、JSON 形式のレポートは**その場で出力せず**
/// ここへ積む。複数ファイルを処理する場合に各ファイル分の JSON を逐次
/// print すると、オブジェクトが連結されて**単一の JSON 文書として不正**に
/// なるため（#750 のレビュー指摘）。呼び出し側が全件処理後に配列として
/// 一度だけ直列化する。
pub(crate) fn cmd_lint(
    input: &std::path::Path,
    fix: bool,
    format: LintOutputFormat,
    json_sink: Option<&mut Vec<LintReportOutput>>,
) -> Result<(), String> {
    let source = super::read_source(input)?;
    let mut file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;

    let mut fix_applied = 0usize;
    let mut lint_source = source.clone();
    if fix {
        fix_applied = apply_lint_fixes(&mut file);
        // 再 emit は tdsl-parser の正準フォーマッタを使う。LSP の Code Action
        // (`tdsl_core::lint::fix_source`) と同一の emitter を共有し、`lint --fix` と
        // エディタの quick fix が同じ出力になることを保証する。
        let rewritten = tdsl_parser::format_file(&file);
        if rewritten != source {
            std::fs::write(input, &rewritten)
                .map_err(|e| format!("Failed to write {}: {e}", input.display()))?;
            lint_source = rewritten;
        }
    }

    let issues = lint_issues(&file, &lint_source);
    // ERROR の件数は match の前に数える。JSON 分岐で `issues` が
    // `LintReportOutput` へ move されるため、後から参照できない。
    let error_count = issues
        .iter()
        .filter(|i| matches!(i.severity, LintSeverity::Error))
        .count();
    match format {
        LintOutputFormat::Text => {
            if fix {
                println!("Applied {fix_applied} fix(es) to {}", input.display());
            }
            if issues.is_empty() {
                println!("OK: no lint issues");
                return Ok(());
            }
            println!("Found {} issue(s):", issues.len());
            for issue in &issues {
                println!(
                    "- {severity} [{code}] line {line}: {message}{fixable}",
                    severity = match issue.severity {
                        LintSeverity::Error => "ERROR",
                        LintSeverity::Warning => "WARN",
                    },
                    code = issue.code,
                    line = issue.line,
                    message = issue.message,
                    fixable = if issue.fixable { " (fixable)" } else { "" }
                );
            }
        }
        LintOutputFormat::Json => {
            let report = LintReportOutput {
                file: input.display().to_string(),
                fix_applied,
                issue_count: issues.len(),
                ok: issues.is_empty(),
                issues,
            };
            match json_sink {
                Some(sink) => sink.push(report),
                None => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
                ),
            }
        }
    }

    // ERROR が残っていれば非ゼロ終了する。`main.rs` は `Err` のときだけ
    // `process::exit(1)` するため、ここで Ok を返すと CI で lint をゲートにできない
    // （`fmt --check` は未整形時に Err を返しており、そちらと非一貫だった。#766）。
    //
    // WARN のみの場合は従来どおり成功にする。警告で落とすかは `check` の
    // `--deny-warnings` 提案（#748）と揃えて別途決める。
    if error_count > 0 {
        return Err(format!(
            "{error_count} lint error(s) in {}",
            input.display()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_issues_detects_initial_rule_set() {
        let src = r#"
timeline "Lint" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span b 20..10 "" { tags ["x", "", "x"]; id "dup"; };
event a 30 "E" { id "dup"; };
event a 40 "No ID" {};
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        let codes: std::collections::HashSet<String> =
            issues.iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains("unknown_lane"));
        assert!(codes.contains("duplicate_id"));
        assert!(codes.contains("start_gt_end"));
        assert!(codes.contains("empty_label"));
        assert!(codes.contains("invalid_tags"));
        assert!(codes.contains("missing_id"));
    }

    #[test]
    fn apply_lint_fixes_normalizes_tags_swaps_ranges_and_generates_ids() {
        let src = r#"
timeline "Fix" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span a 20..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
event_range a 50..40 "R" { tags ["war", "war"]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 5);

        let rendered = tdsl_parser::format_file(&file);
        let reparsed = tdsl_parser::parse(&rendered).unwrap();
        let issues = lint_issues(&reparsed, &rendered);
        assert!(!issues.iter().any(|i| i.code == "start_gt_end"
            || i.code == "invalid_tags"
            || i.code == "missing_id"));

        let ir = tdsl_core::lower::lower_static(&reparsed).unwrap();
        assert_eq!(ir.items.len(), 3);
    }
}
