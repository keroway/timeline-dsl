use std::collections::HashMap;

use super::helpers::{MockWikidataClient, make_entity, make_time_statement};
use crate::{error, ir, lower};
use tdsl_wikidata::WikidataEntity;
use tdsl_wikidata::entity::LabelValue;

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

// ─── claim_expr offset (#148) ────────────────────────────────────────────

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

#[tokio::test]
async fn unknown_lane_stops_before_wikidata_fetch() {
    // 未宣言 lane があるとき、Wikidata フェッチ前に early exit して UnknownLane を返す。
    // MockWikidataClient は空（Q99999 不在）なので、pass3 が実行されると
    // LoweringError::Wikidata(NotFound) が追加される。
    // early exit が正しく機能すれば Wikidata エラーは含まれない。
    let src = r#"
        timeline "Test" { unit year; range 0..2000; }
        lane "A" as a { kind dynasty; }

        span nonexistent_lane 100..200 "Foo" {};

        import wikidata as wd {
            entity Q99999 as some_entity;
        }

        map wd.some_entity to span {
            lane a;
            start claim(P571).year;
            end claim(P576).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();
    let client = MockWikidataClient {
        entities: HashMap::new(),
        query_results: vec![],
    };

    let result = lower::lower_with_wikidata(&file, &client).await;
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, error::LoweringError::UnknownLane(_))),
        "UnknownLane エラーが含まれていること"
    );
    assert!(
        !errors
            .iter()
            .any(|e| matches!(e, error::LoweringError::Wikidata(_))),
        "Wikidata フェッチは UnknownLane エラー検出後は実行されないこと"
    );
}
