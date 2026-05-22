pub mod decompile;
pub mod error;
pub mod ir;
pub mod lower;
pub mod merge;
pub mod validate;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "wikidata")]
    use std::collections::HashMap;

    #[cfg(feature = "wikidata")]
    use async_trait::async_trait;
    #[cfg(feature = "wikidata")]
    use tdsl_wikidata::entity::{DataValue, LabelValue, Snak, Statement, TimeValue};
    #[cfg(feature = "wikidata")]
    use tdsl_wikidata::{SearchResult, WikidataClient, WikidataEntity, WikidataError};

    #[cfg(feature = "wikidata")]
    struct MockWikidataClient {
        entities: HashMap<String, WikidataEntity>,
        query_results: Vec<String>,
    }

    #[cfg(feature = "wikidata")]
    #[async_trait]
    impl WikidataClient for MockWikidataClient {
        async fn get_entity(
            &self,
            qid: &str,
            _langs: &[&str],
        ) -> Result<WikidataEntity, WikidataError> {
            self.entities
                .get(qid)
                .cloned()
                .ok_or_else(|| WikidataError::NotFound(qid.to_string()))
        }

        async fn get_entity_by_sitelink(
            &self,
            _site: &str,
            title: &str,
            _langs: &[&str],
        ) -> Result<WikidataEntity, WikidataError> {
            let qid =
                self.entities
                    .iter()
                    .find_map(|(qid, entity)| {
                        if entity.labels.values().any(|label| {
                            label.value == title || label.value.replace(' ', "_") == title
                        }) {
                            Some(qid.clone())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| WikidataError::NotFound(title.to_string()))?;
            self.entities
                .get(&qid)
                .cloned()
                .ok_or(WikidataError::NotFound(title.to_string()))
        }

        async fn search_entities(
            &self,
            _query: &str,
            _lang: &str,
            _limit: usize,
        ) -> Result<Vec<SearchResult>, WikidataError> {
            Ok(Vec::new())
        }

        async fn sparql_query(&self, _query: &str) -> Result<Vec<String>, WikidataError> {
            Ok(self.query_results.clone())
        }
    }

    #[cfg(feature = "wikidata")]
    fn make_time(year: i64) -> TimeValue {
        TimeValue {
            time: format!("{year:+05}-01-01T00:00:00Z"),
            precision: 9,
            calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
        }
    }

    #[cfg(feature = "wikidata")]
    fn make_time_statement(property: &str, year: i64) -> Statement {
        Statement {
            mainsnak: Snak {
                snaktype: "value".to_string(),
                property: property.to_string(),
                datavalue: Some(DataValue::Time {
                    value: make_time(year),
                }),
            },
            rank: "normal".to_string(),
            qualifiers: HashMap::new(),
        }
    }

    #[cfg(feature = "wikidata")]
    fn make_entity(id: &str, ja_label: &str, start: i64, end: i64) -> WikidataEntity {
        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: ja_label.to_string(),
            },
        );

        let mut claims = HashMap::new();
        claims.insert("P571".to_string(), vec![make_time_statement("P571", start)]);
        claims.insert("P576".to_string(), vec![make_time_statement("P576", end)]);

        WikidataEntity {
            id: id.to_string(),
            labels,
            claims,
        }
    }

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
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::UnknownLane(_)))
        );
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

    // ─── 月日精度の lowering / 補完ヘルパ (#247) ─────────────────────────

    #[test]
    fn lower_static_event_with_date_precision() {
        let src = r#"
            timeline "T" { title "T"; unit day; range 1969-07-01..1969-07-31; }
            lane "A" as a { kind custom; order 1; }
            event a 1969-07-20 "月面着陸" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::Event {
                time,
                time_month,
                time_day,
                ..
            } => {
                assert_eq!(*time, 1969);
                assert_eq!(*time_month, Some(7));
                assert_eq!(*time_day, Some(20));
            }
            _ => panic!("expected event"),
        }
    }

    #[test]
    fn lower_static_span_with_date_precision() {
        let src = r#"
            timeline "T" { title "T"; unit month; range 1939-09-01..1945-09-30; }
            lane "war" as war { kind custom; order 1; }
            span war 1939-09-01..1945-09-02 "WW2" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::Span {
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                ..
            } => {
                assert_eq!(*start, 1939);
                assert_eq!(*start_month, Some(9));
                assert_eq!(*start_day, Some(1));
                assert_eq!(*end, 1945);
                assert_eq!(*end_month, Some(9));
                assert_eq!(*end_day, Some(2));
            }
            _ => panic!("expected span"),
        }
    }

    #[test]
    fn lower_static_event_range_with_year_month() {
        let src = r#"
            timeline "T" { title "T"; unit month; range 1939-09..1945-09; }
            lane "war" as war { kind custom; order 1; }
            event_range war 1939-09..1945-09 "WW2" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::EventRange {
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                ..
            } => {
                assert_eq!(*start, 1939);
                assert_eq!(*start_month, Some(9));
                assert!(start_day.is_none());
                assert_eq!(*end, 1945);
                assert_eq!(*end_month, Some(9));
                assert!(end_day.is_none());
            }
            _ => panic!("expected event_range"),
        }
    }

    #[test]
    fn lower_meta_range_keeps_precision() {
        let src = r#"
            timeline "T" { title "T"; unit month; range 1939-09..1945-09; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        assert_eq!(ir.meta.range, (1939, 1945));
        assert_eq!(ir.meta.range_start_month, Some(9));
        assert!(ir.meta.range_start_day.is_none());
        assert_eq!(ir.meta.range_end_month, Some(9));
        assert!(ir.meta.range_end_day.is_none());
    }

    #[test]
    fn lower_static_mixed_precision_range() {
        // 仕様 §1.4: 範囲の片端が year、もう片端が date の混在
        let src = r#"
            timeline "T" { title "T"; unit year; range 1900..2000; }
            lane "x" as x { kind custom; order 1; }
            span x 1900..1969-07-20 "Mixed" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::Span {
                start,
                end,
                start_month,
                end_month,
                end_day,
                ..
            } => {
                assert_eq!(*start, 1900);
                assert!(start_month.is_none());
                assert_eq!(*end, 1969);
                assert_eq!(*end_month, Some(7));
                assert_eq!(*end_day, Some(20));
            }
            _ => panic!("expected span"),
        }
    }

    #[test]
    fn lower_meta_range_year_only_no_precision_fields() {
        // 後方互換: year のみの range では新フィールドはすべて None
        let src = r#"
            timeline "T" { title "T"; unit year; range -500..2000; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        assert_eq!(ir.meta.range, (-500, 2000));
        assert!(ir.meta.range_start_month.is_none());
        assert!(ir.meta.range_start_day.is_none());
        assert!(ir.meta.range_end_month.is_none());
        assert!(ir.meta.range_end_day.is_none());

        // JSON 出力に precision フィールドが現れないこと
        let json = serde_json::to_string(&ir).unwrap();
        assert!(!json.contains("range_start_month"));
        assert!(!json.contains("range_end_month"));
    }

    #[test]
    fn ir_start_frac_year_only_uses_jan_first() {
        assert!((ir::start_frac(1939, None, None) - 1939.0).abs() < 1e-9);
    }

    #[test]
    fn ir_end_frac_year_only_uses_year_end() {
        // 1939-12-31 → 1939 + 11/12 + 30/365.25
        let v = ir::end_frac(1939, None, None);
        let expected = 1939.0 + 11.0 / 12.0 + 30.0 / 365.25;
        assert!(
            (v - expected).abs() < 1e-9,
            "end_frac year-only: got {v}, expected {expected}"
        );
    }

    #[test]
    fn ir_end_frac_year_month_uses_month_end() {
        // 1939-02 → 1939 + 1/12 + 27/365.25（1939は非うるう年で28日）
        let v = ir::end_frac(1939, Some(2), None);
        let expected = 1939.0 + 1.0 / 12.0 + 27.0 / 365.25;
        assert!((v - expected).abs() < 1e-9);

        // 1940-02 → うるう年の29日
        let v = ir::end_frac(1940, Some(2), None);
        let expected = 1940.0 + 1.0 / 12.0 + 28.0 / 365.25;
        assert!((v - expected).abs() < 1e-9);
    }

    #[test]
    fn ir_days_in_month_examples() {
        assert_eq!(ir::days_in_month(2024, 2), 29); // うるう年
        assert_eq!(ir::days_in_month(2025, 2), 28);
        assert_eq!(ir::days_in_month(2000, 2), 29); // 400 で割り切れる
        assert_eq!(ir::days_in_month(1900, 2), 28); // 100 で割り切れるが 400 で割れない
        assert_eq!(ir::days_in_month(2025, 4), 30);
        assert_eq!(ir::days_in_month(2025, 12), 31);
    }

    #[test]
    fn decompile_round_trip_with_date_precision() {
        let src = r#"timeline "T" {
    title "T";
    unit day;
    range 1969-07-01..1969-07-31;
}

lane "A" as a { kind custom; order 1; }

event a 1969-07-20 "月面着陸" { id "event:a:1969"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        let regenerated = decompile::decompile(&ir);

        // 月日精度が保たれていること
        assert!(
            regenerated.contains("1969-07-20"),
            "expected date literal, got:\n{regenerated}"
        );

        // 再パース可能であること（roundtrip）
        let reparsed = tdsl_parser::parse(&regenerated);
        assert!(
            reparsed.is_ok(),
            "decompiled output must reparse: {regenerated}\nerror: {:?}",
            reparsed.err()
        );
    }

    #[test]
    fn decompile_round_trip_with_year_month_range() {
        let src = r#"timeline "T" {
    title "T";
    unit month;
    range 1939-09..1945-09;
}

lane "war" as war { kind custom; order 1; }

event_range war 1939-09..1945-09 "WW2" { id "event_range:war:1939"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        let regenerated = decompile::decompile(&ir);

        assert!(
            regenerated.contains("range 1939-09..1945-09"),
            "expected year-month range, got:\n{regenerated}"
        );
        assert!(
            regenerated.contains("1939-09..1945-09"),
            "expected event_range with year-month, got:\n{regenerated}"
        );

        let reparsed = tdsl_parser::parse(&regenerated);
        assert!(reparsed.is_ok(), "{:?}", reparsed.err());
    }

    #[test]
    fn lower_bc_year_does_not_keep_month_day_from_static() {
        // 仕様 §1.3: 紀元前 (-206) は parser 段階で year 精度のみ。
        // 静的 lowering でも year 精度として落とされること。
        let src = r#"
            timeline "T" { title "T"; unit year; range -300..0; }
            lane "qin" as qin { kind dynasty; order 1; }
            event qin -206 "始皇帝崩御" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::Event {
                time,
                time_month,
                time_day,
                ..
            } => {
                assert_eq!(*time, -206);
                assert!(time_month.is_none());
                assert!(time_day.is_none());
            }
            _ => panic!("expected event"),
        }
    }

    #[test]
    fn validate_warns_on_bad_range() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Bad".into(),
                unit: "year".into(),
                range: (100, 0),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn validate_warns_on_span_start_gt_end() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Test".into(),
                unit: "year".into(),
                range: (0, 1000),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![ir::Lane {
                id: "a".into(),
                label: "A".into(),
                kind: "dynasty".into(),
                order: 1,
            }],
            items: vec![ir::Item::Span {
                id: "span:a:200".into(),
                lane: "a".into(),
                start: 200,
                end: 100,
                label: "Bad Span".into(),
                tags: vec![],
                source: None,
                origin: None,
                start_month: None,
                start_day: None,
                end_month: None,
                end_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("start") && w.contains("end")),
            "expected start > end warning, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_warns_on_event_range_start_gt_end() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Test".into(),
                unit: "year".into(),
                range: (0, 1000),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![ir::Lane {
                id: "a".into(),
                label: "A".into(),
                kind: "dynasty".into(),
                order: 1,
            }],
            items: vec![ir::Item::EventRange {
                id: "event_range:a:300".into(),
                lane: "a".into(),
                start: 300,
                end: 150,
                label: "Bad EventRange".into(),
                tags: vec![],
                source: None,
                origin: None,
                start_month: None,
                start_day: None,
                end_month: None,
                end_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("start") && w.contains("end")),
            "expected start > end warning, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_no_warning_on_valid_span() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Test".into(),
                unit: "year".into(),
                range: (0, 1000),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![ir::Lane {
                id: "a".into(),
                label: "A".into(),
                kind: "dynasty".into(),
                order: 1,
            }],
            items: vec![ir::Item::Span {
                id: "span:a:100".into(),
                lane: "a".into(),
                start: 100,
                end: 200,
                label: "Valid Span".into(),
                tags: vec![],
                source: None,
                origin: None,
                start_month: None,
                start_day: None,
                end_month: None,
                end_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(
            warnings.is_empty(),
            "expected no warnings, got: {warnings:?}"
        );
    }

    #[test]
    fn static_event_source_registered_in_sources() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            event a 100 "E" { source wd:Q1234; };
            event_range a 100..200 "ER" { source wd:Q5678; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        assert_eq!(ir.sources.len(), 2);
        assert!(ir.sources.iter().any(|s| s.id == "wd:Q1234"));
        assert!(ir.sources.iter().any(|s| s.id == "wd:Q5678"));
    }

    #[test]
    fn japanese_lane_without_alias_gets_auto_id() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "秦" { kind dynasty; }
            lane "漢" { kind dynasty; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        assert_eq!(ir.lanes.len(), 2);
        assert_eq!(ir.lanes[0].id, "lane_0");
        assert_eq!(ir.lanes[0].label, "秦");
        assert_eq!(ir.lanes[1].id, "lane_1");
        assert_eq!(ir.lanes[1].label, "漢");
    }

    #[test]
    fn static_item_origin_preserved_in_ir() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            span a 100..200 "S" { origin manual; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        match &ir.items[0] {
            ir::Item::Span { origin, .. } => assert_eq!(origin.as_deref(), Some("manual")),
            _ => panic!("expected span"),
        }
    }

    #[test]
    fn ir_json_roundtrip_with_origin() {
        let src = r#"
            timeline "Test" { unit year; range 0..100; }
            lane "A" as a { kind custom; }
            span a 10..20 "S" { origin manual; source wd:Q1; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();

        let json = serde_json::to_string(&ir).unwrap();
        let ir2: ir::TimelineIr = serde_json::from_str(&json).unwrap();

        match &ir2.items[0] {
            ir::Item::Span { origin, source, .. } => {
                assert_eq!(origin.as_deref(), Some("manual"));
                assert_eq!(source.as_deref(), Some("wd:Q1"));
            }
            _ => panic!("expected span"),
        }
        assert_eq!(ir2.sources.len(), 1);
        assert_eq!(ir2.sources[0].id, "wd:Q1");
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_with_wikidata_supports_query_import_mapping_multiple_entities() {
        let src = r#"
            timeline "Dynasties" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                query "SELECT ?item WHERE { ?item wdt:P31 wd:Q28171280 . }" as chinese_dynasties;
            }

            map wd.chinese_dynasties to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
                tags ["imported"];
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();

        let mut entities = HashMap::new();
        entities.insert("Q7183".to_string(), make_entity("Q7183", "秦", -221, -206));
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![
                "Q7209".to_string(),
                "Q7183".to_string(),
                "Q7209".to_string(),
            ],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();

        assert_eq!(ir.items.len(), 2);
        assert_eq!(ir.imports.len(), 2);
        assert!(ir.imports.iter().any(|r| r.qid == "Q7183"));
        assert!(ir.imports.iter().any(|r| r.qid == "Q7209"));

        let labels: Vec<&str> = ir
            .items
            .iter()
            .map(|item| match item {
                ir::Item::Span { label, .. } => label.as_str(),
                _ => panic!("expected span"),
            })
            .collect();
        assert!(labels.contains(&"秦"));
        assert!(labels.contains(&"漢"));
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_with_wikidata_keep_manual_skips_conflicting_imported_item() {
        let src = r#"
            timeline "Policy" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            span dynasty -206..220 "手動の漢" { id "span:q7209:-206"; origin manual; };

            import wikidata as wd {
                entity Q7209 as han_dynasty;
                policy keep_manual;
            }

            map wd.han_dynasty to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
                tags ["imported"];
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        assert_eq!(ir.imports.len(), 0);
        match &ir.items[0] {
            ir::Item::Span { label, origin, .. } => {
                assert_eq!(label, "手動の漢");
                assert_eq!(origin.as_deref(), Some("manual"));
            }
            _ => panic!("expected span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_with_wikidata_overwrite_imported_replaces_previous_imported_item() {
        let src = r#"
            timeline "Policy" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han_single;
                query "SELECT ?item WHERE { VALUES ?item { wd:Q7209 } }" as han_group;
                policy overwrite_imported;
            }

            map wd.han_single to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
                tags ["first"];
            }

            map wd.han_group to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
                tags ["second"];
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec!["Q7209".to_string()],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        assert_eq!(ir.imports.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { tags, .. } => {
                assert_eq!(tags, &vec!["second".to_string()]);
            }
            _ => panic!("expected span"),
        }
        assert_eq!(ir.imports[0].mapped_to, "span:q7209:-206");
    }

    // ─── Edge cases ──────────────────────────────────────────────────────────

    #[test]
    fn lower_detects_no_timeline() {
        let src = r#"lane "A" as a { kind dynasty; }"#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::NoTimeline))
        );
    }

    #[test]
    fn lower_detects_multiple_timelines() {
        let src = r#"
            timeline "A" { unit year; range 0..100; }
            timeline "B" { unit year; range 0..100; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::MultipleTimelines))
        );
    }

    #[test]
    fn lower_custom_id_is_preserved() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            span a 100..200 "S" { id "my-custom-id"; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::Span { id, .. } => assert_eq!(id, "my-custom-id"),
            _ => panic!("expected span"),
        }
    }

    #[test]
    fn lower_event_range_auto_id_format() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            event_range a 50..150 "ER" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        match &ir.items[0] {
            ir::Item::EventRange { id, .. } => assert_eq!(id, "event_range:a:50"),
            _ => panic!("expected event_range"),
        }
    }

    #[test]
    fn lower_duplicate_item_id_is_error() {
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            span a 100..200 "S1" { id "same-id"; };
            span a 300..400 "S2" { id "same-id"; };
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::DuplicateItemId(_)))
        );
    }

    #[test]
    fn lower_import_ignored_in_static_mode() {
        // import block should not cause errors in lower_static
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "A" as a { kind custom; }
            import wikidata as wd { entity Q7209; }
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_ok());
    }

    #[test]
    fn lower_meta_defaults_when_optional_fields_missing() {
        let src = r#"timeline "Minimal" {}"#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        assert_eq!(ir.meta.title, "Minimal");
        assert_eq!(ir.meta.unit, "year");
        assert_eq!(ir.meta.range, (0, 2000));
        assert_eq!(ir.meta.calendar, "proleptic_gregorian");
        assert!(ir.meta.color_map.is_empty());
    }

    #[test]
    fn lower_color_map_in_meta() {
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
            lane "A" as a { kind dynasty; }
            span a 100..200 "テスト" {};
        "##;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        assert_eq!(ir.meta.color_map.len(), 2);
        assert_eq!(
            ir.meta.color_map.get("dynasty").map(String::as_str),
            Some("#3366cc")
        );
        assert_eq!(
            ir.meta.color_map.get("war").map(String::as_str),
            Some("#cc0000")
        );
    }

    #[test]
    fn lower_color_map_serializes_to_json() {
        let src = r##"
            timeline "T" {
                color_map {
                    tag1: "#aabbcc";
                }
            }
        "##;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        let json = serde_json::to_string(&ir).unwrap();
        assert!(json.contains("color_map"));
        assert!(json.contains("tag1"));
        assert!(json.contains("#aabbcc"));
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_wikidata_entity_without_label_skips_item() {
        // An entity with no label results in empty label → item is skipped
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q9999 as nolabel;
            }

            map wd.nolabel to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja ?? label@en;
            }
        "#;

        // Entity with no labels
        let mut entity = make_entity("Q9999", "", -100, 100);
        entity.labels.clear();

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q9999".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 0);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_wikidata_entity_missing_claim_skips_item() {
        // An entity without P571 (start year) results in None → span not generated
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;

        let mut entity = make_entity("Q7209", "漢", -206, 220);
        entity.claims.remove("P571"); // Remove start claim

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 0);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_with_template_apply_generates_items() {
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            template "王朝スパン" as dynasty_tpl
                to span {
                    start claim(P571).year;
                    end claim(P576).year;
                    label label@ja ?? label@en;
                }

            import wikidata as wd {
                entity Q7209 as han;
            }

            apply dynasty_tpl to wd {
                lane dynasty;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span {
                lane,
                label,
                start,
                end,
                ..
            } => {
                assert_eq!(lane, "dynasty");
                assert_eq!(label, "漢");
                assert_eq!(*start, -206);
                assert_eq!(*end, 220);
            }
            _ => panic!("expected span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_apply_lane_override_works() {
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "History" as history { kind custom; order 1; }
            lane "Dynasty" as dynasty { kind dynasty; order 2; }

            template "スパンテンプレート" as tpl
                to span {
                    lane history;
                    start claim(P571).year;
                    end claim(P576).year;
                    label label@ja ?? label@en;
                }

            import wikidata as wd {
                entity Q7209 as han;
            }

            apply tpl to wd {
                lane dynasty;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { lane, .. } => {
                // override should win over template's lane
                assert_eq!(lane, "dynasty");
            }
            _ => panic!("expected span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_apply_unknown_template_is_error() {
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            apply nonexistent_template to wd {
                lane dynasty;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let result = lower::lower_with_wikidata(&file, &client).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::UnknownTemplate(_)))
        );
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn lower_apply_with_sparql_query_no_duplicates() {
        // Regression test: apply block with a SPARQL query import must not create duplicate items.
        // Previously, query entities were processed twice — once from import_entities and once from
        // import_groups — causing DuplicateItemId errors under merge_by_source policy.
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            template "スパンテンプレート" as tpl
                to span {
                    lane dynasty;
                    start claim(P571).year;
                    end claim(P576).year;
                    label label@ja ?? label@en;
                }

            import wikidata as wd {
                query "SELECT ?item WHERE { ?item wdt:P31 wd:Q783794 }" as dynasties;
            }

            apply tpl to wd {}
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        entities.insert("Q7183".to_string(), make_entity("Q7183", "秦", -221, -206));
        let client = MockWikidataClient {
            entities,
            query_results: vec!["Q7209".to_string(), "Q7183".to_string()],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        // Must produce exactly 2 items (one per entity), not 4.
        assert_eq!(ir.items.len(), 2);
    }

    #[test]
    fn lower_static_duplicate_template_is_error() {
        let src = r#"
            timeline "Test" { unit year; range 0..100; }

            template "テンプレート" as tpl
                to span {
                    start claim(P571).year;
                    end claim(P576).year;
                    label label@ja;
                }

            template "別のテンプレート" as tpl
                to event {
                    time claim(P571).year;
                    label label@ja;
                }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let result = lower::lower_static(&file);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, error::LoweringError::DuplicateTemplate(_)))
        );
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_map_expr_unknown_accessor_returns_none() {
        // .month のような未対応アクセサは None を返し、アイテムが生成されない
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                start claim(P571).month;
                end claim(P576).year;
                label label@ja ?? label@en;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        // start が None になるためアイテムは生成されない
        assert_eq!(ir.items.len(), 0);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_map_expr_falls_back_when_first_missing() {
        // P580/P582 が無い → P571/P576 にフォールバック
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                start claim(P580).year ?? claim(P571).year;
                end claim(P582).year ?? claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        // make_entity は P571=-206, P576=220 のみ設定（P580/P582 は無し）
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { start, end, .. } => {
                assert_eq!(*start, -206); // P580 無し → P571 採用
                assert_eq!(*end, 220); // P582 無し → P576 採用
            }
            _ => panic!("expected Span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_map_expr_uses_first_when_present() {
        // P580/P582 と P571/P576 両方ある → 短絡評価で P580/P582 採用
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q1 as e;
            }

            map wd.e to span {
                lane dynasty;
                start claim(P580).year ?? claim(P571).year;
                end claim(P582).year ?? claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entity = make_entity("Q1", "test", -206, 220);
        // P580=100, P582=300 を追加で仕込む
        entity
            .claims
            .insert("P580".to_string(), vec![make_time_statement("P580", 100)]);
        entity
            .claims
            .insert("P582".to_string(), vec![make_time_statement("P582", 300)]);
        let mut entities = HashMap::new();
        entities.insert("Q1".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { start, end, .. } => {
                assert_eq!(*start, 100); // P580 が存在するため採用（P571 は使わない）
                assert_eq!(*end, 300); // P582 が存在するため採用（P576 は使わない）
            }
            _ => panic!("expected Span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_map_expr_all_missing_skips_item() {
        // 全 fallback とも空 → アイテム生成されない
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                start claim(P580).year ?? claim(P9999).year;
                end claim(P582).year ?? claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        // start のフォールバック両方とも None → アイテムスキップ
        assert_eq!(ir.items.len(), 0);
    }

    // ─── filter clause (issue #142) ─────────────────────────

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn filter_excludes_entity_when_false() {
        // P571=500 の entity に対し `filter ... > 1000` → 除外
        let src = r#"
            timeline "Test" { unit year; range -500..2000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                filter claim(P571).year > 1000;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", 500, 700));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 0);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn filter_includes_entity_when_true() {
        // P571=1500 の entity に対し `filter ... > 1000` → 含まれる
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                filter claim(P571).year > 1000;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", 1500, 1700));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn filter_null_check_excludes_when_absent() {
        // P576 を欠いた entity に `filter claim(P576).year != null;` → 除外
        let src = r#"
            timeline "Test" { unit year; range -500..2000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q1 as e;
            }

            map wd.e to span {
                lane dynasty;
                filter claim(P576).year != null;
                start claim(P571).year;
                end claim(P571).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        // P571 のみ持つ entity を作る (P576 不在)
        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "現存王朝".to_string(),
            },
        );
        let mut claims = HashMap::new();
        claims.insert("P571".to_string(), vec![make_time_statement("P571", 100)]);
        let entity = WikidataEntity {
            id: "Q1".to_string(),
            labels,
            claims,
        };
        let mut entities = HashMap::new();
        entities.insert("Q1".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 0);
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn multiple_filters_are_anded() {
        // 2 つの filter のうち片方が false → 除外
        let src = r#"
            timeline "Test" { unit year; range -500..2000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                filter claim(P571).year > 0;
                filter claim(P576).year > 999;
                start claim(P571).year;
                end claim(P576).year;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        // P571=100 > 0 (true), P576=500 > 999 (false) → 除外される
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "test", 100, 500));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 0);
    }

    // ─── source_span regression tests ─────────────────────────────────────────

    #[test]
    fn lower_static_without_source_has_no_source_span() {
        let src = r#"
            timeline "T" { unit year; range 0..100; }
            lane "A" as a { kind dynasty; }
            span a 10..20 "S" {};
            event a 15 "E" {};
            event_range a 30..40 "R" {};
        "#;
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        for item in &ir.items {
            match item {
                ir::Item::Span { source_span, .. } => assert!(source_span.is_none()),
                ir::Item::Event { source_span, .. } => assert!(source_span.is_none()),
                ir::Item::EventRange { source_span, .. } => assert!(source_span.is_none()),
            }
        }
    }

    #[test]
    fn lower_static_with_source_sets_correct_line_numbers() {
        // 各アイテム定義の行番号を検証する。
        // line 1: timeline, line 2: lane, line 3: span, line 4: event, line 5: event_range
        let src = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a { kind dynasty; }\n",
            "span a 10..20 \"S\" {};\n",
            "event a 15 \"E\" {};\n",
            "event_range a 30..40 \"R\" {};\n",
        );
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
        assert_eq!(ir.items.len(), 3);
        for item in &ir.items {
            match item {
                ir::Item::Span {
                    source_span, label, ..
                } => {
                    let ss = source_span.as_ref().expect("span should have source_span");
                    assert_eq!(
                        ss.line, 3,
                        "span '{label}' expected line 3, got {}",
                        ss.line
                    );
                }
                ir::Item::Event {
                    source_span, label, ..
                } => {
                    let ss = source_span.as_ref().expect("event should have source_span");
                    assert_eq!(
                        ss.line, 4,
                        "event '{label}' expected line 4, got {}",
                        ss.line
                    );
                }
                ir::Item::EventRange {
                    source_span, label, ..
                } => {
                    let ss = source_span
                        .as_ref()
                        .expect("event_range should have source_span");
                    assert_eq!(
                        ss.line, 5,
                        "event_range '{label}' expected line 5, got {}",
                        ss.line
                    );
                }
            }
        }
    }

    #[test]
    fn lower_static_with_source_span_col_start_is_one_based() {
        // col_start は 1-based でインデント列を反映する。
        let src = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a { kind dynasty; }\n",
            "span a 1..2 \"S\" {};\n",
        );
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
        let span = ir
            .items
            .iter()
            .find(|i| matches!(i, ir::Item::Span { .. }))
            .unwrap();
        if let ir::Item::Span { source_span, .. } = span {
            let ss = source_span.as_ref().unwrap();
            assert!(
                ss.col_start >= 1,
                "col_start should be ≥1, got {}",
                ss.col_start
            );
            assert_eq!(ss.line, 3, "span should be on line 3, got {}", ss.line);
        }
    }

    #[test]
    fn lower_static_with_source_json_contains_source_span() {
        let src = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a { kind dynasty; }\n",
            "span a 1..2 \"S\" {};\n",
        );
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
        let json = serde_json::to_string(&ir).unwrap();
        assert!(
            json.contains("source_span"),
            "JSON should contain 'source_span'"
        );
        assert!(json.contains("\"line\""), "JSON should contain 'line'");
    }

    #[test]
    fn lower_static_json_omits_source_span_when_none() {
        let src = concat!(
            "timeline \"T\" { unit year; range 0..100; }\n",
            "lane \"A\" as a { kind dynasty; }\n",
            "span a 1..2 \"S\" {};\n",
        );
        let file = tdsl_parser::parse(src).unwrap();
        let ir = lower::lower_static(&file).unwrap();
        let json = serde_json::to_string(&ir).unwrap();
        assert!(
            !json.contains("source_span"),
            "JSON without source should omit 'source_span'"
        );
    }

    // ─── claim_expr offset (#148) ────────────────────────────────────────────

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_claim_expr_positive_offset_applied_to_year() {
        // start claim(P569).year +1: 誕生年 + 1 = span start になる
        let src = r#"
            timeline "Test" { unit year; range 0..2000; }
            lane "People" as people { kind custom; order 1; }

            import wikidata as wd {
                entity Q1 as person;
            }

            map wd.person to span {
                lane people;
                start claim(P571).year +1;
                end claim(P576).year -5;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();

        // P571=100, P576=220 の entity を作る
        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "テスト人物".to_string(),
            },
        );
        let mut claims = HashMap::new();
        claims.insert("P571".to_string(), vec![make_time_statement("P571", 100)]);
        claims.insert("P576".to_string(), vec![make_time_statement("P576", 220)]);
        let entity = WikidataEntity {
            id: "Q1".to_string(),
            labels,
            claims,
        };
        let mut entities = HashMap::new();
        entities.insert("Q1".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { start, end, .. } => {
                assert_eq!(*start, 101); // 100 + 1
                assert_eq!(*end, 215); // 220 - 5
            }
            _ => panic!("expected Span"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_claim_expr_negative_offset_applied_to_year() {
        // start claim(P571).year -30: 30年前
        let src = r#"
            timeline "Test" { unit year; range -100..2000; }
            lane "People" as people { kind custom; order 1; }

            import wikidata as wd {
                entity Q1 as person;
            }

            map wd.person to event {
                lane people;
                time claim(P571).year -30;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();

        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "test".to_string(),
            },
        );
        let mut claims = HashMap::new();
        claims.insert("P571".to_string(), vec![make_time_statement("P571", 200)]);
        let entity = WikidataEntity {
            id: "Q1".to_string(),
            labels,
            claims,
        };
        let mut entities = HashMap::new();
        entities.insert("Q1".to_string(), entity);
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Event { time, .. } => {
                assert_eq!(*time, 170); // 200 - 30
            }
            _ => panic!("expected Event"),
        }
    }

    #[cfg(feature = "wikidata")]
    #[tokio::test]
    async fn eval_claim_expr_zero_offset_is_noop() {
        // オフセット 0 はオフセットなしと同じ結果
        let src = r#"
            timeline "Test" { unit year; range -500..1000; }
            lane "Dynasty" as dynasty { kind dynasty; order 1; }

            import wikidata as wd {
                entity Q7209 as han;
            }

            map wd.han to span {
                lane dynasty;
                start claim(P571).year +0;
                end claim(P576).year +0;
                label label@ja;
            }
        "#;

        let file = tdsl_parser::parse(src).unwrap();
        let mut entities = HashMap::new();
        entities.insert("Q7209".to_string(), make_entity("Q7209", "漢", -206, 220));
        let client = MockWikidataClient {
            entities,
            query_results: vec![],
        };

        let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
        assert_eq!(ir.items.len(), 1);
        match &ir.items[0] {
            ir::Item::Span { start, end, .. } => {
                assert_eq!(*start, -206);
                assert_eq!(*end, 220);
            }
            _ => panic!("expected Span"),
        }
    }
}
