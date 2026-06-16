//! `textDocument/hover` の純粋ロジック。
//!
//! `diagnostics.rs` / `completion.rs` の純粋関数パターンに倣い、
//! LSP サーバ非依存・単体テスト可能な形で実装する。
//!
//! - lane ID にカーソルを当てると、その lane のラベル・kind・order を表示する。
//! - QID（`Q[0-9]+`）にカーソルを当てると、キャッシュ済みエンティティ情報を表示する。
//! - ネットワーク I/O は行わない（offline 前提・CI 安全）。

use tdsl_wikidata::WikidataEntity;
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// QID かどうかを判定する（`^Q[0-9]+$`）。
/// regex クレートを使わず手書き判定でコンパイル時間・依存を抑える。
fn is_qid(word: &str) -> bool {
    let mut chars = word.chars();
    match chars.next() {
        Some('Q') => {}
        _ => return false,
    }
    let rest: String = chars.collect();
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// LSP `Position`（0-based 行, UTF-16 character）の位置にあるトークン（identifier / QID）を
/// 抽出し、(トークン文字列, そのトークンの LSP Range) を返す。トークンが無ければ None。
///
/// `position.character` は UTF-16 コードユニット数であるため、日本語文字を含む行でも
/// 正しくバイト境界を特定できるよう各 `char` の `len_utf16()` を累積して変換する。
pub(crate) fn word_at_position(source: &str, position: Position) -> Option<(String, Range)> {
    let line_str = source.lines().nth(position.line as usize)?;

    // UTF-16 character オフセット → バイトオフセットに変換
    let target_utf16 = position.character as usize;
    let cursor_byte = utf16_offset_to_byte(line_str, target_utf16)?;

    // トークン文字: [A-Za-z0-9_]
    let is_token_char = |c: char| c.is_ascii_alphanumeric() || c == '_';

    // カーソル位置がトークン文字上でなければ None
    let cursor_char = line_str[cursor_byte..].chars().next()?;
    if !is_token_char(cursor_char) {
        return None;
    }

    // トークン左境界（バイト）: cursor_byte から左に拡張
    let left_bytes = &line_str[..cursor_byte];
    let token_start_byte = left_bytes
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_token_char(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(cursor_byte);

    // トークン右境界（バイト）: cursor_byte から右に拡張
    let token_end_byte = line_str[cursor_byte..]
        .char_indices()
        .take_while(|(_, c)| is_token_char(*c))
        .last()
        .map(|(i, c)| cursor_byte + i + c.len_utf8())
        .unwrap_or(cursor_byte + cursor_char.len_utf8());

    let word = line_str[token_start_byte..token_end_byte].to_string();
    if word.is_empty() {
        return None;
    }

    // バイトオフセット → UTF-16 character オフセットに戻す
    let start_utf16 = byte_offset_to_utf16(line_str, token_start_byte);
    let end_utf16 = byte_offset_to_utf16(line_str, token_end_byte);

    let range = Range {
        start: Position {
            line: position.line,
            character: start_utf16 as u32,
        },
        end: Position {
            line: position.line,
            character: end_utf16 as u32,
        },
    };

    Some((word, range))
}

/// UTF-16 character オフセット → バイトオフセット変換。
/// 範囲外の場合は None。
pub(crate) fn utf16_offset_to_byte(s: &str, utf16_offset: usize) -> Option<usize> {
    let mut utf16_count = 0usize;
    for (byte_pos, ch) in s.char_indices() {
        if utf16_count >= utf16_offset {
            return Some(byte_pos);
        }
        utf16_count += ch.len_utf16();
    }
    // cursor が行末（1つ過ぎた位置）の場合
    if utf16_count == utf16_offset {
        Some(s.len())
    } else {
        None
    }
}

/// バイトオフセット → UTF-16 character オフセット変換。
pub(crate) fn byte_offset_to_utf16(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset.min(s.len())]
        .chars()
        .map(|c| c.len_utf16())
        .sum()
}

/// 単一の lane ID トークンに対する hover マークダウン本文を返す。
/// source をパース・静的 lowering して lanes を引き、id 一致する lane があれば
/// ラベル・kind・order を整形して返す。一致しなければ None。
fn lane_hover_markdown(source: &str, word: &str) -> Option<String> {
    let file = tdsl_parser::parse(source).ok()?;
    let ir = tdsl_core::lower::lower_static_with_source(&file, Some(source)).ok()?;
    let lane = ir.lanes.iter().find(|l| l.id == word)?;
    Some(format!(
        "**lane** `{id}`\n\n- label: {label}\n- kind: {kind}\n- order: {order}",
        id = lane.id,
        label = lane.label,
        kind = lane.kind,
        order = lane.order,
    ))
}

/// QID トークンに対する hover マークダウン本文を返す。
///
/// `entity` はキャッシュ読み出し結果（呼び出し側が注入＝テスト可能にするため引数化）。
/// - `entity` が `Some` なら label と主要 claim（P569/P570/P571/P576）を表示する。
/// - `entity` が `None` なら「キャッシュ未取得」の旨を添える。
fn qid_hover_markdown(qid: &str, entity: Option<&WikidataEntity>) -> String {
    let url = format!("https://www.wikidata.org/wiki/{qid}");
    let mut md = format!("**{qid}** — [Wikidata]({url})\n\n");

    match entity {
        None => {
            md.push_str("_キャッシュ未取得（`tdsl build` または `tdsl render` でオンラインビルドすると取得可能）_");
        }
        Some(e) => {
            if let Some(label) = e.label_with_fallback(&["ja", "en"]) {
                md.push_str(&format!("**{label}**\n\n"));
            }
            // 主要 claim の年情報を補足表示
            let claim_labels = [
                ("P569", "誕生"),
                ("P570", "死亡"),
                ("P571", "成立"),
                ("P576", "消滅"),
                ("P580", "開始"),
                ("P582", "終了"),
            ];
            let mut has_claims = false;
            for (prop, label) in &claim_labels {
                if let Some(year) = extract_claim_year(e, prop) {
                    if !has_claims {
                        has_claims = true;
                    }
                    md.push_str(&format!("- {label}: {year}\n"));
                }
            }
            if !has_claims {
                md.push_str("_（claim 情報なし）_");
            }
        }
    }

    md
}

/// エンティティの指定プロパティから year を抽出するヘルパー。
fn extract_claim_year(entity: &WikidataEntity, property: &str) -> Option<i64> {
    match entity.claim(property) {
        Some(tdsl_wikidata::entity::DataValue::Time { value }) => {
            tdsl_wikidata::entity::time_value_to_year(value).ok()
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// 公開インタフェース
// ---------------------------------------------------------------------------

/// hover 要求を処理して LSP Hover を返す（内部関数・テスト可能版）。
///
/// `lookup` クロージャでキャッシュ読み出しを注入するため、
/// 実ファイルシステム・ネットワーク非依存で単体テスト可能。
pub fn compute_hover_with<F>(source: &str, position: Position, lookup: F) -> Option<Hover>
where
    F: Fn(&str) -> Option<WikidataEntity>,
{
    let (word, word_range) = word_at_position(source, position)?;

    // まず lane として解決を試みる
    if let Some(md) = lane_hover_markdown(source, &word) {
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: Some(word_range),
        });
    }

    // QID 判定
    if is_qid(&word) {
        let entity = lookup(&word);
        let md = qid_hover_markdown(&word, entity.as_ref());
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: Some(word_range),
        });
    }

    None
}

/// hover 要求を処理して LSP Hover を返す。
///
/// lane ID → lane 情報、QID → エンティティ情報（キャッシュ）。該当なしは None。
/// キャッシュ読み出しには `tdsl_wikidata::default_cache_dir()` を使う。
pub fn compute_hover(source: &str, position: Position) -> Option<Hover> {
    compute_hover_with(source, position, |qid| {
        tdsl_wikidata::read_cached_entity(&tdsl_wikidata::default_cache_dir(), qid)
    })
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tdsl_wikidata::entity::{LabelValue, TimeValue};

    // --- word_at_position ---

    /// ASCII のみの行でカーソルがトークン上にある場合
    #[test]
    fn word_at_position_ascii_basic() {
        let src = "span foo 100..200 \"bar\" {};";
        // "foo" は 5..8 の位置（0-indexed byte）
        // character=5 (0-based UTF-16) = 'f' の位置
        let pos = Position {
            line: 0,
            character: 5,
        };
        let result = word_at_position(src, pos);
        assert!(result.is_some(), "トークン 'foo' が見つかるべき");
        let (word, range) = result.unwrap();
        assert_eq!(word, "foo");
        assert_eq!(range.start.character, 5);
        assert_eq!(range.end.character, 8);
    }

    /// ASCII のみの行でカーソルがスペース上（トークン外）にある場合
    #[test]
    fn word_at_position_ascii_on_space() {
        let src = "span foo 100..200 \"bar\" {};";
        // character=4 はスペース
        let pos = Position {
            line: 0,
            character: 4,
        };
        let result = word_at_position(src, pos);
        assert!(result.is_none(), "スペース上では None を返す");
    }

    /// 日本語ラベルを含む行で ASCII トークンを正しく抽出できること（UTF-16 対応）
    #[test]
    fn word_at_position_after_japanese_label() {
        // `lane "漢" as han {` の行。
        // "漢" は U+6F22（len_utf16=1, len_utf8=3）。
        // 行 = lane_0 "漢" as han {
        //  byte: 0123456789...
        //  UTF-16 chars: l=1,a=1,n=1,e=1,' '=1,'"'=1,'漢'=1,'"'=1,' '=1,'a'=1,'s'=1,' '=1,'h'=1,'a'=1,'n'=1
        // "漢" の UTF-16 offset=6 (len_utf16=1), "as" = 8,9、"han" starts at UTF-16 char 12
        let src = "lane \"漢\" as han {";
        // "han" の開始位置:
        // l=0,a=1,n=2,e=3,' '=4,'"'=5,'漢'=6(len_utf16=1),'"'=7,' '=8,'a'=9,'s'=10,' '=11,'h'=12,'a'=13,'n'=14
        let pos = Position {
            line: 0,
            character: 12,
        };
        let result = word_at_position(src, pos);
        assert!(result.is_some(), "'han' が見つかるべき。result={result:?}");
        let (word, _) = result.unwrap();
        assert_eq!(word, "han");
    }

    /// トークン中央にカーソルを置いても同じトークン全体を返す
    #[test]
    fn word_at_position_cursor_in_middle_of_token() {
        let src = "span Q7209_foo 100..200 \"bar\" {};";
        // "Q7209_foo" の中央 'f' (byte=11, UTF-16=11)
        let pos = Position {
            line: 0,
            character: 11,
        };
        let result = word_at_position(src, pos);
        assert!(result.is_some());
        let (word, _) = result.unwrap();
        assert_eq!(word, "Q7209_foo");
    }

    /// 存在しない行番号では None
    #[test]
    fn word_at_position_invalid_line() {
        let src = "span foo 100..200 \"bar\" {};";
        let pos = Position {
            line: 99,
            character: 0,
        };
        let result = word_at_position(src, pos);
        assert!(result.is_none());
    }

    // --- is_qid ---

    #[test]
    fn is_qid_valid() {
        assert!(is_qid("Q7209"));
        assert!(is_qid("Q1"));
        assert!(is_qid("Q42"));
    }

    #[test]
    fn is_qid_invalid() {
        assert!(!is_qid("q42")); // 小文字
        assert!(!is_qid("Q")); // 数字なし
        assert!(!is_qid("Q42a")); // 末尾に非数字
        assert!(!is_qid(""));
        assert!(!is_qid("foo"));
    }

    // --- lane_hover_markdown ---

    /// lane が存在する場合は markdown を返す
    #[test]
    fn lane_hover_markdown_found() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "漢" as han { kind dynasty; order 10; }
span han 100..200 "foo" {};
"#;
        let result = lane_hover_markdown(src, "han");
        assert!(result.is_some(), "lane 'han' が存在するので Some を返す");
        let md = result.unwrap();
        assert!(md.contains("han"), "lane id を含む");
        assert!(md.contains("漢"), "label を含む");
        assert!(md.contains("dynasty"), "kind を含む");
        assert!(md.contains("10"), "order を含む");
    }

    /// lane が存在しない場合は None
    #[test]
    fn lane_hover_markdown_not_found() {
        let src = r#"
timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "foo" as foo { kind custom; order 1; }
"#;
        let result = lane_hover_markdown(src, "nonexistent");
        assert!(result.is_none(), "存在しない lane ID は None を返す");
    }

    /// パースエラーがある場合は None
    #[test]
    fn lane_hover_markdown_parse_error_returns_none() {
        let src = "timeline @@@invalid";
        let result = lane_hover_markdown(src, "foo");
        assert!(result.is_none(), "パースエラーは None を返す");
    }

    // --- qid_hover_markdown ---

    /// entity が None の場合はキャッシュ未取得メッセージを含む
    #[test]
    fn qid_hover_markdown_no_entity() {
        let md = qid_hover_markdown("Q42", None);
        assert!(md.contains("Q42"), "QID を含む");
        assert!(md.contains("wikidata.org"), "Wikidata URL を含む: {md}");
        assert!(
            md.contains("キャッシュ未取得") || md.contains("tdsl fetch"),
            "未取得の旨を含む: {md}"
        );
    }

    /// entity が Some の場合はラベルと URL を含む
    #[test]
    fn qid_hover_markdown_with_entity_label() {
        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "ダグラス・アダムス".to_string(),
            },
        );
        let entity = WikidataEntity {
            id: "Q42".to_string(),
            labels,
            claims: HashMap::new(),
        };
        let md = qid_hover_markdown("Q42", Some(&entity));
        assert!(md.contains("Q42"), "QID を含む");
        assert!(md.contains("wikidata.org"), "Wikidata URL を含む");
        assert!(md.contains("ダグラス・アダムス"), "ラベルを含む");
    }

    /// entity が Some で誕生年 claim がある場合
    #[test]
    fn qid_hover_markdown_with_birth_year_claim() {
        use tdsl_wikidata::entity::{DataValue, Snak, Statement};
        let mut claims = HashMap::new();
        claims.insert(
            "P569".to_string(),
            vec![Statement {
                mainsnak: Snak {
                    snaktype: "value".to_string(),
                    property: "P569".to_string(),
                    datavalue: Some(DataValue::Time {
                        value: TimeValue {
                            time: "+1952-03-11T00:00:00Z".to_string(),
                            precision: 11,
                            calendarmodel: String::new(),
                        },
                    }),
                },
                rank: "normal".to_string(),
                qualifiers: HashMap::new(),
            }],
        );
        let entity = WikidataEntity {
            id: "Q42".to_string(),
            labels: HashMap::new(),
            claims,
        };
        let md = qid_hover_markdown("Q42", Some(&entity));
        assert!(md.contains("1952"), "誕生年を含む: {md}");
        assert!(md.contains("誕生"), "誕生ラベルを含む: {md}");
    }

    // --- compute_hover_with ---

    /// lane hover: ソースに lane が定義されており、lane ID の位置でカーソルを当てると hover を返す
    #[test]
    fn compute_hover_with_lane() {
        let src = r#"timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "漢" as han { kind dynasty; order 10; }
span han 100..200 "foo" {};
"#;
        // 2行目（0-based: 1）の "han" は `lane "漢" as han { ...` の position 15 あたり
        // l=0,a=1,n=2,e=3,' '=4,'"'=5,'漢'=6(utf-16=1),'"'=7,' '=8,'a'=9,'s'=10,' '=11,'h'=12,'a'=13,'n'=14
        let pos = Position {
            line: 1,
            character: 12,
        };
        let result = compute_hover_with(src, pos, |_| None);
        assert!(result.is_some(), "lane hover が返るべき: {pos:?}");
        let hover = result.unwrap();
        if let HoverContents::Markup(mc) = hover.contents {
            assert!(mc.value.contains("漢"), "ラベル '漢' を含む: {}", mc.value);
            assert!(mc.value.contains("dynasty"), "kind を含む: {}", mc.value);
        } else {
            panic!("MarkupContent でない");
        }
    }

    /// QID hover: キャッシュにエンティティがある場合
    #[test]
    fn compute_hover_with_qid_cached() {
        // 行 "entity Q7209 as han_dynasty;" の Q7209 位置
        let src = r#"timeline "test" { title "test"; unit year; range -500..2000; calendar proleptic_gregorian; }
lane "test" as tl { kind custom; order 1; }
import wikidata as wd {
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}
"#;
        // 4行目（0-based: 3）の Q7209: "    entity Q7209..."
        // '    entity ' = 11 chars → Q=11, 7=12, 2=13, 0=14, 9=15
        let pos = Position {
            line: 3,
            character: 11,
        };

        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "漢".to_string(),
            },
        );
        let mock_entity = WikidataEntity {
            id: "Q7209".to_string(),
            labels,
            claims: HashMap::new(),
        };

        let result = compute_hover_with(src, pos, |qid| {
            if qid == "Q7209" {
                Some(mock_entity.clone())
            } else {
                None
            }
        });
        assert!(result.is_some(), "QID hover が返るべき");
        let hover = result.unwrap();
        if let HoverContents::Markup(mc) = hover.contents {
            assert!(mc.value.contains("Q7209"), "QID を含む: {}", mc.value);
            assert!(mc.value.contains("漢"), "ラベルを含む: {}", mc.value);
        } else {
            panic!("MarkupContent でない");
        }
    }

    /// QID hover: キャッシュ未取得の場合
    #[test]
    fn compute_hover_with_qid_not_cached() {
        let src = r#"timeline "test" { title "test"; unit year; range -500..2000; calendar proleptic_gregorian; }
lane "test" as tl { kind custom; order 1; }
import wikidata as wd {
    entity Q7209 as han_dynasty;
    policy merge_by_source;
}
"#;
        let pos = Position {
            line: 3,
            character: 11,
        };
        let result = compute_hover_with(src, pos, |_| None);
        assert!(
            result.is_some(),
            "キャッシュ未取得でも hover を返す（未取得メッセージ）"
        );
        let hover = result.unwrap();
        if let HoverContents::Markup(mc) = hover.contents {
            assert!(
                mc.value.contains("キャッシュ未取得") || mc.value.contains("tdsl fetch"),
                "未取得メッセージを含む: {}",
                mc.value
            );
        } else {
            panic!("MarkupContent でない");
        }
    }

    /// 該当なし: 識別子でないトークン位置では None
    #[test]
    fn compute_hover_with_no_match() {
        let src = r#"timeline "test" { title "test"; unit year; range 0..2000; calendar proleptic_gregorian; }
lane "foo" as foo { kind custom; order 1; }
"#;
        // 最初の行の '{' の位置（非トークン文字）
        let pos = Position {
            line: 0,
            character: 16,
        };
        let result = compute_hover_with(src, pos, |_| None);
        assert!(result.is_none(), "非トークン位置は None を返す");
    }
}
