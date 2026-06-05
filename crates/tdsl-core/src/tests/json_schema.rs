use schemars::schema_for;

use crate::ir::TimelineIr;

/// `TimelineIr` の JSON Schema が生成でき、required な最上位プロパティを含むことを確認する。
#[test]
fn json_schema_contains_top_level_properties() {
    let schema = schema_for!(TimelineIr);
    let json = serde_json::to_string(&schema).expect("schema serialization failed");
    assert!(json.contains("\"meta\""), "schema should contain 'meta'");
    assert!(json.contains("\"lanes\""), "schema should contain 'lanes'");
    assert!(json.contains("\"items\""), "schema should contain 'items'");
}

/// スキーマが有効な JSON として解析可能で、`$schema` キーを持つことを確認する。
#[test]
fn json_schema_is_valid_json_with_schema_key() {
    let schema = schema_for!(TimelineIr);
    let value: serde_json::Value =
        serde_json::to_value(&schema).expect("schema to serde_json::Value failed");
    assert!(
        value.get("$schema").is_some(),
        "schema should have a $schema key"
    );
}

/// Item の type タグ（span / event / event_range）がスキーマに反映されることを確認する。
#[test]
fn json_schema_item_enum_variants() {
    let schema = schema_for!(TimelineIr);
    let json = serde_json::to_string(&schema).expect("schema serialization failed");
    assert!(
        json.contains("\"span\""),
        "schema should contain span variant"
    );
    assert!(
        json.contains("\"event\""),
        "schema should contain event variant"
    );
    assert!(
        json.contains("\"event_range\""),
        "schema should contain event_range variant"
    );
}

/// Meta 型のドキュメント文字列が description に反映されることを確認する。
#[test]
fn json_schema_meta_has_description() {
    let schema = schema_for!(TimelineIr);
    let json = serde_json::to_string_pretty(&schema).expect("schema serialization failed");
    // Meta 型に付いたドキュメントコメントが description として出力される
    assert!(
        json.contains("description"),
        "schema should contain description fields from doc comments"
    );
}
