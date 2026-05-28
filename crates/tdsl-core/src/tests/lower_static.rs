use crate::{decompile, error, ir, lower};

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
