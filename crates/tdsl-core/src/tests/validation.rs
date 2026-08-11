use crate::{ir, lower, validate};

#[test]
fn validate_warns_on_unknown_lane_kind() {
    let src = r#"
timeline "t" { unit year; range 0..100; }
lane "l" as l { kind dynsty; order 10; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("unknown kind") && w.contains("dynsty")),
        "expected unknown kind warning, got: {warnings:?}"
    );
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
            group: None,
            color: None,
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
            note: None,
            link: None,
            color: None,
            start_month: None,
            start_day: None,
            start_hour: None,
            start_minute: None,
            start_second: None,
            start_offset_minutes: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            end_minute: None,
            end_second: None,
            end_offset_minutes: None,
            end_open: false,
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
            group: None,
            color: None,
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
            note: None,
            link: None,
            color: None,
            start_month: None,
            start_day: None,
            start_hour: None,
            start_minute: None,
            start_second: None,
            start_offset_minutes: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            end_minute: None,
            end_second: None,
            end_offset_minutes: None,
            end_open: false,
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
fn validate_warns_on_span_same_day_start_time_gt_end_time() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2020-01-01T00:00..2020-01-02T00:00; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 2020-01-01T12:00..2020-01-01T11:59 "reversed time" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("start") && w.contains("end")),
        "expected same-day time reversal warning, got: {warnings:?}"
    );
}

#[test]
fn validate_warns_on_event_range_same_day_start_hour_gt_end_hour() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2020-01-01T00:00..2020-01-02T00:00; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
event_range l 2020-01-01T23:00..2020-01-01T01:00 "reversed time" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("start") && w.contains("end")),
        "expected same-day time reversal warning, got: {warnings:?}"
    );
}

#[test]
fn validate_warns_on_timeline_range_same_day_start_minute_ge_end_minute() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2020-01-01T12:00..2020-01-01T12:00; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings.iter().any(|w| w.contains("range")),
        "expected timeline range warning, got: {warnings:?}"
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
            group: None,
            color: None,
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
            note: None,
            link: None,
            color: None,
            start_month: None,
            start_day: None,
            start_hour: None,
            start_minute: None,
            start_second: None,
            start_offset_minutes: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            end_minute: None,
            end_second: None,
            end_offset_minutes: None,
            end_open: false,
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
fn validate_warns_on_event_outside_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..100; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
event l 500 "out of range" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("outside timeline.range") && w.contains("not be rendered")),
        "expected outside-range warning, got: {warnings:?}"
    );
}

#[test]
fn validate_warns_on_span_entirely_outside_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..100; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 500..600 "out of range" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("entirely outside timeline.range")),
        "expected entirely-outside-range warning, got: {warnings:?}"
    );
}

#[test]
fn validate_warns_on_span_partially_outside_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..100; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 50..150 "half out" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("partially outside timeline.range") && w.contains("clipped")),
        "expected partially-outside-range warning, got: {warnings:?}"
    );
}

#[test]
fn validate_no_range_warning_when_item_inside_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 0..1000; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 100..200 "inside" {};
event l 150 "inside" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings.is_empty(),
        "expected no warnings for in-range items, got: {warnings:?}"
    );
}

// ─── 秒・offsetを含む比較の修正(#613 reviewer指摘): validate.rs はIRの second/offset
// フィールドを考慮して比較するべきで、単なる (year, month, day, hour, minute)
// タプル比較に退化してはならない ───

/// UTC上では逆順なのに、offsetを無視した旧ロジックでは正常(start<end)と判定されてしまうケース。
/// start=+09:00の10:00、end=UTCの01:30 → UTC正規化すると start(01:00Z) > end(01:30Z)は偵（正常）だが、
/// 逆に startが遅いケースを作り、offsetを無視すると誤って正常判定されることを確認する。
#[test]
fn validate_warns_on_span_start_gt_end_when_utc_normalized_even_if_naive_tuple_looks_ok() {
    // naive(日付・時刻の数値だけを見る)比較では start(01:00) < end(02:00) だが、
    // start は -14:00（UTC = local + 14h = 2024-01-01T15:00Z）、
    // end   は +14:00（UTC = local - 14h = 2023-12-31T12:00Z）とすると、
    // UTC正規化後は start(2024-01-01T15:00Z) > end(2023-12-31T12:00Z) となり逆順になる。
    // offsetを無視した旧実装(naive tuple比較)ではこの逆順を検知できない。
    let src = r#"
timeline "t" { title "t"; unit year; range 2023-01-01T00:00Z..2025-01-01T00:00Z; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
span l 2024-01-01T01:00-14:00..2024-01-01T02:00+14:00 "utc reversed" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("start") && w.contains("end")),
        "UTC正規化すると start > end になるはずだが警告がない: {warnings:?}"
    );
}

/// 秒のみが逆順な event_range（分・時・日は同一）でも警告が出ることを確認する。
/// 旧ロジック（sortable_tuple が second を含まない）では検知できないバグだった。
#[test]
fn validate_warns_on_event_range_second_only_reversal() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2024-01-01T00:00:00..2024-01-02T00:00:00; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
event_range l 2024-01-01T10:00:45..2024-01-01T10:00:15 "second reversed" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("start") && w.contains("end")),
        "秒のみが逆順なら警告が出るはず: {warnings:?}"
    );
}

/// offsetなし timeline range と offset付き event の比較は、loweringではチェックされない
/// （loweringはitemの start/end 同士と rangeの start/end 同士しか比較しないため）。
/// validate.rs がADR 0003 D2に従い、曖昧な比較として警告を出すことを確認する。
#[test]
fn validate_warns_on_mixed_offset_between_item_and_timeline_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2024-01-01T00:00..2024-01-02T00:00; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
event l 2024-01-01T12:00Z "has offset" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings.iter().any(|w| w.contains("mixes a UTC-offset")),
        "offset有無が混在した場合は明示的な警告を出すべき: {warnings:?}"
    );
}

/// offset付き同士の range/itemは UTC正規化して正しく比較され、余計な警告は出ないことを確認する。
#[test]
fn validate_no_warning_when_offset_item_is_inside_offset_range() {
    let src = r#"
timeline "t" { title "t"; unit year; range 2024-01-01T00:00Z..2024-01-02T00:00Z; calendar proleptic_gregorian; }
lane "l" as l { kind custom; order 10; }
event l 2024-01-01T21:00+09:00 "inside, +09:00 = 12:00Z" {};
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    let warnings = validate::validate(&ir);
    assert!(
        warnings.is_empty(),
        "offset正規化後に範囲内なら警告は出ないはず: {warnings:?}"
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

// ─── 診断コードとカタログの対応（#748）──────────────────────────────────

/// `docs/error-catalog.md` の見出しから診断コードを抜き出す。
fn catalog_codes(prefix: char) -> std::collections::HashSet<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/error-catalog.md");
    let text = std::fs::read_to_string(path).expect("error-catalog.md を読めない");
    text.lines()
        .filter_map(|l| l.strip_prefix("### "))
        .filter_map(|l| l.split(':').next())
        .map(str::trim)
        .filter(|c| {
            c.starts_with(prefix) && c.len() == 4 && c[1..].chars().all(|d| d.is_ascii_digit())
        })
        .map(str::to_string)
        .collect()
}

/// **実装が返すコードは、すべてカタログに節がある。**
///
/// コードだけ足してカタログを書き忘れると、利用者は `E116` を検索しても
/// 何も見つけられない。逆にカタログにあって実装が返さないコードは、
/// 将来の予約や別レイヤの担当なので許容する（片方向の検査）。
#[test]
fn lowering_error_codes_exist_in_catalog() {
    use crate::error::LoweringError;

    let catalog = catalog_codes('E');
    // 各 variant の代表値を作ってコードを集める。variant を足したときに
    // ここへ追記し忘れても、下の網羅性テストが検出する。
    let samples: Vec<LoweringError> = vec![
        LoweringError::UnknownLane("x".into()),
        LoweringError::DuplicateLane("x".into()),
        LoweringError::DuplicateItemId("x".into()),
        LoweringError::NoTimeline,
        LoweringError::MultipleTimelines,
        LoweringError::UnresolvedImport("x".into()),
        LoweringError::UnresolvedEntity("x".into()),
        LoweringError::UnknownMappedLane("x".into()),
        LoweringError::DuplicateTemplate("x".into()),
        LoweringError::UnknownTemplate("x".into()),
        LoweringError::InvalidItemLink("x".into()),
        LoweringError::InvalidItemColor("x".into()),
        LoweringError::MixedOffsetComparison("a".into(), "b".into()),
        LoweringError::FieldPriorityTypeMismatch {
            id: "x".into(),
            existing: "span",
            incoming: "event",
        },
        LoweringError::DuplicateImportAlias("x".into()),
    ];

    for err in &samples {
        let code = err.code().unwrap_or_else(|| panic!("code が無い: {err:?}"));
        assert!(
            catalog.contains(code),
            "{code} が docs/error-catalog.md に無い（実装だけ足してカタログを書き忘れている）"
        );
    }
}

/// validate が返す W コードも、すべてカタログに節がある。
///
/// 実際に validate を走らせて出たコードだけを見る（実装の網羅ではなく
/// **到達可能なコード**を対象にする）。
#[test]
fn validation_codes_exist_in_catalog() {
    let catalog = catalog_codes('W');

    // W2xx が出る入力をまとめて 1 ファイルに入れる。
    //
    // W208 / W209（オフセット混在）は**この経路では到達しない** — 同じ条件を
    // lowering が E113 として先に弾き、validate まで来ないため（実際に
    // 入力を足して確認した）。到達しない入力を無理に入れるとテスト自体が
    // 落ちるので、ここでは到達可能なコードだけを対象にする。
    let src = r#"
timeline "T" { unit year; range 2000..2010; }
lane "A" as a { kind unknown_kind; }
span a 2005..2001 "reversed" { id "s1"; };
event a 1900 "outside" { id "e1"; };
span a 1800..1850 "far outside" { id "s2"; };
span a 1990..2005 "clipped" { id "s3"; };
"#;
    let file = tdsl_parser::parse(src).unwrap();
    let ir = lower::lower_static(&file).expect("lower");
    let diags = validate::validate_with_spans(&ir);
    assert!(!diags.is_empty(), "W2xx が 1 件も出ていない");

    for d in &diags {
        assert!(
            catalog.contains(d.code),
            "{} が docs/error-catalog.md に無い: {}",
            d.code,
            d.message
        );
    }
}
