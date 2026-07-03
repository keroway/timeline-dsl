//! Timeline DSL キーワード定数。
//!
//! **単一真実源は `apps/webui/src/lang-tdsl/keywords.json`** です。
//! 本ファイルはそのミラーであり、末尾のドリフト防止テストで同期を保証します。
//!
//! keywords.json を変更した場合は、本ファイルの定数も同じ順序・内容で更新し、
//! `cargo test -p tdsl-lsp` でドリフト防止テストが通ることを確認してください。

/// ブロックキーワード（`BLOCK_KEYWORDS`）。
///
/// `timeline`, `lane`, `import` 等、DSL のトップレベルブロックを開始するキーワード。
pub const BLOCK_KEYWORDS: &[&str] = &[
    "timeline",
    "lane",
    "group",
    "import",
    "map",
    "template",
    "apply",
    "color_map",
    "policy",
];

/// アイテムキーワード（`ITEM_KEYWORDS`）。
///
/// `span`, `event`, `event_range` の 3 種。
pub const ITEM_KEYWORDS: &[&str] = &["span", "event", "event_range"];

/// その他のキーワード（`MISC_KEYWORDS`）。
///
/// プロパティ名・修飾子・値として使われるキーワード群。
pub const MISC_KEYWORDS: &[&str] = &[
    "as",
    "query",
    "wikidata",
    "unit",
    "range",
    "calendar",
    "kind",
    "order",
    "tags",
    "source",
    "label",
    "start",
    "end",
    "time",
    "id",
    "target_type",
    "target_lane",
    "merge_by_source",
    "overwrite_imported",
    "keep_manual",
    "proleptic_gregorian",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "now",
    "dynasty",
    "person",
    "country",
    "custom",
    "era",
    "title",
    "field_priority",
    "origin",
    "expand",
    "qualifier",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// `keywords.json` の単一真実源と Rust 定数のドリフト防止テスト。
    ///
    /// `apps/webui/src/lang-tdsl/keywords.json` を読み込み、3 つの配列が
    /// Rust 定数と順序込みで完全一致することを検証する。
    #[test]
    fn keywords_match_json_source() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let json_path =
            std::path::Path::new(manifest_dir).join("../../apps/webui/src/lang-tdsl/keywords.json");
        let json_src = std::fs::read_to_string(&json_path).unwrap_or_else(|e| {
            panic!(
                "keywords.json を読み込めない: {} (パス: {})",
                e,
                json_path.display()
            )
        });
        let parsed: serde_json::Value = serde_json::from_str(&json_src)
            .unwrap_or_else(|e| panic!("keywords.json をパースできない: {e}"));

        let json_array = |name: &str| -> Vec<String> {
            parsed[name]
                .as_array()
                .unwrap_or_else(|| panic!("keywords.json に配列 `{name}` がない"))
                .iter()
                .map(|v| {
                    v.as_str()
                        .unwrap_or_else(|| panic!("keywords.json の `{name}` に非文字列要素がある"))
                        .to_string()
                })
                .collect()
        };

        let json_block = json_array("BLOCK_KEYWORDS");
        let rust_block: Vec<&str> = BLOCK_KEYWORDS.to_vec();
        assert_eq!(
            rust_block, json_block,
            "BLOCK_KEYWORDS がドリフトしています。\nRust: {rust_block:?}\nJSON: {json_block:?}"
        );

        let json_item = json_array("ITEM_KEYWORDS");
        let rust_item: Vec<&str> = ITEM_KEYWORDS.to_vec();
        assert_eq!(
            rust_item, json_item,
            "ITEM_KEYWORDS がドリフトしています。\nRust: {rust_item:?}\nJSON: {json_item:?}"
        );

        let json_misc = json_array("MISC_KEYWORDS");
        let rust_misc: Vec<&str> = MISC_KEYWORDS.to_vec();
        assert_eq!(
            rust_misc, json_misc,
            "MISC_KEYWORDS がドリフトしています。\nRust: {rust_misc:?}\nJSON: {json_misc:?}"
        );
    }
}
