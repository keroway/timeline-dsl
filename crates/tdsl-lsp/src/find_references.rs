//! `textDocument/references` の純粋ロジック。
//!
//! - lane 宣言・lane 参照のいずれにカーソルがあっても、その lane ID の全参照位置を返す。
//! - `includeDeclaration` フラグに応じて lane 宣言位置を含める／含めない。
//! - 未定義 lane や lane 以外のトークンでは `None` を返す（silent fallback 禁止）。
//! - ネットワーク I/O は行わない（offline 前提・CI 安全）。

use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::hover::{byte_offset_to_utf16, word_at_position};

// ---------------------------------------------------------------------------
// 内部ヘルパー
// ---------------------------------------------------------------------------

/// ソーステキストの各行の先頭バイトオフセット配列（0-indexed）を構築する。
fn build_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// バイトオフセットを LSP Position（0-based, UTF-16 列）に変換する。
fn byte_offset_to_position(offset: usize, source: &str, line_offsets: &[usize]) -> Position {
    let line_idx = line_offsets
        .partition_point(|&o| o <= offset)
        .saturating_sub(1);
    let col_byte = offset.saturating_sub(line_offsets[line_idx]);
    let line_str = source.lines().nth(line_idx).unwrap_or("");
    Position {
        line: line_idx as u32,
        character: byte_offset_to_utf16(line_str, col_byte) as u32,
    }
}

/// statement 先頭バイトオフセット `stmt_start` から始まるソーステキストにおいて、
/// 第2トークン（lane_ref）のバイト範囲 `(abs_start, abs_end)` を返す。
///
/// DSL 文の構造: `<keyword> <lane_ref> ...`
/// トークン文字: `[A-Za-z0-9_]`
fn second_token_range(source: &str, stmt_start: usize) -> Option<(usize, usize)> {
    let slice = source.get(stmt_start..)?;
    let is_token_char = |c: char| c.is_ascii_alphanumeric() || c == '_';

    let mut offset = 0usize;

    // 第1トークン（キーワード）をスキップ
    // まず先頭の空白を読み飛ばす（通常はないが念のため）
    while offset < slice.len() {
        let ch = slice[offset..].chars().next()?;
        if !ch.is_whitespace() {
            break;
        }
        offset += ch.len_utf8();
    }
    // キーワード本体をスキップ
    while offset < slice.len() {
        let ch = slice[offset..].chars().next()?;
        if !is_token_char(ch) {
            break;
        }
        offset += ch.len_utf8();
    }
    // キーワードと lane_ref の間の空白をスキップ
    while offset < slice.len() {
        let ch = slice[offset..].chars().next()?;
        if !ch.is_whitespace() {
            break;
        }
        offset += ch.len_utf8();
    }

    // ここから第2トークン（lane_ref）の開始
    let token_start = stmt_start + offset;

    // lane_ref の末尾まで進む
    while offset < slice.len() {
        let ch = slice[offset..].chars().next()?;
        if !is_token_char(ch) {
            break;
        }
        offset += ch.len_utf8();
    }
    let token_end = stmt_start + offset;

    if token_start >= token_end {
        return None;
    }

    Some((token_start, token_end))
}

/// `map` ブロックのバイト範囲 `source[block_start..block_end]` の中で
/// `lane <lane_id>` パターンを探し、`lane_id` トークンの絶対バイト範囲を返す。
///
/// 文字列リテラル（`"..."` で囲まれた部分）内のマッチはスキップする。
fn find_lane_prop_in_map(
    source: &str,
    block_start: usize,
    block_end: usize,
    lane_id: &str,
) -> Option<(usize, usize)> {
    let slice = source.get(block_start..block_end)?;

    let keyword = "lane";
    let kw_bytes = keyword.as_bytes();
    let id_bytes = lane_id.as_bytes();

    let is_token_char = |c: u8| c.is_ascii_alphanumeric() || c == b'_';

    let mut pos = 0usize;
    let bytes = slice.as_bytes();

    while pos < bytes.len() {
        // 文字列リテラルのスキップ
        if bytes[pos] == b'"' {
            pos += 1;
            while pos < bytes.len() {
                if bytes[pos] == b'\\' {
                    pos += 2; // エスケープシーケンスをスキップ
                } else if bytes[pos] == b'"' {
                    pos += 1;
                    break;
                } else {
                    pos += 1;
                }
            }
            continue;
        }

        // `lane` キーワードの探索
        if bytes[pos..].starts_with(kw_bytes) {
            let after_kw = pos + kw_bytes.len();
            // `lane` の直後がトークン文字でない（境界チェック）
            let kw_is_word = if after_kw < bytes.len() {
                is_token_char(bytes[after_kw])
            } else {
                false
            };
            // `lane` の前がトークン文字でない（境界チェック）
            let kw_start_ok = if pos > 0 {
                !is_token_char(bytes[pos - 1])
            } else {
                true
            };

            if kw_start_ok && !kw_is_word {
                // `lane` の後の空白をスキップして lane_id を確認
                let mut id_pos = after_kw;
                while id_pos < bytes.len() && (bytes[id_pos] == b' ' || bytes[id_pos] == b'\t') {
                    id_pos += 1;
                }
                if bytes[id_pos..].starts_with(id_bytes) {
                    let after_id = id_pos + id_bytes.len();
                    // id の後がトークン文字でないことを確認（完全一致）
                    let id_end_ok = if after_id < bytes.len() {
                        !is_token_char(bytes[after_id])
                    } else {
                        true
                    };
                    if id_end_ok {
                        let abs_start = block_start + id_pos;
                        let abs_end = block_start + after_id;
                        return Some((abs_start, abs_end));
                    }
                }
            }
        }

        pos += 1;
    }

    None
}

// ---------------------------------------------------------------------------
// 公開インタフェース
// ---------------------------------------------------------------------------

/// Find References 要求を処理して LSP Location 一覧を返す。
///
/// カーソル位置のトークンが lane ID として宣言されている場合、その lane ID の
/// 全参照位置（span / event / event_range の lane_ref、map ブロックの MapProp::Lane）を返す。
/// `include_declaration` が true の場合は lane 宣言位置も含む。
///
/// 未定義 lane またはカーソルが lane 以外のトークン上にある場合は `None` を返す。
pub fn compute_references(
    source: &str,
    position: Position,
    include_declaration: bool,
    uri: &Url,
) -> Option<Vec<Location>> {
    let (word, _word_range) = word_at_position(source, position)?;

    // IR を生成して lane 存在確認と宣言位置取得を行う
    let file = tdsl_parser::parse(source).ok()?;
    let ir = tdsl_core::lower::lower_static_with_source(&file, Some(source)).ok()?;

    // 未定義 lane は None を返す（silent fallback 禁止）
    let lane = ir.lanes.iter().find(|l| l.id == word)?;

    let line_offsets = build_line_offsets(source);
    let mut locations: Vec<Location> = Vec::new();

    // include_declaration が true なら lane 宣言位置を先頭に追加
    if include_declaration && let Some(span) = lane.source_span.as_ref() {
        let line_0based = span.line.saturating_sub(1) as usize;
        let line_str = source.lines().nth(line_0based).unwrap_or("");
        let col_start =
            byte_offset_to_utf16(line_str, span.col_start.saturating_sub(1) as usize) as u32;
        let col_end =
            byte_offset_to_utf16(line_str, span.col_end.saturating_sub(1) as usize) as u32;
        locations.push(Location {
            uri: uri.clone(),
            range: Range {
                start: Position {
                    line: line_0based as u32,
                    character: col_start,
                },
                end: Position {
                    line: line_0based as u32,
                    character: col_end,
                },
            },
        });
    }

    // AST の各 statement を走査して lane_ref への参照を収集
    for stmt in &file.statements {
        match &stmt.node {
            tdsl_parser::ast::Statement::Span(decl) if decl.lane_ref == word => {
                if let Some((start, end)) = second_token_range(source, stmt.span.start) {
                    let range = Range {
                        start: byte_offset_to_position(start, source, &line_offsets),
                        end: byte_offset_to_position(end, source, &line_offsets),
                    };
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
            tdsl_parser::ast::Statement::Event(decl) if decl.lane_ref == word => {
                if let Some((start, end)) = second_token_range(source, stmt.span.start) {
                    let range = Range {
                        start: byte_offset_to_position(start, source, &line_offsets),
                        end: byte_offset_to_position(end, source, &line_offsets),
                    };
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
            tdsl_parser::ast::Statement::EventRange(decl) if decl.lane_ref == word => {
                if let Some((start, end)) = second_token_range(source, stmt.span.start) {
                    let range = Range {
                        start: byte_offset_to_position(start, source, &line_offsets),
                        end: byte_offset_to_position(end, source, &line_offsets),
                    };
                    locations.push(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
            tdsl_parser::ast::Statement::Map(block) => {
                // MapProp::Lane を検索
                for prop in &block.props {
                    if let tdsl_parser::ast::MapProp::Lane(id) = prop
                        && id == &word
                    {
                        if let Some((start, end)) =
                            find_lane_prop_in_map(source, stmt.span.start, stmt.span.end, &word)
                        {
                            let range = Range {
                                start: byte_offset_to_position(start, source, &line_offsets),
                                end: byte_offset_to_position(end, source, &line_offsets),
                            };
                            locations.push(Location {
                                uri: uri.clone(),
                                range,
                            });
                        }
                        // 1つの map ブロックには Lane prop は最大1つなのでここで break
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    Some(locations)
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
        "event han 150 \"bar\" {};\n",
        "event_range han 160..170 \"baz\" {};\n",
    );

    fn test_uri() -> Url {
        Url::parse("file:///test.tdsl").unwrap()
    }

    /// span/event/event_range の3参照を返す
    #[test]
    fn test_lane_ref_returns_all_references() {
        // span 行（0-based: 2）の "han" 上にカーソル
        // "span han ..." の "han" は character=5 (s=0,p=1,a=2,n=3,' '=4,'h'=5)
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_references(MINI_SRC, pos, false, &test_uri());
        assert!(result.is_some(), "lane 'han' が存在するので Some を返す");
        let locs = result.unwrap();
        assert_eq!(
            locs.len(),
            3,
            "span/event/event_range の3参照を返す: {locs:?}"
        );
    }

    /// include_declaration=true で宣言位置も含む（合計4件）
    #[test]
    fn test_include_declaration_adds_lane_decl() {
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_references(MINI_SRC, pos, true, &test_uri());
        assert!(result.is_some());
        let locs = result.unwrap();
        assert_eq!(locs.len(), 4, "宣言含む4件を返す: {locs:?}");
        // 最初の location は lane 宣言行（0-based: 1）
        assert_eq!(locs[0].range.start.line, 1, "宣言は2行目（0-based: 1）");
    }

    /// include_declaration=false で3件のみ
    #[test]
    fn test_exclude_declaration_omits_lane_decl() {
        let pos = Position {
            line: 3,
            character: 6,
        };
        let result = compute_references(MINI_SRC, pos, false, &test_uri());
        assert!(result.is_some());
        let locs = result.unwrap();
        assert_eq!(locs.len(), 3, "宣言除く3件を返す: {locs:?}");
        // 宣言行（0-based: 1）は含まれない
        assert!(
            locs.iter().all(|l| l.range.start.line != 1),
            "宣言行は含まれない: {locs:?}"
        );
    }

    /// 未定義 lane は None を返す
    #[test]
    fn test_unknown_token_returns_none() {
        // "timeline" キーワード上にカーソル → lane として存在しない
        let pos = Position {
            line: 0,
            character: 0,
        };
        let result = compute_references(MINI_SRC, pos, false, &test_uri());
        assert!(result.is_none(), "未定義 lane ID は None を返す");
    }

    /// カーソルを lane 宣言行の `han` に置いた場合も同様に動作する
    #[test]
    fn test_on_declaration_returns_refs() {
        // lane 宣言行（0-based: 1）の "han" にカーソル
        // `lane "漢" as han { ...` の "han":
        // l=0,a=1,n=2,e=3,' '=4,'"'=5,'漢'=6(UTF-16 len=1),'"'=7,' '=8,'a'=9,'s'=10,' '=11,'h'=12
        let pos = Position {
            line: 1,
            character: 12,
        };
        let result = compute_references(MINI_SRC, pos, false, &test_uri());
        assert!(result.is_some(), "宣言行からでも参照を返す");
        let locs = result.unwrap();
        assert_eq!(
            locs.len(),
            3,
            "span/event/event_range の3参照を返す: {locs:?}"
        );
    }

    /// map ブロック内の MapProp::Lane 参照を検出する
    #[test]
    fn test_map_lane_prop_included() {
        // map_block の文法: "map" ~ dotted_ident ~ "to" ~ target_type ~ "{" ~ map_prop* ~ "}"
        // dotted_ident は `alias.key` 形式（例: `wd.e1`）
        // label_ref の文法: "label" ~ "@" ~ ident
        let src = concat!(
            "timeline \"test\" { title \"t\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
            "lane \"漢\" as han { kind dynasty; order 10; }\n",
            "span han 100..200 \"foo\" {};\n",
            "import wikidata as wd { entity Q1 as e1; policy merge_by_source; }\n",
            "map wd.e1 to span { lane han; start claim(P580).year; end claim(P582).year; label label@ja; }\n",
        );

        // span 行（0-based: 2）の "han" 上
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_references(src, pos, false, &test_uri());
        assert!(result.is_some(), "パース・lowering が成功して Some を返す");
        let locs = result.unwrap();
        // span の lane_ref + map の MapProp::Lane = 2件
        assert_eq!(locs.len(), 2, "span参照 + map参照の2件: {locs:?}");
    }

    /// 参照の行・列位置が正確であることを確認する
    #[test]
    fn test_reference_positions_are_correct() {
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = compute_references(MINI_SRC, pos, false, &test_uri());
        let locs = result.unwrap();

        // span 行（0-based: 2）の参照
        let span_loc = locs.iter().find(|l| l.range.start.line == 2);
        assert!(span_loc.is_some(), "span 行の参照が含まれる");
        let span_loc = span_loc.unwrap();
        // "span han ..." の "han" は character=5
        assert_eq!(
            span_loc.range.start.character, 5,
            "span の lane_ref 開始位置は character=5"
        );
        assert_eq!(
            span_loc.range.end.character, 8,
            "span の lane_ref 終了位置は character=8"
        );
    }
}
