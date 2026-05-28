use crate::{ir, lower, validate};

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
            source_span: None,
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
            source_span: None,
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
            source_span: None,
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

// ─── validate_with_spans テスト ──────────────────────────────────────

/// start > end の span → warning 診断が返り、span.message に span ID を含む。
#[test]
fn validate_with_spans_start_gt_end() {
    // span の文法: span <lane_id> <start>..<end> "label" { ... }
    let src = r#"
timeline "t" { title "t"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 500..100 "reversed" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
    let diags = validate::validate_with_spans(&ir);
    assert!(!diags.is_empty(), "start>end の span は警告を返す");
    // 自動生成 ID に "500" が含まれる（format_id_time で "500" になる）
    assert!(
        diags.iter().any(|d| d.message.contains("500")),
        "start 値を含む警告があるべき"
    );
}

/// source_span が付与された item の診断は span を持つ（Some）。
#[test]
fn validate_with_spans_has_source_span_when_source_provided() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 500..100 "reversed" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    // source を渡すと source_span が付与される
    let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
    let diags = validate::validate_with_spans(&ir);
    // start>end の警告があるはず
    let bad_diag = diags.iter().find(|d| d.message.contains("start"));
    assert!(bad_diag.is_some(), "start>end 警告が存在する");
    // ソースを渡しているので source_span は Some になるはず
    assert!(
        bad_diag.unwrap().span.is_some(),
        "source あり lowering では source_span が付与される"
    );
}

/// range 不整合の警告は span: None（アイテムに紐付かない）。
#[test]
fn validate_with_spans_range_incoherence_has_no_span() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2000..1000; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static_with_source(&file, Some(src)).unwrap();
    let diags = validate::validate_with_spans(&ir);
    let range_diag = diags.iter().find(|d| d.message.contains("range"));
    assert!(range_diag.is_some(), "range 不整合の警告が存在する");
    assert!(
        range_diag.unwrap().span.is_none(),
        "range 不整合の警告は span: None"
    );
}

/// 既存の validate() が validate_with_spans() と同じ文字列を返す（後方互換）。
#[test]
fn validate_backward_compat_with_spans() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 500..100 "reversed" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let old_msgs: Vec<String> = validate::validate(&ir);
    let new_msgs: Vec<String> = validate::validate_with_spans(&ir)
        .into_iter()
        .map(|d| d.message)
        .collect();
    assert_eq!(
        old_msgs, new_msgs,
        "validate() と validate_with_spans() のメッセージが一致する"
    );
}

// ─── validate_static_references テスト ────────────────────────────────

/// 宣言済み alias / template を参照する map・apply は参照エラーを出さない。
#[test]
fn static_refs_valid_file_has_no_errors() {
    let src = r#"
timeline "t" { title "t"; unit year; range -500..700; calendar proleptic_gregorian; }
lane "d" as d { kind dynasty; order 1; }
template "tmpl" as dynasty_span to span {
    start claim(P571).year;
    end claim(P576).year;
    label label@ja;
}
import wikidata as wd { entity Q7209 as han; }
map wd.han to span { lane d; start claim(P571).year; end claim(P576).year; label label@ja; }
apply dynasty_span to wd { lane d; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let diags = validate::validate_static_references(&file);
    assert!(
        diags.is_empty(),
        "宣言済みの参照のみなら参照エラーは出ない。実際: {diags:#?}"
    );
}

/// 未宣言 import alias を参照する map はエラー。span は当該 map 文を指す。
#[test]
fn static_refs_map_undeclared_alias() {
    let src = r#"
timeline "t" { title "t"; unit year; range -500..700; calendar proleptic_gregorian; }
lane "d" as d { kind dynasty; order 1; }
import wikidata as wd { entity Q7209 as han; }
map typo.han to span { lane d; start claim(P571).year; end claim(P576).year; label label@ja; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let diags = validate::validate_static_references(&file);
    assert_eq!(diags.len(), 1, "未宣言 alias 参照は 1 件。実際: {diags:#?}");
    assert!(diags[0].message.contains("typo"));
    // span が map 文のバイト範囲を指す（start < end かつソース長以内）
    assert!(diags[0].span.start < diags[0].span.end);
    assert!(diags[0].span.end <= src.len());
}

/// 未宣言の template / import を参照する apply は 2 件のエラー。
#[test]
fn static_refs_apply_undeclared_template_and_import() {
    let src = r#"
timeline "t" { title "t"; unit year; range -500..700; calendar proleptic_gregorian; }
lane "d" as d { kind dynasty; order 1; }
apply missing_tmpl to missing_import { lane d; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let diags = validate::validate_static_references(&file);
    assert_eq!(
        diags.len(),
        2,
        "未宣言 import + template = 2 件。実際: {diags:#?}"
    );
    assert!(diags.iter().any(|d| d.message.contains("missing_import")));
    assert!(diags.iter().any(|d| d.message.contains("missing_tmpl")));
}
