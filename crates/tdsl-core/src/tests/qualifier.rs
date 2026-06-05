/// Wikidata qualifier マッピング（issue #361）のテスト。
///
/// - `claim(P39).qualifier(P580).year` 形式で qualifier にアクセスできる
/// - `expand claim(P39);` で複数 statement から複数アイテムを生成できる
/// - qualifier が無い場合は値なし（silent fallback しない）
/// - deprecated statement は skip される
/// - 既存の qualifier なし claim(P571).year は引き続き動作する
use std::collections::HashMap;

use super::helpers::{MockWikidataClient, make_time, make_time_statement};
use crate::{ir, lower};
use tdsl_wikidata::WikidataEntity;
use tdsl_wikidata::entity::{DataValue, LabelValue, Snak, Statement};

// ─── ヘルパー ──────────────────────────────────────────────────────────

fn make_label(lang: &str, value: &str) -> LabelValue {
    LabelValue {
        language: lang.to_string(),
        value: value.to_string(),
    }
}

/// qualifier 付きの Statement を構築する。
/// `main_value` は mainsnak の DataValue（WikibaseEntityId として扱う）。
fn make_stmt_with_time_qualifiers(
    rank: &str,
    main_prop: &str,
    qualifiers: &[(&str, i64)],
) -> Statement {
    // mainsnak は string 値（entity ID など）を使う — qualifier のテストには mainsnak の値は不要
    let mainsnak = Snak {
        snaktype: "value".to_string(),
        property: main_prop.to_string(),
        datavalue: Some(DataValue::String {
            value: "Q999".to_string(),
        }),
    };

    let mut qualifier_map: HashMap<String, Vec<Snak>> = HashMap::new();
    for (prop, year) in qualifiers {
        qualifier_map.insert(
            prop.to_string(),
            vec![Snak {
                snaktype: "value".to_string(),
                property: prop.to_string(),
                datavalue: Some(DataValue::Time {
                    value: make_time(*year),
                }),
            }],
        );
    }

    Statement {
        mainsnak,
        rank: rank.to_string(),
        qualifiers: qualifier_map,
    }
}

fn make_entity_with_offices(
    id: &str,
    label_ja: &str,
    offices: &[(&str, i64, i64)], // (rank, start_year, end_year)
) -> WikidataEntity {
    let mut labels = HashMap::new();
    labels.insert("ja".to_string(), make_label("ja", label_ja));

    let stmts: Vec<Statement> = offices
        .iter()
        .map(|(rank, start, end)| {
            make_stmt_with_time_qualifiers(rank, "P39", &[("P580", *start), ("P582", *end)])
        })
        .collect();

    let mut claims = HashMap::new();
    claims.insert("P39".to_string(), stmts);

    WikidataEntity {
        id: id.to_string(),
        labels,
        claims,
    }
}

// ─── テスト ───────────────────────────────────────────────────────────

#[tokio::test]
async fn qualifier_access_generates_single_item_without_expand() {
    // expand なし: 最初の non-deprecated statement の qualifier を使う
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();
    let entity = make_entity_with_offices("Q1", "テスト人物", &[("normal", 1868, 1879)]);

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
            assert_eq!(*start, 1868);
            assert_eq!(*end, 1879);
        }
        _ => panic!("expected Span"),
    }
}

#[tokio::test]
async fn qualifier_missing_returns_no_item() {
    // P39 statement に P580/P582 qualifier が無い → start/end が None → アイテム生成されない
    // これは silent fallback ではなく、単に値なし = 生成条件を満たさない
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();

    // P39 の statement に qualifier なし
    let stmt_without_qualifiers = make_stmt_with_time_qualifiers("normal", "P39", &[]);
    let mut labels = HashMap::new();
    labels.insert("ja".to_string(), make_label("ja", "テスト人物"));
    let mut claims = HashMap::new();
    claims.insert("P39".to_string(), vec![stmt_without_qualifiers]);
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
    // qualifier が無いため start が None → アイテム生成されない（silent fallback なし）
    assert_eq!(ir.items.len(), 0);
}

#[tokio::test]
async fn expand_generates_multiple_items() {
    // P39 statement が 2 件 → 2 件の span が生成される
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            expand claim(P39);
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();
    let entity = make_entity_with_offices(
        "Q1",
        "テスト人物",
        &[("normal", 1868, 1879), ("normal", 1885, 1888)],
    );

    let mut entities = HashMap::new();
    entities.insert("Q1".to_string(), entity);
    let client = MockWikidataClient {
        entities,
        query_results: vec![],
    };

    let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
    assert_eq!(
        ir.items.len(),
        2,
        "2 件の statement から 2 件の span が生成される"
    );

    let starts: Vec<i64> = ir
        .items
        .iter()
        .map(|item| match item {
            ir::Item::Span { start, .. } => *start,
            _ => panic!("expected Span"),
        })
        .collect();
    assert!(starts.contains(&1868), "1868 が含まれる");
    assert!(starts.contains(&1885), "1885 が含まれる");
}

#[tokio::test]
async fn expand_skips_deprecated_statements() {
    // deprecated の statement は skip される
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            expand claim(P39);
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();
    // normal 1 件 + deprecated 1 件
    let entity = make_entity_with_offices(
        "Q1",
        "テスト人物",
        &[("normal", 1868, 1879), ("deprecated", 1800, 1810)],
    );

    let mut entities = HashMap::new();
    entities.insert("Q1".to_string(), entity);
    let client = MockWikidataClient {
        entities,
        query_results: vec![],
    };

    let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
    // deprecated は skip されるので 1 件のみ
    assert_eq!(ir.items.len(), 1, "deprecated を除く 1 件のみ生成される");
    match &ir.items[0] {
        ir::Item::Span { start, .. } => assert_eq!(*start, 1868),
        _ => panic!("expected Span"),
    }
}

#[tokio::test]
async fn expand_empty_statements_generates_no_items() {
    // P39 statement が 0 件 → アイテム生成なし
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            expand claim(P39);
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();

    // P39 が存在しない entity
    let mut labels = HashMap::new();
    labels.insert("ja".to_string(), make_label("ja", "テスト人物"));
    let entity = WikidataEntity {
        id: "Q1".to_string(),
        labels,
        claims: HashMap::new(),
    };

    let mut entities = HashMap::new();
    entities.insert("Q1".to_string(), entity);
    let client = MockWikidataClient {
        entities,
        query_results: vec![],
    };

    let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
    assert_eq!(ir.items.len(), 0, "statement なし → アイテム生成なし");
}

#[tokio::test]
async fn expand_generates_unique_ids_per_statement() {
    // 複数 statement が同じ start year を持つ場合でも ID が重複しないこと
    let src = r#"
        timeline "Test" { unit year; range 1800..2000; }
        lane "Offices" as offices { kind custom; order 1; }

        import wikidata as wd {
            entity Q1 as person;
        }

        map wd.person to span {
            lane offices;
            expand claim(P39);
            start claim(P39).qualifier(P580).year;
            end   claim(P39).qualifier(P582).year;
            label label@ja;
        }
    "#;

    let file = tdsl_parser::parse(src).unwrap();
    // 両方とも start=1868 (同一 year)
    let entity = make_entity_with_offices(
        "Q1",
        "テスト人物",
        &[("normal", 1868, 1879), ("normal", 1868, 1888)],
    );

    let mut entities = HashMap::new();
    entities.insert("Q1".to_string(), entity);
    let client = MockWikidataClient {
        entities,
        query_results: vec![],
    };

    let ir = lower::lower_with_wikidata(&file, &client).await.unwrap();
    // expand index が ID に含まれるので重複せず 2 件生成される
    assert_eq!(
        ir.items.len(),
        2,
        "同一 year でも expand index で ID が一意になる"
    );
    let ids: Vec<&str> = ir
        .items
        .iter()
        .map(|item| match item {
            ir::Item::Span { id, .. } => id.as_str(),
            _ => panic!("expected Span"),
        })
        .collect();
    // ID が異なること
    assert_ne!(ids[0], ids[1], "expand された 2 アイテムの ID は異なる");
}

#[tokio::test]
async fn existing_claim_without_qualifier_still_works() {
    // qualifier を指定しない既存の claim(P571).year は引き続き動作する（後方互換）
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

    let file = tdsl_parser::parse(src).unwrap();

    let mut labels = HashMap::new();
    labels.insert("ja".to_string(), make_label("ja", "漢"));
    let mut claims = HashMap::new();
    claims.insert("P571".to_string(), vec![make_time_statement("P571", -206)]);
    claims.insert("P576".to_string(), vec![make_time_statement("P576", 220)]);
    let entity = WikidataEntity {
        id: "Q7209".to_string(),
        labels,
        claims,
    };

    let mut entities = HashMap::new();
    entities.insert("Q7209".to_string(), entity);
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
