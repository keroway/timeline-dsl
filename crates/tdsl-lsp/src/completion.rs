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
    /// `apply <template> to <import> { }` 内。
    ///
    /// `grammar.pest` の `apply_override` は `lane` のみを受理するため、
    /// `map` と同じ候補を出すと通らないキーワードを勧めることになる（#753）。
    ApplyBody,
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
                    // `template` の中身は `map_prop` と同一（grammar.pest）。
                    // ここが抜けていたため、template ブロック内で
                    // TopLevel 補完（timeline, lane, …）が出ていた（#753）。
                    Some("template") => CompletionContext::Map,
                    Some("apply") => CompletionContext::ApplyBody,
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

/// ソースから宣言済みの lane ID を集める（パース不要の軽量スキャン）。
///
/// 補完は**編集途中の壊れたソース**に対しても呼ばれるため、パーサに通さない。
/// 途中まで書かれた `lane "A" as a {` でも `a` を拾えることが要件。
fn declared_lane_ids(text: &str) -> Vec<String> {
    scan_declarations(text, "lane")
}

/// import alias（`import <src> as <alias>`）を集める。
fn declared_import_aliases(text: &str) -> Vec<String> {
    scan_declarations(text, "import")
}

/// template ID（`template "..." as <id>`）を集める。
fn declared_template_ids(text: &str) -> Vec<String> {
    scan_declarations(text, "template")
}

/// `<keyword> ... as <ident>` の `<ident>` を行単位で集める。
///
/// `as` を省略した宣言（`lane "A" { }` / `import Q7209 { }`）は ID が
/// ラベルや source 名から導出されるため、ここでは拾わない。**推測で
/// 候補を出すより、確実なものだけを出す**（間違った候補は打ち間違いを誘う）。
fn scan_declarations(text: &str, keyword: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        // **先頭トークンの完全一致で判定する。** `starts_with` だと
        // `lanes are documented as invalid` のような非宣言行も拾い、
        // `invalid` を候補に出してしまう（CodeRabbit の指摘）。
        if parts.next() != Some(keyword) {
            continue;
        }
        while let Some(tok) = parts.next() {
            if tok == "as"
                && let Some(id) = parts.next()
            {
                let id = id.trim_end_matches(['{', '}', ';']);
                if !id.is_empty() && !out.iter().any(|e| e == id) {
                    out.push(id.to_string());
                }
                break;
            }
        }
    }
    out
}

/// 宣言済み ID を `VALUE` kind の補完候補にする。
fn make_value_items(ids: &[String], detail: &str) -> Vec<CompletionItem> {
    ids.iter()
        .map(|id| CompletionItem {
            label: id.clone(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

/// 「値を書く位置」なら候補を返す。キーワード補完より優先する。
///
/// 対象は 3 つ:
///
/// - `span` / `event` / `event_range` の直後 → 宣言済み lane ID
/// - `apply <template> to ` の直後 → import alias（その手前は template ID）
/// - `map ` の直後 → import alias（`alias.key` の alias 部分）
///
/// **offline のみ。** entity key の補完は Wikidata 取得が要るのでここでは扱わない
/// （補完のために暗黙にネットワークへ出ない）。
fn value_completions(text: &str, before: &str) -> Option<Vec<CompletionItem>> {
    // 現在行の、カーソルまでの部分だけを見る。
    let current_line = before.rsplit('\n').next().unwrap_or("");
    let trimmed = current_line.trim_start();

    // 末尾が空白のときだけ「次のトークンを書き始める位置」とみなす。
    // `span l` の途中（`l` を入力中）は既存のキーワード補完に任せる。
    let at_new_token = current_line.ends_with(char::is_whitespace);
    if !at_new_token {
        return None;
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    match words.as_slice() {
        // `span ` / `event ` / `event_range ` の直後は lane 参照。
        ["span"] | ["event"] | ["event_range"] => {
            Some(make_value_items(&declared_lane_ids(text), "lane"))
        }
        // `apply ` の直後は template、`apply <t> to ` の直後は import。
        ["apply"] => Some(make_value_items(&declared_template_ids(text), "template")),
        ["apply", _, "to"] => Some(make_value_items(&declared_import_aliases(text), "import")),
        // `map ` の直後は import alias。
        ["map"] => Some(make_value_items(&declared_import_aliases(text), "import")),
        _ => None,
    }
}

/// コンテキスト依存の補完候補を返す。
///
/// カーソル位置のテキストを解析し、DSL のブロック構造から補完候補を絞り込む。
/// ドキュメントが取得できない場合は `keyword_completions()` をフォールバックとして使う。
pub fn contextual_completions(text: &str, line: u32, character: u32) -> Vec<CompletionItem> {
    let before = text_before_cursor(text, line, character);

    // 値の位置（キーワードではなく ID を書く場所）を先に判定する。
    // ここでキーワード候補を出すと、`span ` の直後に `timeline` などが
    // 並んで邪魔になる（#753）。
    if let Some(items) = value_completions(text, &before) {
        return items;
    }

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
        CompletionContext::LaneProps => make_keyword_items(&["kind", "order", "color"]),
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
        // `note` / `link` / `color` は文法・IR・ハイライト（keywords.json）に
        // 揃っているのに補完だけ欠けていた（#753）。
        CompletionContext::ItemOptions => {
            make_keyword_items(&["tags", "source", "id", "origin", "note", "link", "color"])
        }
        CompletionContext::ApplyBody => make_keyword_items(&["lane"]),
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

#[cfg(test)]
mod completion_context_tests {
    use super::*;

    fn labels(items: &[CompletionItem]) -> Vec<String> {
        items.iter().map(|i| i.label.clone()).collect()
    }

    /// カーソルをテキスト末尾に置いて補完を取る。
    fn complete(text: &str) -> Vec<CompletionItem> {
        let line = text.lines().count().saturating_sub(1) as u32;
        let col = text.lines().last().map(str::len).unwrap_or(0) as u32;
        contextual_completions(text, line, col)
    }

    // ─── template / apply の文脈（#753）──────────────────────────────────

    /// `template` の中身は `map_prop` と同一なので Map 候補を出す。
    /// 以前はここが抜けており、TopLevel 候補（timeline, lane, …）が出ていた。
    #[test]
    fn template_body_offers_map_completions() {
        let text = "template \"tpl\" as t to span {\n  ";
        let got = labels(&complete(text));
        assert!(got.iter().any(|l| l == "start"), "got: {got:?}");
        assert!(
            !got.iter().any(|l| l == "timeline"),
            "TopLevel 候補が出ている: {got:?}"
        );
    }

    /// `apply` の中身は `lane` のみ（grammar.pest の `apply_override`）。
    /// Map 候補を出すと通らないキーワードを勧めることになる。
    #[test]
    fn apply_body_offers_only_lane() {
        let text = "apply t to wd {\n  ";
        let got = labels(&complete(text));
        assert_eq!(got, vec!["lane".to_string()], "got: {got:?}");
    }

    // ─── item オプションの欠落（#753）────────────────────────────────────

    /// `note` / `link` / `color` は文法・IR・ハイライトに揃っているのに
    /// 補完だけ欠けていた。
    #[test]
    fn item_options_include_note_link_color() {
        let text = "span l 2001..2002 \"S\" {\n  ";
        let got = labels(&complete(text));
        for expected in ["tags", "source", "id", "origin", "note", "link", "color"] {
            assert!(
                got.iter().any(|l| l == expected),
                "{expected} が無い: {got:?}"
            );
        }
    }

    /// lane の `color`（#747 で追加）も補完に出る。
    #[test]
    fn lane_props_include_color() {
        let text = "lane \"L\" as l {\n  ";
        let got = labels(&complete(text));
        assert!(got.iter().any(|l| l == "color"), "got: {got:?}");
    }

    // ─── 値の補完（#753 提案 3）─────────────────────────────────────────

    #[test]
    fn span_offers_declared_lane_ids() {
        let text =
            "lane \"A\" as alpha { kind custom; }\nlane \"B\" as beta { kind custom; }\nspan ";
        let got = labels(&complete(text));
        assert!(
            got.iter().any(|l| l == "alpha") && got.iter().any(|l| l == "beta"),
            "got: {got:?}"
        );
        assert!(
            !got.iter().any(|l| l == "timeline"),
            "キーワードが混ざっている: {got:?}"
        );
    }

    #[test]
    fn apply_offers_template_then_import() {
        let text = "template \"T\" as tpl to span { lane a; }\nimport Q7209 as wd { entity Q7209 as han; }\napply ";
        assert!(labels(&complete(text)).iter().any(|l| l == "tpl"));

        let text2 = format!("{text}tpl to ");
        assert!(labels(&complete(&text2)).iter().any(|l| l == "wd"));
    }

    #[test]
    fn map_offers_import_aliases() {
        let text = "import Q7209 as wd { entity Q7209 as han; }\nmap ";
        assert!(labels(&complete(text)).iter().any(|l| l == "wd"));
    }

    /// **入力途中はキーワード補完に任せる。** `span al` のように書きかけの
    /// 位置で値候補だけを返すと、フィルタが効かず使いにくい。
    #[test]
    fn partial_token_falls_back_to_keyword_completion() {
        let text = "lane \"A\" as alpha { kind custom; }\nspan al";
        let got = labels(&complete(text));
        assert!(
            !got.iter().any(|l| l == "alpha"),
            "書きかけで値候補を出している: {got:?}"
        );
    }

    /// **先頭トークンが完全一致する行だけを宣言とみなす。**
    /// `starts_with` で判定すると `lanes are documented as invalid` のような
    /// 非宣言行も拾い、無効な ID を候補に出してしまう。
    #[test]
    fn non_declaration_lines_are_not_scanned() {
        let text = "lanes are documented as invalid\nlane \"A\" as alpha { kind custom; }\nspan ";
        let got = labels(&complete(text));
        assert!(
            !got.iter().any(|l| l == "invalid"),
            "非宣言行から候補を拾っている: {got:?}"
        );
        assert!(got.iter().any(|l| l == "alpha"), "got: {got:?}");
    }

    /// `as` を省略した宣言は ID を推測しない（間違った候補は打ち間違いを誘う）。
    #[test]
    fn declarations_without_as_are_not_guessed() {
        let text = "lane \"日本\" { kind custom; }\nspan ";
        let got = labels(&complete(text));
        assert!(got.is_empty(), "推測した候補が出ている: {got:?}");
    }
}
