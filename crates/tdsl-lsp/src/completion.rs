//! `textDocument/completion` の純粋ロジック。
//!
//! `diagnostics.rs` の純粋関数パターンに倣い、LSP サーバ非依存・単体テスト可能な
//! 形で実装する。現バージョンは文脈非依存で全キーワードを返す MVP 実装。
//! 文脈依存補完（カーソル位置に基づく候補絞り込み）は別 issue のスコープとする。

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind};

use crate::keywords::{BLOCK_KEYWORDS, ITEM_KEYWORDS, MISC_KEYWORDS};

/// 全 DSL キーワードのコード補完候補リストを返す。
///
/// BLOCK / ITEM / MISC の 3 グループからすべてのキーワードを含む。
/// 各アイテムの `kind` は `KEYWORD`、`detail` には分類名を付与する。
/// ラベルの重複はキーワード定数側で管理（`keywords.ts` の単一真実源に従う）。
pub fn keyword_completions() -> Vec<CompletionItem> {
    let mut items = Vec::new();

    for &kw in BLOCK_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("block keyword".to_string()),
            ..Default::default()
        });
    }

    for &kw in ITEM_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("item keyword".to_string()),
            ..Default::default()
        });
    }

    for &kw in MISC_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("keyword".to_string()),
            ..Default::default()
        });
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::{BLOCK_KEYWORDS, ITEM_KEYWORDS, MISC_KEYWORDS};

    /// `keyword_completions()` が全キーワード件数を返すこと。
    #[test]
    fn keyword_completions_returns_all_keywords() {
        let items = keyword_completions();
        let expected_len = BLOCK_KEYWORDS.len() + ITEM_KEYWORDS.len() + MISC_KEYWORDS.len();
        assert_eq!(
            items.len(),
            expected_len,
            "返却アイテム数が BLOCK+ITEM+MISC の合計と一致すること"
        );
    }

    /// 全アイテムの `kind` が `KEYWORD` であること。
    #[test]
    fn all_completions_have_keyword_kind() {
        let items = keyword_completions();
        for item in &items {
            assert_eq!(
                item.kind,
                Some(CompletionItemKind::KEYWORD),
                "kind が KEYWORD でないアイテムがある: {}",
                item.label
            );
        }
    }

    /// 代表的なキーワードが補完候補に含まれること。
    #[test]
    fn completions_include_representative_keywords() {
        let items = keyword_completions();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for expected in &["timeline", "span", "import", "event_range"] {
            assert!(
                labels.contains(expected),
                "補完候補に `{expected}` が含まれていない。実際: {labels:?}"
            );
        }
    }

    /// ラベルに重複がないこと（keywords.ts の重複管理の補足検証）。
    #[test]
    fn completions_have_no_duplicate_labels() {
        let items = keyword_completions();
        let mut seen = std::collections::HashSet::new();
        for item in &items {
            assert!(
                seen.insert(item.label.as_str()),
                "補完候補にラベルの重複がある: {}",
                item.label
            );
        }
    }

    /// BLOCK キーワードの `detail` が "block keyword" であること。
    #[test]
    fn block_keywords_have_correct_detail() {
        let items = keyword_completions();
        let block_items: Vec<_> = items
            .iter()
            .filter(|i| i.detail.as_deref() == Some("block keyword"))
            .collect();
        assert_eq!(
            block_items.len(),
            BLOCK_KEYWORDS.len(),
            "block keyword detail を持つアイテム数が BLOCK_KEYWORDS の長さと一致すること"
        );
    }

    /// ITEM キーワードの `detail` が "item keyword" であること。
    #[test]
    fn item_keywords_have_correct_detail() {
        let items = keyword_completions();
        let item_items: Vec<_> = items
            .iter()
            .filter(|i| i.detail.as_deref() == Some("item keyword"))
            .collect();
        assert_eq!(
            item_items.len(),
            ITEM_KEYWORDS.len(),
            "item keyword detail を持つアイテム数が ITEM_KEYWORDS の長さと一致すること"
        );
    }
}
