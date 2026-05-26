//! `textDocument/definition` の純粋ロジック。
//!
//! - lane ID にカーソルを当てると、その lane の宣言位置にジャンプする。
//! - 未定義の lane 参照ではジャンプ先なし（`None` を返す）。
//! - ネットワーク I/O は行わない（offline 前提・CI 安全）。

use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

use crate::hover::{byte_offset_to_utf16, word_at_position};

// ---------------------------------------------------------------------------
// 公開インタフェース
// ---------------------------------------------------------------------------

/// Goto Definition 要求を処理して LSP GotoDefinitionResponse を返す。
///
/// カーソル位置のトークンが lane ID として宣言されている場合、その lane 宣言の
/// ソース位置（行・列）を返す。一致しない場合は `None`（ジャンプなし）。
///
/// `SourceSpan` はバイト列ベース 1-based。LSP `Position` は UTF-16 0-based のため変換する。
pub fn compute_goto_definition(
    source: &str,
    position: Position,
    uri: &Url,
) -> Option<GotoDefinitionResponse> {
    let (word, _word_range) = word_at_position(source, position)?;

    // IR を生成して lane を検索する。lower_static_with_source は Some(source) を渡すことで
    // 各 Lane に source_span を付与する。
    let file = tdsl_parser::parse(source).ok()?;
    let ir = tdsl_core::lower::lower_static_with_source(&file, Some(source)).ok()?;

    // 未定義 lane は None を返す（silent fallback 禁止）
    let lane = ir.lanes.iter().find(|l| l.id == word)?;

    // lane の source_span がなければジャンプ不可（source_span は source 渡し時のみ付与）
    let span = lane.source_span.as_ref()?;

    // SourceSpan は 1-based バイト列オフセット。LSP Position は 0-based UTF-16。
    // 行はそのまま saturating_sub(1) で 0-based に変換する。
    // 列は UTF-16 変換が必要なため行文字列を取得して byte_offset_to_utf16 を適用する。
    let line_0based = span.line.saturating_sub(1) as usize;
    let line_str = source.lines().nth(line_0based)?;

    // col_start / col_end はバイト列 1-based → 0-based に変換してから UTF-16 列数を算出
    let col_start_byte = span.col_start.saturating_sub(1) as usize;
    let col_end_byte = span.col_end.saturating_sub(1) as usize;

    let col_start_utf16 = byte_offset_to_utf16(line_str, col_start_byte) as u32;
    let col_end_utf16 = byte_offset_to_utf16(line_str, col_end_byte) as u32;

    let location = Location {
        uri: uri.clone(),
        range: Range {
            start: Position {
                line: line_0based as u32,
                character: col_start_utf16,
            },
            end: Position {
                line: line_0based as u32,
                character: col_end_utf16,
            },
        },
    };

    Some(GotoDefinitionResponse::Scalar(location))
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_SRC: &str = concat!(
        "timeline \"test\" { title \"test\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
        "lane \"漢\" as han { kind dynasty; order 10; }\n",
        "span han 100..200 \"foo\" {};\n",
    );

    // ---- compute_goto_definition ----

    /// lane ID の位置でカーソルを当てると lane 宣言行を返す
    #[test]
    fn goto_definition_lane_id_returns_declaration_line() {
        // 行 2 (0-based: 1) の "han" は `lane "漢" as han { ...`
        // l=0,a=1,n=2,e=3,' '=4,'"'=5,'漢'=6(utf-16=1),'"'=7,' '=8,'a'=9,'s'=10,' '=11,'h'=12,'a'=13,'n'=14
        let pos = Position {
            line: 2,      // span 行（0-based）の lane 参照
            character: 5, // "span han ..." の "han" 開始位置: s=0,p=1,a=2,n=3,' '=4,'h'=5
        };
        let uri = Url::parse("file:///test.tdsl").unwrap();
        let result = compute_goto_definition(MINI_SRC, pos, &uri);
        assert!(result.is_some(), "lane 'han' が存在するので Some を返す");

        if let Some(GotoDefinitionResponse::Scalar(loc)) = result {
            // lane 宣言は 0-based 行 1（2 行目）
            assert_eq!(loc.range.start.line, 1, "lane 宣言は 2 行目（0-based: 1）");
            assert_eq!(loc.uri, uri);
        } else {
            panic!("Scalar Location でない");
        }
    }

    /// 未定義の lane ID では None を返す（silent fallback 禁止）
    #[test]
    fn goto_definition_unknown_id_returns_none() {
        let pos = Position {
            line: 0,
            character: 0,
        };
        let src = concat!(
            "timeline \"test\" { title \"t\"; unit year; range 0..100; calendar proleptic_gregorian; }\n",
            "lane \"foo\" as foo { kind custom; order 1; }\n",
        );
        let uri = Url::parse("file:///test.tdsl").unwrap();
        // カーソルは "timeline" の 't' 上 → トークンは "timeline" → lane として存在しない
        let result = compute_goto_definition(src, pos, &uri);
        assert!(
            result.is_none(),
            "未定義 lane ID またはキーワードは None を返す"
        );
    }

    /// パースエラーのソースでは None を返す
    #[test]
    fn goto_definition_parse_error_returns_none() {
        let src = "timeline @@@invalid";
        let pos = Position {
            line: 0,
            character: 0,
        };
        let uri = Url::parse("file:///test.tdsl").unwrap();
        let result = compute_goto_definition(src, pos, &uri);
        assert!(result.is_none(), "パースエラーは None を返す");
    }

    /// カーソルが非トークン文字（スペース等）の上にある場合は None
    #[test]
    fn goto_definition_on_space_returns_none() {
        let uri = Url::parse("file:///test.tdsl").unwrap();
        // 行 0 の '{' の位置（非トークン文字）
        let pos = Position {
            line: 0,
            character: 16,
        };
        let result = compute_goto_definition(MINI_SRC, pos, &uri);
        assert!(result.is_none(), "非トークン位置は None を返す");
    }
}
