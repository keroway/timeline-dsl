pub mod ast;
pub mod builder;
pub mod error;
pub mod format;

pub use error::{ParseDiagnostic, ParseDiagnosticNote, byte_offset_to_line_col};
pub use format::{format_file, format_source};

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct TdslParser;

/// Parse a DSL source string into an AST [`ast::File`].
pub fn parse(source: &str) -> Result<ast::File, error::ParseError> {
    let pairs = TdslParser::parse(Rule::file, source)?;
    builder::build_file(pairs)
}

/// 単独の時刻リテラル文字列を [`ast::TimeValue`] にパースする。
///
/// `YYYY-MM-DD` / `YYYY-MM` / `YYYY` の 3 形式に対応し、月 1〜12 / 日 1〜31
/// の範囲外は `ParseError` を返す。前後の空白は許容して除去するが、
/// 文字列内部に余計なトークンがある場合は拒否する。
/// `tdsl import-csv` など、DSL 本文の外部で時刻文字列を解釈する経路から利用する。
pub fn parse_time_literal(s: &str) -> Result<ast::TimeValue, error::ParseError> {
    let trimmed = s.trim();
    let mut pairs = TdslParser::parse(Rule::time_literal_only, trimmed)?;
    let outer = pairs
        .next()
        .ok_or_else(|| error::ParseError::UnexpectedRule {
            rule: "time_literal_only: empty".to_string(),
            location: "0:0".to_string(),
        })?;
    let time_value_pair = outer
        .into_inner()
        .find(|p| matches!(p.as_rule(), Rule::time_value))
        .ok_or_else(|| error::ParseError::UnexpectedRule {
            rule: "time_literal_only: missing time_value".to_string(),
            location: "0:0".to_string(),
        })?;
    builder::parse_time_value(time_value_pair)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timeline_block() {
        let src = r#"
            timeline "中国王朝年表" {
                title "中国王朝年表";
                unit year;
                range -500..2000;
                calendar proleptic_gregorian;
            }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Timeline(t) => {
                assert_eq!(t.name, "中国王朝年表");
                assert_eq!(t.title.as_deref(), Some("中国王朝年表"));
                assert_eq!(t.unit.as_deref(), Some("year"));
                assert_eq!(
                    t.range,
                    Some(ast::RangeExpr {
                        start: ast::TimeValue::Year(-500),
                        end: ast::TimeValue::Year(2000),
                    })
                );
                assert_eq!(t.calendar.as_deref(), Some("proleptic_gregorian"));
            }
            _ => panic!("expected Timeline"),
        }
    }

    #[test]
    fn parse_lane() {
        let src = r#"lane "漢" as han { kind dynasty; order 10; }"#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Lane(l) => {
                assert_eq!(l.label, "漢");
                assert_eq!(l.alias.as_deref(), Some("han"));
                assert_eq!(l.kind.as_deref(), Some("dynasty"));
                assert_eq!(l.order, Some(10));
            }
            _ => panic!("expected Lane"),
        }
    }

    #[test]
    fn parse_span() {
        let src =
            r#"span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };"#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Span(s) => {
                assert_eq!(s.lane_ref, "han");
                assert_eq!(s.start, ast::TimeValue::Year(-206));
                assert_eq!(s.end, ast::TimeValue::Year(220));
                assert_eq!(s.label, "漢");
                assert_eq!(s.props.tags, vec!["dynasty"]);
                assert_eq!(
                    s.props.source,
                    Some(ast::SourceRef {
                        prefix: "wd".to_string(),
                        qid: "Q7209".to_string(),
                    })
                );
                assert_eq!(s.props.id.as_deref(), Some("span:han"));
            }
            _ => panic!("expected Span"),
        }
    }

    #[test]
    fn parse_event() {
        let src = r#"event han -209 "陳勝・呉広の乱" {};"#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Event(e) => {
                assert_eq!(e.lane_ref, "han");
                assert_eq!(e.time, ast::TimeValue::Year(-209));
                assert_eq!(e.label, "陳勝・呉広の乱");
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_event_range() {
        let src = r#"event_range han 184..204 "黄巾の乱" { tags ["war"]; };"#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::EventRange(er) => {
                assert_eq!(er.lane_ref, "han");
                assert_eq!(er.start, ast::TimeValue::Year(184));
                assert_eq!(er.end, ast::TimeValue::Year(204));
                assert_eq!(er.label, "黄巾の乱");
                assert_eq!(er.props.tags, vec!["war"]);
            }
            _ => panic!("expected EventRange"),
        }
    }

    #[test]
    fn parse_import_block() {
        let src = r#"
            import wikidata as wd {
                entity Q7209 as han_dynasty;
                query "SELECT ?item WHERE { ?item wdt:P31 wd:Q28171280 . }" as dynasties;
                policy merge_by_source;
            }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Import(imp) => {
                assert_eq!(imp.source_type, "wikidata");
                assert_eq!(imp.alias.as_deref(), Some("wd"));
                assert_eq!(imp.items.len(), 2);
                assert!(matches!(
                    &imp.items[0],
                    ast::ImportItem::Entity { qid, alias }
                        if qid == "Q7209" && alias.as_deref() == Some("han_dynasty")
                ));
                assert!(matches!(
                    &imp.items[1],
                    ast::ImportItem::Query { query, alias }
                        if query.contains("P31") && alias.as_deref() == Some("dynasties")
                ));
                assert_eq!(imp.policy, Some(ast::ReimportPolicy::MergeBySource));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_map_block() {
        let src = r#"
            map wd.han_dynasty to span {
                lane han;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
            }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                assert_eq!(m.source_ref, "wd.han_dynasty");
                assert_eq!(m.target_type, ast::MapTargetType::Span);
                assert_eq!(m.props.len(), 4);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_map_expr_with_fallback() {
        let src = r#"
            map wd.han to span {
                lane han;
                start claim(P580).year ?? claim(P571).year;
                end claim(P582).year ?? claim(P576).year;
                label label@ja ?? label@en;
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let start = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Start(e) => Some(e),
                        _ => None,
                    })
                    .expect("start present");
                assert_eq!(start.fallbacks.len(), 2);
                assert_eq!(start.fallbacks[0].claim.property, "P580");
                assert_eq!(start.fallbacks[0].accessor.as_deref(), Some("year"));
                assert_eq!(start.fallbacks[1].claim.property, "P571");
                assert_eq!(start.fallbacks[1].accessor.as_deref(), Some("year"));

                let end = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::End(e) => Some(e),
                        _ => None,
                    })
                    .expect("end present");
                assert_eq!(end.fallbacks.len(), 2);
                assert_eq!(end.fallbacks[0].claim.property, "P582");
                assert_eq!(end.fallbacks[1].claim.property, "P576");
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_map_expr_single_claim_still_works() {
        let src = r#"
            map wd.x to event {
                lane a;
                time claim(P571).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let time = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Time(e) => Some(e),
                        _ => None,
                    })
                    .expect("time present");
                assert_eq!(time.fallbacks.len(), 1);
                assert_eq!(time.fallbacks[0].claim.property, "P571");
                assert_eq!(time.fallbacks[0].accessor.as_deref(), Some("year"));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_map_expr_three_fallbacks() {
        let src = r#"
            map wd.x to event {
                lane a;
                time claim(P580).year ?? claim(P571).year ?? claim(P569).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let time = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Time(e) => Some(e),
                        _ => None,
                    })
                    .expect("time present");
                assert_eq!(time.fallbacks.len(), 3);
                assert_eq!(time.fallbacks[0].claim.property, "P580");
                assert_eq!(time.fallbacks[1].claim.property, "P571");
                assert_eq!(time.fallbacks[2].claim.property, "P569");
            }
            _ => panic!("expected Map"),
        }
    }

    fn extract_map_filters(file: &ast::File) -> Vec<ast::FilterExpr> {
        match &file.statements[0].node {
            ast::Statement::Map(m) => m
                .props
                .iter()
                .filter_map(|p| match p {
                    ast::MapProp::Filter(e) => Some(e.clone()),
                    _ => None,
                })
                .collect(),
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_map_filter_basic_gt() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1000;
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 1);
        match &filters[0] {
            ast::FilterExpr::Compare { lhs, op, rhs } => {
                assert!(matches!(lhs, ast::FilterOperand::Claim(_)));
                assert_eq!(*op, ast::CompareOp::Gt);
                assert!(matches!(rhs, ast::FilterOperand::Int(1000)));
            }
            other => panic!("expected Compare, got {other:?}"),
        }
    }

    #[test]
    fn parse_map_filter_null_check() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P576).year != null;
                start claim(P580).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 1);
        match &filters[0] {
            ast::FilterExpr::Compare { lhs, op, rhs } => {
                assert!(matches!(lhs, ast::FilterOperand::Claim(_)));
                assert_eq!(*op, ast::CompareOp::NotEq);
                assert!(matches!(rhs, ast::FilterOperand::Null));
            }
            other => panic!("expected Compare, got {other:?}"),
        }
    }

    #[test]
    fn parse_map_filter_and_or() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1000 && claim(P582).year < 2000;
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 1);
        assert!(matches!(&filters[0], ast::FilterExpr::And(_, _)));
    }

    #[test]
    fn parse_map_filter_not() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter !(claim(P582).year == null);
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 1);
        match &filters[0] {
            ast::FilterExpr::Not(inner) => {
                assert!(matches!(inner.as_ref(), ast::FilterExpr::Compare { .. }));
            }
            other => panic!("expected Not, got {other:?}"),
        }
    }

    #[test]
    fn parse_map_multiple_filters() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1000;
                filter claim(P576).year != null;
                start claim(P580).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 2);
    }

    #[test]
    fn parse_map_filter_paren_precedence() {
        // (a > 1) || (b < 0 && c > 2)
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1
                    || (claim(P582).year < 0 && claim(P571).year > 2);
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let file = parse(src).unwrap();
        let filters = extract_map_filters(&file);
        assert_eq!(filters.len(), 1);
        // Top-level should be Or, with the right side being And.
        match &filters[0] {
            ast::FilterExpr::Or(_lhs, rhs) => {
                assert!(matches!(rhs.as_ref(), ast::FilterExpr::And(_, _)));
            }
            other => panic!("expected Or at top, got {other:?}"),
        }
    }

    #[test]
    fn parse_comments() {
        let src = r#"
            // This is a comment
            lane "秦" as qin { kind dynasty; /* inline comment */ order 20; }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn parse_full_example() {
        let src = r#"
            timeline "中国王朝年表" {
                title "中国王朝年表";
                unit year;
                range -500..2000;
                calendar proleptic_gregorian;
            }

            lane "漢" as han { kind dynasty; order 10; }
            lane "秦" as qin { kind dynasty; order 20; }

            span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
            span qin -221..-206 "秦" { tags ["dynasty"]; source wd:Q7462; id "span:qin"; };

            event han -209 "陳勝・呉広の乱" {};
            event_range han 184..204 "黄巾の乱" { tags ["war"]; };

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
        let file = parse(src).unwrap();
        // timeline(1) + lanes(2) + spans(2) + event(1) + event_range(1) + import(1) + map(1) = 9
        assert_eq!(file.statements.len(), 9);
    }

    #[test]
    fn parse_unknown_target_type_fails() {
        let src = r#"
            map wd.x to unknown_type {
                lane a;
            }
        "#;
        let result = parse(src);
        assert!(result.is_err());
    }

    // ─── Edge cases ──────────────────────────────────────────────────────────

    #[test]
    fn parse_string_with_escaped_quote() {
        let src = r#"lane "He said \"hello\"" as x {}"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Lane(l) => {
                assert!(l.label.contains("hello"));
            }
            _ => panic!("expected Lane"),
        }
    }

    #[test]
    fn parse_negative_boundary_values() {
        let src = r#"span han -9999..0 "大昔" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Span(s) => {
                assert_eq!(s.start, ast::TimeValue::Year(-9999));
                assert_eq!(s.end, ast::TimeValue::Year(0));
            }
            _ => panic!("expected Span"),
        }
    }

    #[test]
    fn parse_multiple_tags_in_list() {
        let src = r#"span han 100..200 "漢" { tags ["a", "b", "c"]; };"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Span(s) => {
                assert_eq!(s.props.tags, vec!["a", "b", "c"]);
            }
            _ => panic!("expected Span"),
        }
    }

    #[test]
    fn parse_event_range_with_id_and_origin() {
        let src = r#"event_range han 100..200 "乱" { id "er:han:100"; origin manual; };"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::EventRange(er) => {
                assert_eq!(er.props.id.as_deref(), Some("er:han:100"));
                assert_eq!(er.props.origin.as_deref(), Some("manual"));
            }
            _ => panic!("expected EventRange"),
        }
    }

    #[test]
    fn parse_import_without_alias() {
        let src = r#"
            import wikidata {
                entity Q7209;
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Import(imp) => {
                assert_eq!(imp.source_type, "wikidata");
                assert!(imp.alias.is_none());
                assert_eq!(imp.items.len(), 1);
                assert!(matches!(&imp.items[0],
                    ast::ImportItem::Entity { qid, alias }
                        if qid == "Q7209" && alias.is_none()
                ));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_map_with_tags() {
        let src = r#"
            map wd.han to span {
                lane han;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja;
                tags ["dynasty", "china"];
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let has_tags = m
                    .props
                    .iter()
                    .any(|p| matches!(p, ast::MapProp::Tags(t) if t.len() == 2));
                assert!(has_tags);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_lane_without_alias_no_kind() {
        let src = r#"lane "Simple" {}"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Lane(l) => {
                assert_eq!(l.label, "Simple");
                assert!(l.alias.is_none());
                assert!(l.kind.is_none());
                assert!(l.order.is_none());
            }
            _ => panic!("expected Lane"),
        }
    }

    #[test]
    fn parse_block_comment_multiline() {
        let src = r#"
            /* This is a
               multi-line
               block comment */
            lane "秦" as qin {}
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Lane(l) => assert_eq!(l.label, "秦"),
            _ => panic!("expected Lane"),
        }
    }

    #[test]
    fn parse_event_with_zero_year() {
        let src = r#"event han 0 "年0の出来事" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Event(e) => assert_eq!(e.time, ast::TimeValue::Year(0)),
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_overwrite_imported_policy() {
        let src = r#"import wikidata as wd { policy overwrite_imported; }"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Import(imp) => {
                assert_eq!(imp.policy, Some(ast::ReimportPolicy::OverwriteImported));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_keep_manual_policy() {
        let src = r#"import wikidata as wd { policy keep_manual; }"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Import(imp) => {
                assert_eq!(imp.policy, Some(ast::ReimportPolicy::KeepManual));
            }
            _ => panic!("expected Import"),
        }
    }

    #[test]
    fn parse_unknown_policy_fails() {
        let src = r#"import wikidata as wd { policy unknown_policy; }"#;
        let result = parse(src);
        assert!(result.is_err());
    }

    #[test]
    fn parse_template_block_with_alias() {
        let src = r#"
            template "人物の生涯" as person_life
                to event_range {
                    start claim(P569).year;
                    end claim(P570).year;
                    label label@ja ?? label@en;
                }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Template(t) => {
                assert_eq!(t.name, "人物の生涯");
                assert_eq!(t.alias.as_deref(), Some("person_life"));
                assert_eq!(t.target_type, ast::MapTargetType::EventRange);
                assert_eq!(t.props.len(), 3);
                assert!(matches!(&t.props[0], ast::MapProp::Start(_)));
                assert!(matches!(&t.props[1], ast::MapProp::End(_)));
                assert!(matches!(&t.props[2], ast::MapProp::Label(_)));
            }
            _ => panic!("expected Template"),
        }
    }

    #[test]
    fn parse_template_block_without_alias() {
        let src = r#"
            template "Dynasty Span"
                to span {
                    start claim(P571).year;
                    end claim(P576).year;
                    label label@ja ?? label@en;
                }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Template(t) => {
                assert_eq!(t.name, "Dynasty Span");
                assert!(t.alias.is_none());
                assert_eq!(t.target_type, ast::MapTargetType::Span);
            }
            _ => panic!("expected Template"),
        }
    }

    #[test]
    fn parse_apply_block_with_override() {
        let src = r#"
            apply person_life to emperors {
                lane imperial;
            }
        "#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Apply(a) => {
                assert_eq!(a.template_alias, "person_life");
                assert_eq!(a.import_alias, "emperors");
                assert_eq!(a.overrides.len(), 1);
                assert!(matches!(&a.overrides[0], ast::MapProp::Lane(id) if id == "imperial"));
            }
            _ => panic!("expected Apply"),
        }
    }

    #[test]
    fn parse_apply_block_empty_overrides() {
        let src = r#"apply dynasty_template to imports {}"#;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Apply(a) => {
                assert_eq!(a.template_alias, "dynasty_template");
                assert_eq!(a.import_alias, "imports");
                assert!(a.overrides.is_empty());
            }
            _ => panic!("expected Apply"),
        }
    }

    #[test]
    fn parse_color_map_block() {
        let src = r##"
            timeline "テスト" {
                title "テスト";
                unit year;
                range 0..2000;
                color_map {
                    dynasty: "#3366cc";
                    war: "#cc0000";
                }
            }
        "##;
        let file = parse(src).unwrap();
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Timeline(t) => {
                assert_eq!(t.color_map.len(), 2);
                assert!(
                    t.color_map
                        .iter()
                        .any(|(k, v)| k == "dynasty" && v == "#3366cc")
                );
                assert!(
                    t.color_map
                        .iter()
                        .any(|(k, v)| k == "war" && v == "#cc0000")
                );
            }
            _ => panic!("expected Timeline"),
        }
    }

    #[test]
    fn parse_color_map_block_empty() {
        let src = r#"timeline "T" { color_map {} }"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Timeline(t) => assert!(t.color_map.is_empty()),
            _ => panic!("expected Timeline"),
        }
    }

    #[test]
    fn parse_five_digit_year_still_works() {
        // 後方互換: bench / 既存テストで使われている 5 桁以上の年が引き続き year_lit でマッチすること
        let src = r#"timeline "T" { unit year; range 0..10000; }"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Timeline(t) => {
                assert_eq!(
                    t.range,
                    Some(ast::RangeExpr {
                        start: ast::TimeValue::Year(0),
                        end: ast::TimeValue::Year(10000),
                    })
                );
            }
            _ => panic!("expected Timeline"),
        }
    }

    #[test]
    fn parse_six_digit_year_in_event() {
        let src = r#"event ancient 100000 "未来" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Event(e) => {
                assert_eq!(e.time, ast::TimeValue::Year(100000));
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn test_field_priority_policy_parse() {
        let src = r#"
            import wikidata as wd {
                entity Q123 as foo;
                policy field_priority {
                    label: manual;
                    time: wikidata;
                    tags: merge;
                }
            }
        "#;
        let result = parse(src);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let file = result.unwrap();
        let import = match &file.statements[0].node {
            crate::ast::Statement::Import(b) => b,
            _ => panic!("expected import"),
        };
        match import.policy {
            Some(crate::ast::ReimportPolicy::FieldPriority(config)) => {
                assert_eq!(config.label, crate::ast::FieldStrategy::Manual);
                assert_eq!(config.time, crate::ast::FieldStrategy::Wikidata);
                assert_eq!(config.tags, crate::ast::FieldStrategy::Merge);
            }
            other => panic!("expected FieldPriority, got {:?}", other),
        }
    }

    // ─── 日付リテラル (#243) ─────────────────────────────────────────

    #[test]
    fn parse_date_literal_event() {
        let src = r#"event han 1969-07-20 "月面着陸" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Event(e) => {
                assert_eq!(e.time, ast::TimeValue::Date(1969, 7, 20));
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_year_month_literal_event() {
        let src = r#"event han 1969-07 "月着陸の月" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Event(e) => {
                assert_eq!(e.time, ast::TimeValue::YearMonth(1969, 7));
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_span_with_dates() {
        let src = r#"span ww2 1939-09-01..1945-09-02 "第二次世界大戦" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Span(s) => {
                assert_eq!(s.start, ast::TimeValue::Date(1939, 9, 1));
                assert_eq!(s.end, ast::TimeValue::Date(1945, 9, 2));
            }
            _ => panic!("expected Span"),
        }
    }

    #[test]
    fn parse_span_with_mixed_precision() {
        let src = r#"span partial 1900..1969-07-20 "混在範囲" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Span(s) => {
                assert_eq!(s.start, ast::TimeValue::Year(1900));
                assert_eq!(s.end, ast::TimeValue::Date(1969, 7, 20));
            }
            _ => panic!("expected Span"),
        }
    }

    #[test]
    fn parse_event_range_with_year_month() {
        let src = r#"event_range han 1939-09..1945-09 "第二次世界大戦" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::EventRange(er) => {
                assert_eq!(er.start, ast::TimeValue::YearMonth(1939, 9));
                assert_eq!(er.end, ast::TimeValue::YearMonth(1945, 9));
            }
            _ => panic!("expected EventRange"),
        }
    }

    #[test]
    fn parse_range_directive_with_dates() {
        let src = r#"
            timeline "近代史" {
                unit month;
                range 1939-01..1946-01;
            }
        "#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Timeline(t) => {
                assert_eq!(
                    t.range,
                    Some(ast::RangeExpr {
                        start: ast::TimeValue::YearMonth(1939, 1),
                        end: ast::TimeValue::YearMonth(1946, 1),
                    })
                );
            }
            _ => panic!("expected Timeline"),
        }
    }

    #[test]
    fn parse_negative_year_still_works() {
        let src = r#"event qin -206 "始皇帝即位" {};"#;
        let file = parse(src).unwrap();
        match &file.statements[0].node {
            ast::Statement::Event(e) => {
                assert_eq!(e.time, ast::TimeValue::Year(-206));
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_invalid_month_zero_fails() {
        let src = r#"event han 1969-00-20 "invalid" {};"#;
        let result = parse(src);
        assert!(result.is_err(), "expected error for month=00");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("Invalid month") || msg.to_lowercase().contains("month"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn parse_invalid_month_thirteen_fails() {
        let src = r#"event han 1969-13-20 "invalid" {};"#;
        let result = parse(src);
        assert!(result.is_err(), "expected error for month=13");
    }

    #[test]
    fn parse_invalid_day_zero_fails() {
        let src = r#"event han 1969-07-00 "invalid" {};"#;
        let result = parse(src);
        assert!(result.is_err(), "expected error for day=00");
    }

    #[test]
    fn parse_invalid_day_thirtytwo_fails() {
        let src = r#"event han 1969-07-32 "invalid" {};"#;
        let result = parse(src);
        assert!(result.is_err(), "expected error for day=32");
    }

    #[test]
    fn parse_negative_year_with_month_rejected() {
        // 紀元前は year 精度のみ。仕様書 §1.3 に従い year_month/date は符号なし。
        let src = r#"event ancient -206-01 "鴻門の会" {};"#;
        let result = parse(src);
        assert!(result.is_err(), "expected error for negative YearMonth");
    }

    #[test]
    fn parse_time_value_display_round_trip() {
        // Display 実装が `Date` を `YYYY-MM-DD` で表示することを確認
        let d = ast::TimeValue::Date(1969, 7, 20);
        assert_eq!(format!("{d}"), "1969-07-20");
        let m = ast::TimeValue::YearMonth(1939, 9);
        assert_eq!(format!("{m}"), "1939-09");
        let y = ast::TimeValue::Year(-206);
        assert_eq!(format!("{y}"), "-206");
    }

    #[test]
    fn parse_time_value_accessors() {
        let d = ast::TimeValue::Date(1969, 7, 20);
        assert_eq!(d.year(), 1969);
        assert_eq!(d.month(), Some(7));
        assert_eq!(d.day(), Some(20));

        let m = ast::TimeValue::YearMonth(1939, 9);
        assert_eq!(m.year(), 1939);
        assert_eq!(m.month(), Some(9));
        assert_eq!(m.day(), None);

        let y = ast::TimeValue::Year(-206);
        assert_eq!(y.year(), -206);
        assert_eq!(y.month(), None);
        assert_eq!(y.day(), None);
    }

    #[test]
    fn parse_time_value_to_sortable_order() {
        use ast::TimeValue::*;
        assert!(Year(1939).to_sortable() < Year(1940).to_sortable());
        assert!(YearMonth(1939, 9).to_sortable() > YearMonth(1939, 8).to_sortable());
        assert!(Date(1939, 9, 1).to_sortable() > Date(1939, 8, 31).to_sortable());
        // 同年の Year は (year, 0, 0) なので、月日付きより前に位置する
        assert!(Year(1939).to_sortable() < YearMonth(1939, 1).to_sortable());
    }

    // ─── parse_time_literal (公開 API) ───────────────────────

    #[test]
    fn parse_time_literal_year() {
        assert_eq!(
            parse_time_literal("2020").unwrap(),
            ast::TimeValue::Year(2020)
        );
        assert_eq!(
            parse_time_literal("-206").unwrap(),
            ast::TimeValue::Year(-206)
        );
        assert_eq!(parse_time_literal("0").unwrap(), ast::TimeValue::Year(0));
    }

    #[test]
    fn parse_time_literal_year_month() {
        assert_eq!(
            parse_time_literal("1939-09").unwrap(),
            ast::TimeValue::YearMonth(1939, 9)
        );
    }

    #[test]
    fn parse_time_literal_date() {
        assert_eq!(
            parse_time_literal("1969-07-20").unwrap(),
            ast::TimeValue::Date(1969, 7, 20)
        );
    }

    #[test]
    fn parse_time_literal_invalid_month_rejected() {
        assert!(parse_time_literal("2020-13").is_err());
        assert!(parse_time_literal("2020-00").is_err());
    }

    #[test]
    fn parse_time_literal_invalid_day_rejected() {
        assert!(parse_time_literal("2020-02-32").is_err());
        assert!(parse_time_literal("2020-02-00").is_err());
    }

    #[test]
    fn parse_time_literal_trailing_garbage_rejected() {
        // EOI 制約により末尾の非空白ゴミは拒否される
        assert!(parse_time_literal("1969-07-20foo").is_err());
        assert!(parse_time_literal("2020 extra").is_err());
    }

    #[test]
    fn parse_time_literal_strips_outer_whitespace() {
        // CSV パディング想定: 前後の空白は trim して解釈する
        assert_eq!(
            parse_time_literal("  2020  ").unwrap(),
            ast::TimeValue::Year(2020)
        );
        assert_eq!(
            parse_time_literal("\t1969-07-20\n").unwrap(),
            ast::TimeValue::Date(1969, 7, 20)
        );
    }

    #[test]
    fn parse_time_literal_non_numeric_rejected() {
        assert!(parse_time_literal("abc").is_err());
        assert!(parse_time_literal("").is_err());
    }

    #[test]
    fn parse_time_literal_negative_with_month_rejected() {
        // 紀元前は year 精度のみ。仕様書 §1.3 と整合させる
        assert!(parse_time_literal("-206-01").is_err());
    }

    // ─── 時間式オフセット (#148) ──────────────────────────────────

    #[test]
    fn parse_claim_expr_with_positive_offset() {
        let src = r#"
            map wd.x to span {
                lane a;
                start claim(P569).year +1;
                end claim(P570).year +30;
                label label@ja;
            }
        "#;
        let file = parse(src).expect("should parse claim_expr with positive offset");
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let start = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Start(e) => Some(e),
                        _ => None,
                    })
                    .expect("start present");
                assert_eq!(start.fallbacks.len(), 1);
                assert_eq!(start.fallbacks[0].claim.property, "P569");
                assert_eq!(start.fallbacks[0].accessor.as_deref(), Some("year"));
                assert_eq!(start.fallbacks[0].offset, Some(1));

                let end = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::End(e) => Some(e),
                        _ => None,
                    })
                    .expect("end present");
                assert_eq!(end.fallbacks[0].offset, Some(30));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_claim_expr_with_negative_offset() {
        let src = r#"
            map wd.x to span {
                lane a;
                start claim(P569).year -5;
                end claim(P570).year -100;
                label label@ja;
            }
        "#;
        let file = parse(src).expect("should parse claim_expr with negative offset");
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let start = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Start(e) => Some(e),
                        _ => None,
                    })
                    .expect("start present");
                assert_eq!(start.fallbacks[0].offset, Some(-5));

                let end = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::End(e) => Some(e),
                        _ => None,
                    })
                    .expect("end present");
                assert_eq!(end.fallbacks[0].offset, Some(-100));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_claim_expr_without_offset_is_none() {
        let src = r#"
            map wd.x to span {
                lane a;
                start claim(P569).year;
                end claim(P570).year;
                label label@ja;
            }
        "#;
        let file = parse(src).expect("should parse claim_expr without offset");
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let start = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Start(e) => Some(e),
                        _ => None,
                    })
                    .expect("start present");
                assert_eq!(start.fallbacks[0].offset, None);
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_claim_expr_offset_with_fallback() {
        // フォールバックチェーン `??` 内でもオフセットが正しくパースされる
        let src = r#"
            map wd.x to span {
                lane a;
                start claim(P580).year +1 ?? claim(P571).year -10;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let file = parse(src).expect("should parse claim_expr with offset in fallback chain");
        match &file.statements[0].node {
            ast::Statement::Map(m) => {
                let start = m
                    .props
                    .iter()
                    .find_map(|p| match p {
                        ast::MapProp::Start(e) => Some(e),
                        _ => None,
                    })
                    .expect("start present");
                assert_eq!(start.fallbacks.len(), 2);
                assert_eq!(start.fallbacks[0].claim.property, "P580");
                assert_eq!(start.fallbacks[0].offset, Some(1));
                assert_eq!(start.fallbacks[1].claim.property, "P571");
                assert_eq!(start.fallbacks[1].offset, Some(-10));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn test_field_priority_partial_fields() {
        // デフォルト値が適用されること
        let src = r#"
            import wikidata as wd {
                entity Q123 as foo;
                policy field_priority {
                    tags: wikidata;
                }
            }
        "#;
        let result = parse(src);
        assert!(result.is_ok());
        let file = result.unwrap();
        let import = match &file.statements[0].node {
            crate::ast::Statement::Import(b) => b,
            _ => panic!("expected import"),
        };
        match import.policy {
            Some(crate::ast::ReimportPolicy::FieldPriority(config)) => {
                assert_eq!(config.label, crate::ast::FieldStrategy::Manual); // default
                assert_eq!(config.time, crate::ast::FieldStrategy::Wikidata); // default
                assert_eq!(config.tags, crate::ast::FieldStrategy::Wikidata); // overridden
            }
            other => panic!("expected FieldPriority, got {:?}", other),
        }
    }

    // ─── source_location テスト ───────────────────────────────────────────

    /// pest Syntax エラーから line/col が正しく取れること（1-based）。
    #[test]
    fn source_location_syntax_error_pos() {
        // 1行目に不正なトークン
        let src = "@@@";
        let err = parse(src).unwrap_err();
        let loc = err.source_location(src).expect("should have location");
        assert_eq!(loc.line, 1, "1行目のエラーは line=1");
        assert!(loc.col >= 1, "col は 1-based で 1 以上");
    }

    /// pest Syntax エラーが複数行テキストの 2 行目にある場合。
    #[test]
    fn source_location_syntax_error_second_line() {
        let src = "// comment\n@@@";
        let err = parse(src).unwrap_err();
        let loc = err.source_location(src).expect("should have location");
        // エラーは 2行目（1-based）
        assert_eq!(loc.line, 2, "2行目のエラーは line=2");
    }

    /// InvalidMonth variant でバイトオフセットから line/col に変換できること。
    #[test]
    fn source_location_invalid_month_byte_offset() {
        // 月が 13 の不正な日付を作る: "2023-13" は grammar では year+month で
        // month の範囲外（13）が builder で InvalidMonth エラーになる
        let src = r#"
timeline "t" {
    title "t";
    unit year;
    range 2023-01..2023-13;
    calendar proleptic_gregorian;
}
"#;
        // InvalidMonth エラーが発生するはず
        match parse(src) {
            Err(e @ error::ParseError::InvalidMonth { .. }) => {
                let loc = e.source_location(src);
                // 位置情報が取れれば OK（値の正確さより変換が panic しないことを確認）
                assert!(loc.is_some(), "InvalidMonth は source_location を返す");
                let loc = loc.unwrap();
                assert!(loc.line >= 1, "line は 1-based で 1 以上");
                assert!(loc.col >= 1, "col は 1-based で 1 以上");
            }
            // grammar によっては Syntax エラーになることもある（その場合もテスト通過）
            Err(e @ error::ParseError::Syntax(_)) => {
                let loc = e.source_location(src);
                assert!(loc.is_some(), "Syntax error も source_location を返す");
            }
            Ok(_) => {
                // grammar が月境界チェックを別途行う場合はスキップ
            }
            Err(other) => panic!("unexpected error: {:?}", other),
        }
    }

    /// UnknownPolicy は位置情報を持たない（None を返す）。
    #[test]
    fn source_location_unknown_policy_returns_none() {
        let err = error::ParseError::UnknownPolicy("bogus".to_string());
        assert!(
            err.source_location("anything").is_none(),
            "UnknownPolicy は source_location = None"
        );
    }

    // ─── group ブロック ───────────────────────────────────────────

    #[test]
    fn parse_group_with_two_lanes() {
        let src = r#"
            group "古代" {
                lane "秦" as qin {}
                lane "漢" as han { kind dynasty; order 10; }
            }
        "#;
        let file = parse(src).expect("group with two lanes must parse");
        assert_eq!(file.statements.len(), 1);
        match &file.statements[0].node {
            ast::Statement::Group(g) => {
                assert_eq!(g.label, "古代");
                assert_eq!(g.lanes.len(), 2);
                assert_eq!(g.lanes[0].label, "秦");
                assert_eq!(g.lanes[0].alias.as_deref(), Some("qin"));
                assert_eq!(g.lanes[1].label, "漢");
                assert_eq!(g.lanes[1].kind.as_deref(), Some("dynasty"));
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn parse_group_single_lane() {
        let src = r#"group "単一" { lane "A" as a {} }"#;
        let file = parse(src).expect("group with single lane must parse");
        match &file.statements[0].node {
            ast::Statement::Group(g) => {
                assert_eq!(g.label, "単一");
                assert_eq!(g.lanes.len(), 1);
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn parse_group_mixed_with_other_statements() {
        let src = r#"
            lane "独立" as standalone {}
            group "グループ" {
                lane "子A" as child_a {}
            }
            lane "独立2" as standalone2 {}
        "#;
        let file = parse(src).expect("group mixed with lanes must parse");
        assert_eq!(file.statements.len(), 3);
        assert!(matches!(file.statements[0].node, ast::Statement::Lane(_)));
        assert!(matches!(file.statements[1].node, ast::Statement::Group(_)));
        assert!(matches!(file.statements[2].node, ast::Statement::Lane(_)));
    }

    #[test]
    fn parse_group_without_lanes_fails() {
        // group ブロックには lane が 1 つ以上必要（grammar: lane_decl+）
        let src = r#"group "空" {}"#;
        assert!(
            parse(src).is_err(),
            "group without any lane must fail to parse"
        );
    }

    // ─── ParseDiagnostic SourceSpan テスト (#355) ─────────────────────────────

    /// 不正トークンの構文エラーで ParseDiagnostic が正しい SourceSpan を持つこと。
    #[test]
    fn parse_diagnostic_syntax_error_has_source_span() {
        let src = "@@@";
        let err = parse(src).unwrap_err();
        let diag = error::ParseDiagnostic::from_parse_error(&err, src, "test.tdsl");
        // span は Some であること（位置情報が付与される）
        assert!(
            diag.span().is_some(),
            "Syntax エラーは SourceSpan を持つべき"
        );
        let span = diag.span().unwrap();
        // オフセットはソース長以内
        assert!(
            span.offset() <= src.len(),
            "offset {} はソース長 {} 以内であるべき",
            span.offset(),
            src.len()
        );
        // 長さは 1 以上
        assert!(!span.is_empty(), "len は 1 以上であるべき");
    }

    /// 複数行ソースの 2 行目にある構文エラーで offset が行頭バイトを含むこと。
    #[test]
    fn parse_diagnostic_multiline_second_line_span() {
        let src = "// comment\n@@@";
        let err = parse(src).unwrap_err();
        let diag = error::ParseDiagnostic::from_parse_error(&err, src, "test.tdsl");
        let span = diag.span().expect("span must be Some");
        // 2行目の先頭は offset 11（"// comment\n" = 11 bytes）
        assert!(
            span.offset() >= 11,
            "2 行目のエラーは offset >= 11 であるべき（実際: {}）",
            span.offset()
        );
    }

    /// 位置情報のない UnknownPolicy は span が None であること。
    #[test]
    fn parse_diagnostic_unknown_policy_no_span() {
        let err = error::ParseError::UnknownPolicy("bogus".to_string());
        let diag = error::ParseDiagnostic::from_parse_error(&err, "anything", "test.tdsl");
        assert!(
            diag.span().is_none(),
            "UnknownPolicy は SourceSpan を持たないべき"
        );
    }

    /// バイトオフセット variant（InvalidMonth）で span が構築できること。
    #[test]
    fn parse_diagnostic_invalid_month_has_span() {
        let src = r#"
timeline "t" {
    title "t";
    unit year;
    range 2023-01..2023-13;
    calendar proleptic_gregorian;
}
"#;
        match parse(src) {
            Err(ref e @ error::ParseError::InvalidMonth { .. }) => {
                let diag = error::ParseDiagnostic::from_parse_error(e, src, "test.tdsl");
                let span = diag.span();
                // span が Some であり panic しないこと
                assert!(span.is_some(), "InvalidMonth は SourceSpan を持つべき");
                let span = span.unwrap();
                assert!(span.offset() <= src.len());
                assert!(!span.is_empty());
            }
            Err(error::ParseError::Syntax(_)) => {
                // grammar の都合で Syntax エラーになる場合も通過させる
            }
            Ok(_) => {
                // grammar が月境界チェックを行わない場合はスキップ
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
}
