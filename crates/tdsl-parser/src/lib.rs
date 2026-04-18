pub mod ast;
pub mod builder;
pub mod error;

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
                        start: -500,
                        end: 2000
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
                assert_eq!(s.start, -206);
                assert_eq!(s.end, 220);
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
                assert_eq!(e.time, -209);
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
                assert_eq!(er.start, 184);
                assert_eq!(er.end, 204);
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
}
