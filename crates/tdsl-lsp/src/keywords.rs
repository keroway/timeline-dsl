//! Timeline DSL キーワード定数。
//!
//! **単一真実源は `apps/webui/src/lang-tdsl/keywords.ts`** です。
//! 本ファイルはそのミラーであり、末尾のドリフト防止テストで同期を保証します。
//!
//! keywords.ts を変更した場合は、本ファイルの定数も同じ順序・内容で更新し、
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
    "dynasty",
    "person",
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

    /// `keywords.ts` の内容を読み込み、各配列名に対応する文字列リストを返す簡易パーサ。
    ///
    /// 正規表現の代わりに行指向の単純なパーサを使う（追加依存なし）。
    /// `export const NAME = [` から始まり `]` で終わるブロックを探し、
    /// その中の `"..."` をすべて抽出する。
    fn parse_ts_keyword_array(src: &str, array_name: &str) -> Vec<String> {
        let marker = format!("export const {array_name} = [");
        let start = src
            .find(&marker)
            .unwrap_or_else(|| panic!("keywords.ts に `{marker}` が見つからない"));
        let block_start = start + marker.len();
        let block_end = src[block_start..]
            .find(']')
            .unwrap_or_else(|| panic!("keywords.ts の `{array_name}` に `]` がない"))
            + block_start;
        let block = &src[block_start..block_end];

        // `"..."` を順番に抽出
        let mut result = Vec::new();
        let mut rest = block;
        while let Some(open) = rest.find('"') {
            let after_open = &rest[open + 1..];
            let close = after_open
                .find('"')
                .unwrap_or_else(|| panic!("keywords.ts の `{array_name}` に閉じ引用符がない"));
            let kw = &after_open[..close];
            result.push(kw.to_string());
            rest = &after_open[close + 1..];
        }
        result
    }

    /// `keywords.ts` と Rust 定数のドリフト防止テスト。
    ///
    /// `apps/webui/src/lang-tdsl/keywords.ts` を読み込み、3 つの配列が
    /// Rust 定数と順序込みで完全一致することを検証する。
    #[test]
    fn keywords_match_typescript_source() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let ts_path =
            std::path::Path::new(manifest_dir).join("../../apps/webui/src/lang-tdsl/keywords.ts");
        let ts_src = std::fs::read_to_string(&ts_path).unwrap_or_else(|e| {
            panic!(
                "keywords.ts を読み込めない: {} (パス: {})",
                e,
                ts_path.display()
            )
        });

        // BLOCK_KEYWORDS の検証
        let ts_block = parse_ts_keyword_array(&ts_src, "BLOCK_KEYWORDS");
        let rust_block: Vec<&str> = BLOCK_KEYWORDS.to_vec();
        assert_eq!(
            rust_block, ts_block,
            "BLOCK_KEYWORDS がドリフトしています。\nRust: {rust_block:?}\nTS:   {ts_block:?}"
        );

        // ITEM_KEYWORDS の検証
        let ts_item = parse_ts_keyword_array(&ts_src, "ITEM_KEYWORDS");
        let rust_item: Vec<&str> = ITEM_KEYWORDS.to_vec();
        assert_eq!(
            rust_item, ts_item,
            "ITEM_KEYWORDS がドリフトしています。\nRust: {rust_item:?}\nTS:   {ts_item:?}"
        );

        // MISC_KEYWORDS の検証
        let ts_misc = parse_ts_keyword_array(&ts_src, "MISC_KEYWORDS");
        let rust_misc: Vec<&str> = MISC_KEYWORDS.to_vec();
        assert_eq!(
            rust_misc, ts_misc,
            "MISC_KEYWORDS がドリフトしています。\nRust: {rust_misc:?}\nTS:   {ts_misc:?}"
        );
    }
}
