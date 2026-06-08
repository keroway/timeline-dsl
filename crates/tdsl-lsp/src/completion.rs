//! `textDocument/completion` の純粋ロジック。
//!
//! `diagnostics.rs` の純粋関数パターンに倣い、LSP サーバ非依存・単体テスト可能な
//! 形で実装する。`keyword_completions()` は文脈非依存で全キーワードを返す後方互換 API。
//! `contextual_completions()` はカーソル位置のコンテキストに応じて候補を絞り込む。

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

use crate::keywords::{BLOCK_KEYWORDS, ITEM_KEYWORDS, MISC_KEYWORDS};

/// カーソル位置のコンテキストを表す enum。
///
/// `detect_context()` がテキスト解析から判定し、`contextual_completions()` が
/// このコンテキストに基づいて補完候補を絞り込む。
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// ブロック外（トップレベル）
    TopLevel,
    /// `timeline { }` 内
    Timeline,
    /// `lane { }` 内
    LaneProps,
    /// `group { }` 内
    GroupBody,
    /// `map { }` 内
    Map,
    /// `import { }` 内
    Import,
    /// `span/event/event_range { }` 内
    ItemOptions,
}

/// 全 DSL キーワードのコード補完候補リストを返す（後方互換 API）。
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

/// カーソル前テキストをトークン化し、現在のブロックコンテキストを判定する。
///
/// テキストをトークン列（単語 / `{` / `}` / `;`）に変換。
/// 文字列リテラル（`"..."`）・行コメント（`//`）・ブロックコメント（`/* */`）はスキップする。
///
/// スタックベースでネストを追跡し、スタックのトップを返す。
/// スタックが空なら `TopLevel`。
pub fn detect_context(text_before_cursor: &str) -> CompletionContext {
    // トークンを収集する（文字列・コメントはスキップ）
    let tokens = tokenize(text_before_cursor);

    // スタック: 各エントリは「このブロックに入るときに push されたコンテキスト」
    let mut stack: Vec<CompletionContext> = Vec::new();
    // 現在の深度での「文の最初のキーワード」
    let mut stmt_keyword: Vec<Option<String>> = vec![None];

    for token in &tokens {
        match token.as_str() {
            "{" => {
                // 現在の文の最初のキーワードに基づいてコンテキストを決定
                let ctx = match stmt_keyword.last().and_then(|o| o.as_deref()) {
                    Some("timeline") | Some("color_map") => CompletionContext::Timeline,
                    Some("lane") => CompletionContext::LaneProps,
                    Some("group") => CompletionContext::GroupBody,
                    Some("map") => CompletionContext::Map,
                    Some("import") => CompletionContext::Import,
                    Some("span") | Some("event") | Some("event_range") => {
                        CompletionContext::ItemOptions
                    }
                    _ => {
                        // 不明ブロック: スタックのトップを継承、なければ TopLevel
                        stack.last().cloned().unwrap_or(CompletionContext::TopLevel)
                    }
                };
                stack.push(ctx);
                stmt_keyword.push(None);
            }
            "}" => {
                stack.pop();
                stmt_keyword.pop();
                // stmt_keyword が空になることがないよう下限を保つ
                if stmt_keyword.is_empty() {
                    stmt_keyword.push(None);
                }
            }
            ";" => {
                // 文の終わり: 現在の深度のキーワードをリセット
                if let Some(last) = stmt_keyword.last_mut() {
                    *last = None;
                }
            }
            word => {
                // 現在の深度の最初のキーワードが空ならセット
                if let Some(last) = stmt_keyword.last_mut()
                    && last.is_none()
                {
                    *last = Some(word.to_string());
                }
            }
        }
    }

    stack.last().cloned().unwrap_or(CompletionContext::TopLevel)
}

/// テキストをトークン列に変換する。
///
/// - 文字列リテラル `"..."` はスキップ（中の内容はトークンにしない）
/// - 行コメント `// ...` はスキップ
/// - ブロックコメント `/* ... */` はスキップ
/// - `{`, `}`, `;` は単独トークン
/// - それ以外の連続する非区切り文字（空白・特殊記号を除く）は単語トークン
fn tokenize(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        let c = chars[i];

        // 行コメント
        if c == '/' && i + 1 < len && chars[i + 1] == '/' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // ブロックコメント
        if c == '/' && i + 1 < len && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // `*/` を消費
            }
            continue;
        }

        // 文字列リテラル
        if c == '"' {
            i += 1;
            while i < len && chars[i] != '"' {
                // エスケープ `\"` をスキップ
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1; // 閉じ `"` を消費
            }
            continue;
        }

        // 単独トークン
        if c == '{' || c == '}' || c == ';' {
            tokens.push(c.to_string());
            i += 1;
            continue;
        }

        // 空白・タブ・改行
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // 単語トークン（英数字・`_`・`@`・`.`・`(`・`)` を含む）
        // DSL では `claim(P123)`, `label@ja`, `dotted.ident` 等が出現するが
        // キーワード判定は先頭の識別子部分のみで十分なため、
        // 区切り文字（空白・`{`・`}`・`;`・`"`・`/`）以外を取り込む
        let start = i;
        while i < len
            && !chars[i].is_whitespace()
            && chars[i] != '{'
            && chars[i] != '}'
            && chars[i] != ';'
            && chars[i] != '"'
            && !(chars[i] == '/' && i + 1 < len && (chars[i + 1] == '/' || chars[i + 1] == '*'))
        {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        if !word.is_empty() {
            tokens.push(word);
        }
    }

    tokens
}

/// カーソル前テキストを行・文字位置から切り出す。
///
/// LSP の Position は 0-based（`line` 行目の `character` 文字目まで）。
/// 指定行の先頭から `character` 文字目まで + それ以前の全行を返す。
fn text_before_cursor(text: &str, line: u32, character: u32) -> String {
    let mut result = String::new();
    for (idx, l) in text.lines().enumerate() {
        if idx < line as usize {
            result.push_str(l);
            result.push('\n');
        } else if idx == line as usize {
            let char_end = character as usize;
            let slice: String = l.chars().take(char_end).collect();
            result.push_str(&slice);
            break;
        } else {
            break;
        }
    }
    result
}

/// コンテキスト依存の補完候補を返す。
///
/// カーソル位置のテキストを解析し、DSL のブロック構造から補完候補を絞り込む。
/// ドキュメントが取得できない場合は `keyword_completions()` をフォールバックとして使う。
pub fn contextual_completions(text: &str, line: u32, character: u32) -> Vec<CompletionItem> {
    let before = text_before_cursor(text, line, character);
    let ctx = detect_context(&before);

    match ctx {
        CompletionContext::TopLevel => make_keyword_items(&[
            "timeline",
            "lane",
            "group",
            "import",
            "map",
            "template",
            "apply",
            "span",
            "event",
            "event_range",
        ]),
        CompletionContext::Timeline => {
            make_keyword_items(&["unit", "range", "calendar", "color_map", "title"])
        }
        CompletionContext::LaneProps => make_keyword_items(&["kind", "order"]),
        CompletionContext::GroupBody => make_keyword_items(&["lane"]),
        CompletionContext::Map => {
            let mut items = make_keyword_items(&[
                "lane", "start", "end", "time", "label", "tags", "filter", "expand",
            ]);
            // claim() と label@ のスニペット補完を追加
            items.push(CompletionItem {
                label: "claim".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("map expression".to_string()),
                insert_text: Some("claim(${1:P123})".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
            items.push(CompletionItem {
                label: "label@".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("map expression".to_string()),
                insert_text: Some("label@${1:ja}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
            items
        }
        CompletionContext::Import => {
            make_keyword_items(&["entity", "query", "policy", "field_priority"])
        }
        CompletionContext::ItemOptions => make_keyword_items(&["tags", "source", "id", "origin"]),
    }
}

/// キーワード文字列のスライスから `KEYWORD` kind の補完候補リストを生成する。
fn make_keyword_items(keywords: &[&str]) -> Vec<CompletionItem> {
    keywords
        .iter()
        .map(|&kw| CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("keyword".to_string()),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keywords::{BLOCK_KEYWORDS, ITEM_KEYWORDS, MISC_KEYWORDS};

    // ──────────────────────────────────────────────
    // 既存テスト（後方互換）
    // ──────────────────────────────────────────────

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

    // ──────────────────────────────────────────────
    // detect_context テスト
    // ──────────────────────────────────────────────

    /// ブロック外（空テキスト）は `TopLevel`。
    #[test]
    fn detect_context_top_level() {
        assert_eq!(detect_context(""), CompletionContext::TopLevel);
        assert_eq!(
            detect_context("timeline \"foo\" {}\n"),
            CompletionContext::TopLevel
        );
    }

    /// `timeline { }` 内は `Timeline`。
    #[test]
    fn detect_context_timeline() {
        let text = r#"timeline "My Timeline" {
  unit year
  "#;
        assert_eq!(detect_context(text), CompletionContext::Timeline);
    }

    /// `lane { }` 内は `LaneProps`。
    #[test]
    fn detect_context_lane_props() {
        let text = r#"lane "Dynasty" as dynasty {
  kind "#;
        assert_eq!(detect_context(text), CompletionContext::LaneProps);
    }

    /// `map { }` 内は `Map`。
    #[test]
    fn detect_context_map() {
        let text = r#"map wd.Q7209 to span {
  lane "#;
        assert_eq!(detect_context(text), CompletionContext::Map);
    }

    /// `import { }` 内は `Import`。
    #[test]
    fn detect_context_import() {
        let text = r#"import wikidata as wd {
  entity "#;
        assert_eq!(detect_context(text), CompletionContext::Import);
    }

    /// `span { }` 内は `ItemOptions`。
    #[test]
    fn detect_context_item_options() {
        let text = r#"span foo 100 .. 200 "Label" {
  "#;
        assert_eq!(detect_context(text), CompletionContext::ItemOptions);
    }

    /// `group { }` 内は `GroupBody`。
    #[test]
    fn detect_context_group_body() {
        let text = r#"group "Periods" {
  "#;
        assert_eq!(detect_context(text), CompletionContext::GroupBody);
    }

    /// ネストしたブロックを正しく追跡する。
    /// `timeline { }` が閉じた後のトップレベルは再び `TopLevel`。
    #[test]
    fn detect_context_nested_blocks_pop_correctly() {
        let closed = r#"timeline "T" {
  unit year
}
"#;
        assert_eq!(detect_context(closed), CompletionContext::TopLevel);

        // timeline の中のさらに color_map ブロックも Timeline として扱う
        let color_map_inside = r#"timeline "T" {
  color_map {
  "#;
        assert_eq!(
            detect_context(color_map_inside),
            CompletionContext::Timeline
        );
    }

    /// 行コメントはトークンとして無視される。
    #[test]
    fn detect_context_ignores_line_comments() {
        let text = "// timeline \"ignored\" {\nmap foo to span {\n  ";
        assert_eq!(detect_context(text), CompletionContext::Map);
    }

    /// ブロックコメントはトークンとして無視される。
    #[test]
    fn detect_context_ignores_block_comments() {
        let text = "/* import wikidata as wd { */ timeline \"T\" {\n  ";
        assert_eq!(detect_context(text), CompletionContext::Timeline);
    }

    /// 文字列リテラル内のキーワードはトークンとして無視される。
    #[test]
    fn detect_context_ignores_string_literals() {
        // "map" は文字列リテラルの中にあるのでコンテキストに影響しない
        let text = "timeline \"map\" {\n  ";
        assert_eq!(detect_context(text), CompletionContext::Timeline);
    }

    // ──────────────────────────────────────────────
    // contextual_completions テスト
    // ──────────────────────────────────────────────

    /// timeline 内の補完: unit/range/calendar が含まれ、span が含まれない。
    #[test]
    fn contextual_completions_timeline() {
        let src = "timeline \"T\" {\n  unit year\n  ";
        // カーソルは 2 行目末尾（0-based: line=2, character=2）
        let items = contextual_completions(src, 2, 2);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        assert!(
            labels.contains(&"unit"),
            "timeline 内に `unit` が含まれるべき: {labels:?}"
        );
        assert!(
            labels.contains(&"range"),
            "timeline 内に `range` が含まれるべき: {labels:?}"
        );
        assert!(
            labels.contains(&"calendar"),
            "timeline 内に `calendar` が含まれるべき: {labels:?}"
        );
        assert!(
            !labels.contains(&"span"),
            "timeline 内に `span` が含まれてはいけない: {labels:?}"
        );
    }

    /// map 内の補完: lane/start/end/time/label が含まれ、unit が含まれない。claim スニペットが含まれる。
    #[test]
    fn contextual_completions_map() {
        let src = "map wd.Q7209 to span {\n  ";
        let items = contextual_completions(src, 1, 2);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for expected in &["lane", "start", "end", "time", "label"] {
            assert!(
                labels.contains(expected),
                "map 内に `{expected}` が含まれるべき: {labels:?}"
            );
        }
        assert!(
            !labels.contains(&"unit"),
            "map 内に `unit` が含まれてはいけない: {labels:?}"
        );
        // claim スニペットが含まれること
        assert!(
            labels.contains(&"claim"),
            "map 内に `claim` スニペットが含まれるべき: {labels:?}"
        );
        // claim の kind が SNIPPET であること
        let claim_item = items.iter().find(|i| i.label == "claim").unwrap();
        assert_eq!(claim_item.kind, Some(CompletionItemKind::SNIPPET));
    }

    /// トップレベルの補完: span/event/event_range が含まれる。
    #[test]
    fn contextual_completions_top_level() {
        let src = "";
        let items = contextual_completions(src, 0, 0);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for expected in &["span", "event", "event_range", "timeline", "lane"] {
            assert!(
                labels.contains(expected),
                "トップレベルに `{expected}` が含まれるべき: {labels:?}"
            );
        }
    }

    /// lane props 内の補完: kind/order が含まれ、timeline が含まれない。
    #[test]
    fn contextual_completions_lane_props() {
        let src = "lane \"Dynasty\" as dynasty {\n  ";
        let items = contextual_completions(src, 1, 2);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        assert!(
            labels.contains(&"kind"),
            "lane 内に `kind` が含まれるべき: {labels:?}"
        );
        assert!(
            labels.contains(&"order"),
            "lane 内に `order` が含まれるべき: {labels:?}"
        );
        assert!(
            !labels.contains(&"timeline"),
            "lane 内に `timeline` が含まれてはいけない: {labels:?}"
        );
    }

    /// import 内の補完: entity/query/policy が含まれる。
    #[test]
    fn contextual_completions_import() {
        let src = "import wikidata as wd {\n  ";
        let items = contextual_completions(src, 1, 2);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for expected in &["entity", "query", "policy"] {
            assert!(
                labels.contains(expected),
                "import 内に `{expected}` が含まれるべき: {labels:?}"
            );
        }
    }

    /// span ブロック内（ItemOptions）の補完: tags/source/id/origin が含まれる。
    #[test]
    fn contextual_completions_item_options() {
        let src = "span foo 100 .. 200 \"Label\" {\n  ";
        let items = contextual_completions(src, 1, 2);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

        for expected in &["tags", "source", "id", "origin"] {
            assert!(
                labels.contains(expected),
                "span ブロック内に `{expected}` が含まれるべき: {labels:?}"
            );
        }
    }

    /// label@ スニペットが map 内に含まれ、kind が SNIPPET であること。
    #[test]
    fn contextual_completions_map_label_snippet() {
        let src = "map wd.Q7209 to span {\n  ";
        let items = contextual_completions(src, 1, 2);

        let label_item = items.iter().find(|i| i.label == "label@");
        assert!(
            label_item.is_some(),
            "map 内に `label@` スニペットが含まれるべき"
        );
        let label_item = label_item.unwrap();
        assert_eq!(label_item.kind, Some(CompletionItemKind::SNIPPET));
        assert_eq!(
            label_item.insert_text.as_deref(),
            Some("label@${1:ja}"),
            "label@ の insertText が `label@${{1:ja}}` であること"
        );
    }
}
