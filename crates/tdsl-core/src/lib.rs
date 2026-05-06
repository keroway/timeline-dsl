pub mod decompile;
pub mod error;
pub mod ir;
pub mod lower;
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

    #[test]
    fn validate_warns_on_bad_range() {
        let ir = ir::TimelineIr {
            meta: ir::Meta {
                title: "Bad".into(),
                unit: "year".into(),
                range: (100, 0),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(
            warnings.iter().any(|w| w.contains("start") && w.contains("end")),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let warnings = validate::validate(&ir);
        assert!(
            warnings.iter().any(|w| w.contains("start") && w.contains("end")),
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
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::NoTimeline)));
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
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::MultipleTimelines)));
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
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::DuplicateItemId(_))));
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
        assert_eq!(ir.meta.color_map.get("dynasty").map(String::as_str), Some("#3366cc"));
        assert_eq!(ir.meta.color_map.get("war").map(String::as_str), Some("#cc0000"));
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
            ir::Item::Span { lane, label, start, end, .. } => {
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
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::UnknownTemplate(_))));
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
        assert!(errors.iter().any(|e| matches!(e, error::LoweringError::DuplicateTemplate(_))));
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
}
