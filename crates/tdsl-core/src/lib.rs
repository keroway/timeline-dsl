pub mod error;
pub mod ir;
pub mod lower;
pub mod validate;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_static_basic() {
        let src = r#"
            timeline "Test" { title "Test"; unit year; range 0..2000; }
            lane "A" as a { kind dynasty; order 1; }
            span a 100..200 "Span A" {};
            event a 150 "Event A" {};
            event_range a 120..180 "Range A" { tags ["war"]; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        assert_eq!(ir.meta.title, "Test");
        assert_eq!(ir.lanes.len(), 1);
        assert_eq!(ir.lanes[0].id, "a");
        assert_eq!(ir.items.len(), 3);
    }

    #[test]
    fn lower_detects_unknown_lane() {
        let src = r#"
            timeline "Test" { unit year; range 0..100; }
            span nonexistent 0..10 "Bad" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::UnknownLane(_))));
    }

    #[test]
    fn lower_detects_duplicate_lane() {
        let src = r#"
            timeline "Test" { unit year; range 0..100; }
            lane "A" as a { kind dynasty; }
            lane "B" as a { kind dynasty; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
    }

    #[test]
    fn lower_auto_generates_ids() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind dynasty; }
            span a 100..200 "S" {};
            event a 150 "E" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        match &ir.items[0] {
            ir::Item::Span { id, .. } => assert_eq!(id, "span:a:100"),
            _ => panic!("expected span"),
        }
        match &ir.items[1] {
            ir::Item::Event { id, .. } => assert_eq!(id, "event:a:150"),
            _ => panic!("expected event"),
        }
    }

    #[test]
    fn ir_json_roundtrip() {
        let src = r#"
            timeline "RT" { title "Roundtrip"; unit year; range -100..100; }
            lane "X" as x { kind custom; order 5; }
            span x -50..50 "Span" { tags ["test"]; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        let json = serde_json::to_string(&ir).unwrap();
        let ir2: ir::TimelineIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir2.meta.title, "Roundtrip");
        assert_eq!(ir2.lanes.len(), 1);
        assert_eq!(ir2.items.len(), 1);
    }

    #[test]
    fn validate_warns_on_bad_range() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Bad".into(),
                unit: "year".into(),
                range: (100, 0),
                calendar: "proleptic_gregorian".into(),
            },
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(!warnings.is_empty());
    }
}
