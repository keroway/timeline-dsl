//! DSL ソーステキストから LSP 診断（Diagnostic）を生成する純粋関数。
//!
//! ネットワーク不要・LSP サーバ非依存で単体テスト可能。
//! `Backend` の `did_open` / `did_change` からのみ呼ばれることを想定する。

use tdsl_parser::ast::{Span, Statement};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use crate::hover::byte_offset_to_utf16;

/// LSP の `Position` を生成する（0-based）。
///
/// tdsl の line/col は 1-based のため、ここで変換する。
/// `line` / `col` が 0 の場合はドキュメント先頭（0:0）を返す。
fn to_lsp_position(line_1based: u32, col_1based: u32) -> Position {
    // LSP は 0-based。tdsl は 1-based → -1 変換。
    // 0 はオフバイワンの安全ガード（不正な1-basedが来たときも panic しない）
    let line = line_1based.saturating_sub(1);
    let character = col_1based.saturating_sub(1);
    Position { line, character }
}

/// AST のバイトオフセット `Span` を LSP Range（0-based）に変換する。
fn span_to_range(span: &Span, source: &str) -> Range {
    let (start_line, start_col) = tdsl_parser::byte_offset_to_line_col(source, span.start);
    let (end_line, end_col) = tdsl_parser::byte_offset_to_line_col(source, span.end);
    Range {
        start: to_lsp_position(start_line, start_col),
        end: to_lsp_position(end_line, end_col),
    }
}

/// ドキュメント先頭を示す LSP Range（フォールバック用）。
fn document_start_range() -> Range {
    let pos = Position {
        line: 0,
        character: 0,
    };
    Range {
        start: pos,
        end: pos,
    }
}

/// `tdsl_core::lint::LintIssue` は行番号（1-based）しか持たないため、
/// 該当行全体を LSP Range（0-based）として返す。
///
/// 行が範囲外（想定外の入力）の場合は空文字列扱いにして character 0 の範囲にする
/// （panic させない安全側フォールバック）。
fn line_to_range(source: &str, line_1based: usize) -> Range {
    let line_idx = line_1based.saturating_sub(1) as u32;
    let line_str = source
        .split('\n')
        .nth(line_idx as usize)
        .unwrap_or_default();
    let end_character = byte_offset_to_utf16(line_str, line_str.len()) as u32;
    Range {
        start: Position {
            line: line_idx,
            character: 0,
        },
        end: Position {
            line: line_idx,
            character: end_character,
        },
    }
}

/// `tdsl_core::lint::lint_issues` のうち、`validate_with_spans` が既に同等の内容を
/// 報告しているコードを除外する。
///
/// - `unknown_lane` → `validate_with_spans` の `W201`
/// - `start_gt_end` / `mixed_offset_range` → 同 `W202` / `W208`
///
/// これらは lowering 後の IR ベースでより正確な span 付き診断が既に出ているため、
/// lint 側（行番号のみ）を重ねると二重報告になる。
fn is_duplicate_of_validate(code: &str) -> bool {
    matches!(code, "unknown_lane" | "start_gt_end" | "mixed_offset_range")
}

/// lint issue を LSP Diagnostic に変換する。
///
/// `fixable: true` の issue は Code Action で直せる旨も込めて `WARNING`、
/// `fixable: false`（書き手が判断する必要がある）の issue は発見性のために
/// `HINT` として区別する（issue #903 の提案）。
fn lint_issue_to_diagnostic(issue: &tdsl_core::lint::LintIssue, source: &str) -> Diagnostic {
    let severity = if issue.fixable {
        DiagnosticSeverity::WARNING
    } else {
        DiagnosticSeverity::HINT
    };
    Diagnostic {
        range: line_to_range(source, issue.line),
        severity: Some(severity),
        code: Some(tower_lsp::lsp_types::NumberOrString::String(
            issue.code.clone(),
        )),
        message: issue.message.clone(),
        source: Some("tdsl-lint".to_string()),
        ..Default::default()
    }
}

/// DSL ソーステキストをパース・検証し、LSP Diagnostic のリストを返す。
///
/// - パースエラーがあれば error 診断を 1 件返す（実位置付き）。
/// - パース成功なら静的 lowering → `validate_with_spans` で warning 診断を返す。
/// - Wikidata import 解決は行わない（offline 前提）。
pub fn compute_diagnostics(source: &str) -> Vec<Diagnostic> {
    match tdsl_parser::parse(source) {
        Err(parse_err) => {
            // パースエラー → error 診断 1 件
            let range = parse_err
                .source_location(source)
                .map(|loc| Range {
                    start: to_lsp_position(loc.line, loc.col),
                    end: to_lsp_position(loc.end_line, loc.end_col),
                })
                .unwrap_or_else(document_start_range);

            vec![Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: parse_err.to_string(),
                source: Some("tdsl".to_string()),
                ..Default::default()
            }]
        }
        Ok(file) => {
            // lowering — source_span 付与のため `with_source` 版を使う
            let mut diags: Vec<Diagnostic> =
                match tdsl_core::lower::lower_static_with_source(&file, Some(source)) {
                    Err(lowering_errs) => {
                        // lowering エラー → error 診断群（位置は document 先頭で妥当）
                        lowering_errs
                            .into_iter()
                            .map(|e| Diagnostic {
                                range: document_start_range(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: e.to_string(),
                                source: Some("tdsl".to_string()),
                                ..Default::default()
                            })
                            .collect()
                    }
                    Ok(ir) => {
                        // バリデーション警告 → warning 診断群
                        tdsl_core::validate::validate_with_spans(&ir)
                            .into_iter()
                            .map(|diag| {
                                let range = diag
                                    .span
                                    .as_ref()
                                    .map(|s| Range {
                                        start: to_lsp_position(s.line, s.col_start),
                                        end: to_lsp_position(s.line, s.col_end),
                                    })
                                    .unwrap_or_else(document_start_range);
                                Diagnostic {
                                    range,
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message: diag.message,
                                    source: Some("tdsl".to_string()),
                                    ..Default::default()
                                }
                            })
                            .collect()
                    }
                };

            // 静的に判定できる map/apply の参照エラー（未宣言 import alias / template、
            // `alias.key` 形式違反）を error 診断として報告する。エンティティ解決
            // （要ネットワーク）には依存しない。
            let ref_diags = tdsl_core::validate::validate_static_references(&file);
            let error_spans: std::collections::HashSet<(usize, usize)> = ref_diags
                .iter()
                .map(|d| (d.span.start, d.span.end))
                .collect();
            diags.extend(ref_diags.into_iter().map(|d| Diagnostic {
                range: span_to_range(&d.span, source),
                severity: Some(DiagnosticSeverity::ERROR),
                message: d.message,
                source: Some("tdsl".to_string()),
                ..Default::default()
            }));

            // 品質チェック（`tdsl lint` 相当）を diagnostics にも出す。従来は
            // Code Action（quick fix）経由でしか気付けず、`fixable: false` な issue
            // （例: `unused_lane`）は発見手段が無かった（#903）。
            // `validate_with_spans` と重複するコード（unknown_lane / start_gt_end /
            // mixed_offset_range）は除外し、二重報告を避ける。
            diags.extend(
                tdsl_core::lint::lint_issues(&file, source)
                    .iter()
                    .filter(|issue| !is_duplicate_of_validate(&issue.code))
                    .map(|issue| lint_issue_to_diagnostic(issue, source)),
            );

            // offline 診断は Wikidata fetch を行わないため、import/map/apply ブロックは
            // エンティティ解決されない（pass3/pass4 が走らない）。silent に握りつぶさず、
            // 各ブロック位置に「offline では未検証」である旨を Information 診断として明示する。
            // ただし静的参照エラーを既に出したブロックは、二重表示を避けて除外する。
            diags.extend(unresolved_block_notices(&file, source, &error_spans));
            diags
        }
    }
}

/// `import` / `map` / `apply` ブロックに対する「offline 未解決」通知を生成する。
///
/// これらのブロックは Wikidata の解決（ネットワーク）が前提のため、offline の LSP 診断では
/// アイテムが生成・検証されない。利用者がその差異に気付けるよう、各ブロック位置に
/// `Information` 診断を付与する（完全な検証は `tdsl build` / `tdsl check` を案内）。
///
/// `error_spans` に含まれるブロック（= 静的参照エラーを既に報告済み）は除外する。
fn unresolved_block_notices(
    file: &tdsl_parser::ast::File,
    source: &str,
    error_spans: &std::collections::HashSet<(usize, usize)>,
) -> Vec<Diagnostic> {
    file.statements
        .iter()
        .filter_map(|stmt| {
            let kind = match &stmt.node {
                Statement::Import(_) => "import",
                Statement::Map(_) => "map",
                Statement::Apply(_) => "apply",
                _ => return None,
            };
            if error_spans.contains(&(stmt.span.start, stmt.span.end)) {
                return None;
            }
            Some(Diagnostic {
                range: span_to_range(&stmt.span, source),
                severity: Some(DiagnosticSeverity::INFORMATION),
                message: format!(
                    "`{kind}` block is not resolved by offline LSP diagnostics (Wikidata fetch \
                     required); generated items are not shown or validated here. Run `tdsl build` \
                     / `tdsl check` for full validation."
                ),
                source: Some("tdsl".to_string()),
                ..Default::default()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 構文エラーのある DSL → error 診断が返り、期待 line/col を持つ。
    #[test]
    fn parse_error_produces_error_diagnostic() {
        // "timeline" ブロックに閉じ括弧がない
        let src = r#"timeline "test" { title "test";"#;
        let diags = compute_diagnostics(src);
        assert!(!diags.is_empty(), "エラーがある DSL は診断を返す");
        let first = &diags[0];
        assert_eq!(first.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(first.source.as_deref(), Some("tdsl"));
    }

    /// 正常な DSL → 診断 0 件。
    #[test]
    fn valid_dsl_produces_no_diagnostics() {
        // span の文法: span <lane_id> <start>..<end> "label" { ... }
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "lane1" as l1 { kind custom; order 10; }
span l1 100..200 "foo" { id "s1"; };
"#;
        let diags = compute_diagnostics(src);
        assert!(
            diags.is_empty(),
            "正常な DSL は診断 0 件になるべき。実際: {diags:#?}"
        );
    }

    /// import / map ブロックは offline では解決されないため、silent に消さず
    /// Information 診断として明示される。
    #[test]
    fn import_map_blocks_produce_information_notices() {
        let src = r#"
timeline "test" { title "test"; unit year; range -500..300; calendar proleptic_gregorian; }
lane "han" as han { kind dynasty; order 10; }
import wikidata as wd {
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}
map wd.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
}
"#;
        let diags = compute_diagnostics(src);
        let infos: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::INFORMATION))
            .collect();
        // import 1 件 + map 1 件 = 2 件
        assert_eq!(
            infos.len(),
            2,
            "import / map ブロックそれぞれに Information 診断が付くべき。実際: {diags:#?}"
        );
        assert!(
            infos.iter().all(|d| d.message.contains("offline")),
            "通知メッセージは offline の旨を含むべき"
        );
        // import ブロックは 4 行目（0-based: 3）に始まる
        assert!(
            infos.iter().any(|d| d.range.start.line >= 3),
            "通知はブロックの実位置を指すべき"
        );
        // 静的に検出できる問題（参照エラー等）が無ければ error/warning は出ない
        assert!(
            diags
                .iter()
                .all(|d| d.severity != Some(DiagnosticSeverity::ERROR)),
            "解決不能を error にはしない（offline の制約は Information で表現）"
        );
    }

    /// 未宣言の import alias を参照する map は、offline でも error 診断になる
    /// （静的に判定できる参照エラー）。当該ブロックには冗長な Information 通知を出さない。
    #[test]
    fn map_with_undeclared_import_alias_is_error() {
        let src = r#"
timeline "test" { title "test"; unit year; range -500..300; calendar proleptic_gregorian; }
lane "han" as han { kind dynasty; order 10; }
import wikidata as wd {
    entity Q7209 as han_dynasty;
}
map typo.han_dynasty to span {
    lane han;
    start claim(P571).year;
    end claim(P576).year;
    label label@ja ?? label@en;
}
"#;
        let diags = compute_diagnostics(src);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        assert_eq!(
            errors.len(),
            1,
            "未宣言 alias 参照は 1 件の error。実際: {diags:#?}"
        );
        assert!(
            errors[0].message.contains("typo"),
            "error メッセージに未宣言 alias 名を含むべき"
        );
        // map ブロック（error 済み）には Information 通知を重ねない。import ブロックには出る。
        let infos: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::INFORMATION))
            .collect();
        assert_eq!(
            infos.len(),
            1,
            "Information は import ブロックの 1 件のみ（error 済みの map は除外）。実際: {diags:#?}"
        );
    }

    /// 未宣言の template / import を参照する apply は error 診断になる。
    #[test]
    fn apply_with_undeclared_refs_is_error() {
        let src = r#"
timeline "test" { title "test"; unit year; range -500..300; calendar proleptic_gregorian; }
lane "d" as d { kind dynasty; order 10; }
apply missing_tmpl to missing_import {
    lane d;
}
"#;
        let diags = compute_diagnostics(src);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
            .collect();
        // 未宣言 import + 未宣言 template の 2 件
        assert_eq!(
            errors.len(),
            2,
            "apply の未宣言参照は 2 件の error。実際: {diags:#?}"
        );
    }

    /// start > end の span → warning 診断が返る。
    #[test]
    fn start_gt_end_produces_warning() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "lane1" as l1 { kind custom; order 10; }
span l1 500..100 "reversed" {};
"#;
        let diags = compute_diagnostics(src);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .collect();
        assert!(!warnings.is_empty(), "start>end は warning を返す");
        // 自動生成 ID か start 値が警告メッセージに含まれる
        assert!(
            warnings.iter().any(|d| d.message.contains("500")),
            "start 値を含む警告があるべき"
        );
    }

    /// `fixable: false` な lint issue（`unused_lane`）が diagnostics に HINT として現れる（#903）。
    /// 以前は Code Action からしか気付けず、diagnostics には一切出なかった。
    #[test]
    fn unused_lane_produces_hint_diagnostic() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "Used" as used { kind custom; order 1; }
lane "Orphan" as orphan { kind custom; order 2; }
span used 100..200 "foo" { id "s1"; };
"#;
        let diags = compute_diagnostics(src);
        let hint = diags
            .iter()
            .find(|d| d.severity == Some(DiagnosticSeverity::HINT))
            .unwrap_or_else(|| panic!("unused_lane の HINT 診断が無い: {diags:#?}"));
        assert!(
            hint.message.contains("orphan"),
            "HINT メッセージは未使用 lane 名を含むべき: {hint:#?}"
        );
        assert_eq!(hint.source.as_deref(), Some("tdsl-lint"));
    }

    /// `fixable: true` な lint issue（`missing_id`）は WARNING として現れる。
    #[test]
    fn missing_id_produces_warning_diagnostic() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "lane1" as l1 { kind custom; order 10; }
event l1 100 "foo" {};
"#;
        let diags = compute_diagnostics(src);
        let warning = diags
            .iter()
            .find(|d| {
                d.severity == Some(DiagnosticSeverity::WARNING)
                    && d.source.as_deref() == Some("tdsl-lint")
            })
            .unwrap_or_else(|| panic!("missing_id の WARNING 診断が無い: {diags:#?}"));
        assert!(warning.message.contains("id"));
    }

    /// lint の `unknown_lane` / `start_gt_end` は `validate_with_spans` 側の W201/W202 と
    /// 重複するため、二重報告しない（同じ問題に対して診断が 2 件出ない）。
    #[test]
    fn lint_duplicate_codes_are_not_double_reported() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "lane1" as l1 { kind custom; order 10; }
span l1 500..100 "reversed" { id "s1"; };
"#;
        let diags = compute_diagnostics(src);
        // W202 相当の警告は1件のみ（lint 側の重複 start_gt_end は出ない）
        let start_gt_end_like: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("500") && d.message.contains("100"))
            .collect();
        assert_eq!(
            start_gt_end_like.len(),
            1,
            "start>end の警告が重複している: {diags:#?}"
        );
    }

    /// 1-based → 0-based 変換のオフバイワンテスト。
    #[test]
    fn position_conversion_1based_to_0based() {
        // line=1, col=1 → Position { line: 0, character: 0 }
        let pos = to_lsp_position(1, 1);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        // line=3, col=5 → Position { line: 2, character: 4 }
        let pos = to_lsp_position(3, 5);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 4);
    }

    /// saturating_sub で 0 が来てもパニックしない境界テスト。
    #[test]
    fn position_conversion_zero_is_safe() {
        let pos = to_lsp_position(0, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    /// パースエラーの位置が 0-based に変換されていること。
    #[test]
    fn parse_error_position_is_0based() {
        // 2行目に不正な構文を置く
        let src = "// valid comment\n@@@ invalid token";
        let diags = compute_diagnostics(src);
        assert!(!diags.is_empty());
        let d = &diags[0];
        // パースエラーは 2行目付近（0-based なら line >= 1）
        assert!(
            d.range.start.line >= 1,
            "エラーは2行目(0-based:1)以降にあるべき。実際: {}",
            d.range.start.line
        );
    }
}
