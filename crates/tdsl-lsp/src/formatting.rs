//! `textDocument/formatting` の純粋ロジック。
//!
//! `tdsl_parser::format_source` を呼んで整形し、全文置換の [`TextEdit`] を 1 個返す。
//! パースエラー時は `None` を返す（不正なソースは整形しない）。
//! 整形前後で差分がなければ `None` を返す（クライアントは変更なしとみなす）。
//!
//! ネットワーク I/O は行わない（offline 前提・CI 安全）。

use tower_lsp::lsp_types::{Position, Range, TextEdit};

/// ソーステキストを整形し、全文置換の [`TextEdit`] を返す。
///
/// - パース失敗: `None`
/// - 整形前後で同一: `None`（差分なし）
/// - 整形成功かつ差分あり: 全文を `new_text` に置換する `TextEdit` を 1 個含む `Some(Vec<TextEdit>)`
pub fn compute_formatting(source: &str) -> Option<Vec<TextEdit>> {
    let formatted = tdsl_parser::format_source(source).ok()?;

    // 整形前後で差分がなければ空の変更なし
    if formatted == source {
        return None;
    }

    let end_position = source_end_position(source);

    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: end_position,
        },
        new_text: formatted,
    }])
}

/// ソースの末尾 `Position`（0-based 行番号、UTF-16 character 数）を計算する。
///
/// LSP では文字数は UTF-16 コードユニット単位で数えるため、
/// 最終行の各 `char` の `len_utf16()` を累積して計算する。
fn source_end_position(source: &str) -> Position {
    let mut line = 0u32;
    let mut last_line_start = 0usize;

    for (i, ch) in source.char_indices() {
        if ch == '\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }

    let last_line = &source[last_line_start..];
    let character: u32 = last_line.chars().map(|c| c.len_utf16() as u32).sum();

    Position { line, character }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 崩したインデントを持つ正当な .tdsl を整形すると全文置換 TextEdit が返ること。
    #[test]
    fn formatting_returns_text_edit_for_unformatted_source() {
        // インデントが崩れているが文法的に正当なソース
        let src = r#"timeline "T"{title "T";unit year;range 1900..2000;}"#;
        let edits =
            compute_formatting(src).expect("should return Some for valid unformatted source");
        assert_eq!(
            edits.len(),
            1,
            "exactly one TextEdit (full-document replacement)"
        );
        let edit = &edits[0];
        // Range は全文をカバー（先頭 (0,0) から末尾まで）
        assert_eq!(
            edit.range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        // new_text はパース可能でなければならない
        tdsl_parser::parse(&edit.new_text).expect("formatted output must be parseable");
    }

    /// パースエラーのソースでは None が返ること。
    #[test]
    fn formatting_returns_none_for_parse_error() {
        let src = "this is not valid tdsl !!!";
        assert!(
            compute_formatting(src).is_none(),
            "parse error source should return None"
        );
    }

    /// 整形済みソースでは差分なしで None が返ること（冪等性）。
    #[test]
    fn formatting_returns_none_for_already_formatted_source() {
        // 一度整形したソースを再整形すると差分がないはず
        let src = r#"timeline "T"{title "T";unit year;range 1900..2000;}"#;
        let formatted = tdsl_parser::format_source(src).expect("format succeeded");
        let result = compute_formatting(&formatted);
        assert!(
            result.is_none(),
            "already-formatted source should return None (no diff)"
        );
    }

    /// 返る TextEdit の Range が全文をカバーすること。
    #[test]
    fn formatting_range_covers_full_document() {
        let src = "lane \"A\" as a{}\nlane \"B\" as b{}";
        let edits = compute_formatting(src).expect("should return Some");
        let edit = &edits[0];

        // 開始は (0, 0)
        assert_eq!(edit.range.start.line, 0);
        assert_eq!(edit.range.start.character, 0);

        // 終了は最終行の末尾
        let lines: Vec<&str> = src.lines().collect();
        let last_line = lines.last().expect("has last line");
        let expected_end_line = (lines.len() - 1) as u32;
        let expected_end_char: u32 = last_line.chars().map(|c| c.len_utf16() as u32).sum();

        assert_eq!(edit.range.end.line, expected_end_line);
        assert_eq!(edit.range.end.character, expected_end_char);
    }

    /// 日本語を含むソースの末尾 Position が正しい UTF-16 character 数で計算されること。
    #[test]
    fn source_end_position_utf16_multibyte() {
        // 日本語 1 文字は UTF-16 で 1 コードユニット（BMP 範囲内）
        let src = "lane \"漢\" as han {}";
        let pos = source_end_position(src);
        assert_eq!(pos.line, 0);
        // UTF-16 での文字数をカウント
        let expected: u32 = src.chars().map(|c| c.len_utf16() as u32).sum();
        assert_eq!(pos.character, expected);
    }

    /// 末尾が改行で終わるソースの Position 計算が正しいこと。
    #[test]
    fn source_end_position_trailing_newline() {
        let src = "lane \"A\" as a {}\n";
        let pos = source_end_position(src);
        // 改行で終わるので最終行は空（line=1, character=0）
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    /// 複数行ソースの末尾 Position が最終行を正しく指すこと。
    #[test]
    fn source_end_position_multiline() {
        let src = "line1\nline2\nline3";
        let pos = source_end_position(src);
        assert_eq!(pos.line, 2);
        // "line3" は 5 文字 = UTF-16 で 5
        assert_eq!(pos.character, 5);
    }
}
