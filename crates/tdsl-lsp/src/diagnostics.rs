//! DSL ソーステキストから LSP 診断（Diagnostic）を生成する純粋関数。
//!
//! ネットワーク不要・LSP サーバ非依存で単体テスト可能。
//! `Backend` の `did_open` / `did_change` からのみ呼ばれることを想定する。

use tdsl_parser::ast::{Span, Statement};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

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

            // offline 診断は Wikidata fetch を行わないため、import/map/apply ブロックは
            // 解決されない（pass3/pass4 が走らない）。silent に握りつぶさず、各ブロック位置に
            // 「offline では未検証」である旨を Information 診断として明示する。
            diags.extend(unresolved_block_notices(&file, source));
            diags
        }
    }
}

/// `import` / `map` / `apply` ブロックに対する「offline 未解決」通知を生成する。
///
/// これらのブロックは Wikidata の解決（ネットワーク）が前提のため、offline の LSP 診断では
/// アイテムが生成・検証されない。利用者がその差異に気付けるよう、各ブロック位置に
/// `Information` 診断を付与する（完全な検証は `tdsl build` / `tdsl check` を案内）。
fn unresolved_block_notices(file: &tdsl_parser::ast::File, source: &str) -> Vec<Diagnostic> {
    file.statements
        .iter()
        .filter_map(|stmt| {
            let kind = match &stmt.node {
                Statement::Import(_) => "import",
                Statement::Map(_) => "map",
                Statement::Apply(_) => "apply",
                _ => return None,
            };
            Some(Diagnostic {
                range: span_to_range(&stmt.span, source),
                severity: Some(DiagnosticSeverity::INFORMATION),
                message: format!(
                    "`{kind}` ブロックは offline LSP 診断では解決されません（Wikidata fetch が必要）。\
                     生成されるアイテムは表示・検証されません。完全な検証は `tdsl build` / `tdsl check` を使用してください。"
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
span l1 100..200 "foo" {};
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
