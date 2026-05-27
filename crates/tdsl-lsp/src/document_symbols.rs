//! `textDocument/documentSymbol` の純粋ロジック。
//!
//! `.tdsl` の構造（timeline / lane / 各アイテム）をエディタの
//! アウトライン / ブレッドクラム / シンボル検索に表示する。
//! ネットワーク I/O は行わない（offline 前提・CI 安全）。

use std::collections::HashMap;

use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};

use crate::hover::byte_offset_to_utf16;

/// ソーステキストから DocumentSymbol 一覧（階層構造）を返す。
///
/// 階層: timeline（MODULE） > lane（NAMESPACE） > item（EVENT/ARRAY）
///
/// パース不能なソースは空を返す。ネットワーク I/O は行わない。
pub fn compute_document_symbols(source: &str) -> Vec<DocumentSymbol> {
    let file = match tdsl_parser::parse(source) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let ir = match tdsl_core::lower::lower_static_with_source(&file, Some(source)) {
        Ok(ir) => ir,
        Err(_) => return Vec::new(),
    };

    let line_offsets = build_line_offsets(source);

    // アイテムを lane 別に分類
    let mut lane_children: HashMap<String, Vec<DocumentSymbol>> = HashMap::new();
    for item in &ir.items {
        let (lane_id, label, kind, item_span) = match item {
            tdsl_core::ir::Item::Span {
                lane,
                label,
                source_span,
                ..
            } => (
                lane.as_str(),
                label.as_str(),
                SymbolKind::ARRAY,
                source_span,
            ),
            tdsl_core::ir::Item::Event {
                lane,
                label,
                source_span,
                ..
            } => (
                lane.as_str(),
                label.as_str(),
                SymbolKind::EVENT,
                source_span,
            ),
            tdsl_core::ir::Item::EventRange {
                lane,
                label,
                source_span,
                ..
            } => (
                lane.as_str(),
                label.as_str(),
                SymbolKind::ARRAY,
                source_span,
            ),
        };
        let range = item_span
            .as_ref()
            .map(|s| ir_span_to_range(s, source))
            .unwrap_or_default();
        #[allow(deprecated)]
        let sym = DocumentSymbol {
            name: label.to_string(),
            detail: None,
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: None,
        };
        lane_children
            .entry(lane_id.to_string())
            .or_default()
            .push(sym);
    }

    // lane シンボルを構築（IR の順序を保持）
    let lane_syms: Vec<DocumentSymbol> = ir
        .lanes
        .iter()
        .map(|lane| {
            let range = lane
                .source_span
                .as_ref()
                .map(|s| ir_span_to_range(s, source))
                .unwrap_or_default();
            let children = lane_children.remove(&lane.id).unwrap_or_default();
            #[allow(deprecated)]
            DocumentSymbol {
                name: lane.label.clone(),
                detail: Some(lane.id.clone()),
                kind: SymbolKind::NAMESPACE,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            }
        })
        .collect();

    // timeline シンボル（AST の span を使用）
    let tl_range = find_timeline_range(&file, source, &line_offsets);
    let tl_name = if ir.meta.title.is_empty() {
        "timeline".to_string()
    } else {
        ir.meta.title.clone()
    };
    #[allow(deprecated)]
    let tl_sym = DocumentSymbol {
        name: tl_name,
        detail: None,
        kind: SymbolKind::MODULE,
        tags: None,
        deprecated: None,
        range: tl_range,
        selection_range: tl_range,
        children: if lane_syms.is_empty() {
            None
        } else {
            Some(lane_syms)
        },
    };

    vec![tl_sym]
}

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

/// IR の SourceSpan（1-based, バイト列）を LSP Range（0-based, UTF-16）に変換する。
fn ir_span_to_range(span: &tdsl_core::ir::SourceSpan, source: &str) -> Range {
    let line_0based = span.line.saturating_sub(1) as usize;
    let line_str = source.lines().nth(line_0based).unwrap_or("");
    let col_start =
        byte_offset_to_utf16(line_str, span.col_start.saturating_sub(1) as usize) as u32;
    let col_end = byte_offset_to_utf16(line_str, span.col_end.saturating_sub(1) as usize) as u32;
    Range {
        start: Position {
            line: line_0based as u32,
            character: col_start,
        },
        end: Position {
            line: line_0based as u32,
            character: col_end,
        },
    }
}

/// AST から timeline 宣言のソース範囲を取得する。
/// 見つからない場合はドキュメント全体を返す。
fn find_timeline_range(
    file: &tdsl_parser::ast::File,
    source: &str,
    line_offsets: &[usize],
) -> Range {
    for stmt in &file.statements {
        if matches!(&stmt.node, tdsl_parser::ast::Statement::Timeline(_)) {
            let start = byte_offset_to_position(stmt.span.start, source, line_offsets);
            let end = byte_offset_to_position(stmt.span.end, source, line_offsets);
            return Range { start, end };
        }
    }
    let last_line = source.lines().count().saturating_sub(1) as u32;
    let last_col = source.lines().last().map(|l| l.len() as u32).unwrap_or(0);
    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: last_line,
            character: last_col,
        },
    }
}

/// バイトオフセットを LSP Position（0-based, UTF-16 列）に変換する。
fn byte_offset_to_position(offset: usize, source: &str, line_offsets: &[usize]) -> Position {
    let line_idx = line_offsets
        .partition_point(|&o| o <= offset)
        .saturating_sub(1);
    let col_byte = offset - line_offsets[line_idx];
    let line_str = source.lines().nth(line_idx).unwrap_or("");
    Position {
        line: line_idx as u32,
        character: byte_offset_to_utf16(line_str, col_byte) as u32,
    }
}

// ---------------------------------------------------------------------------
// テスト
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const MINI_SRC: &str = concat!(
        "timeline \"test\" { title \"テスト年表\"; unit year; range 0..2000; calendar proleptic_gregorian; }\n",
        "lane \"漢\" as han { kind dynasty; order 10; }\n",
        "lane \"事\" as jikou { kind event; order 20; }\n",
        "span han 100..200 \"前漢\" {};\n",
        "event jikou 105 \"蔡倫\" {};\n",
        "event_range jikou 220..280 \"三国時代\" {};\n",
    );

    #[test]
    fn returns_one_top_level_timeline_symbol() {
        let syms = compute_document_symbols(MINI_SRC);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "テスト年表");
        assert_eq!(syms[0].kind, SymbolKind::MODULE);
    }

    #[test]
    fn timeline_has_lane_children() {
        let syms = compute_document_symbols(MINI_SRC);
        let children = syms[0].children.as_ref().expect("lane children が存在する");
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|c| c.kind == SymbolKind::NAMESPACE));
    }

    #[test]
    fn lane_has_item_children() {
        let syms = compute_document_symbols(MINI_SRC);
        let lanes = syms[0].children.as_ref().unwrap();
        let han = lanes
            .iter()
            .find(|l| l.name == "漢")
            .expect("漢 lane が存在する");
        let items = han.children.as_ref().expect("漢 lane にアイテムが存在する");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, SymbolKind::ARRAY); // span
    }

    #[test]
    fn event_has_event_symbol_kind() {
        let syms = compute_document_symbols(MINI_SRC);
        let lanes = syms[0].children.as_ref().unwrap();
        let jikou = lanes
            .iter()
            .find(|l| l.name == "事")
            .expect("事 lane が存在する");
        let items = jikou
            .children
            .as_ref()
            .expect("事 lane にアイテムが存在する");
        // event と event_range の 2 件
        assert_eq!(items.len(), 2);
        let event_sym = items.iter().find(|i| i.kind == SymbolKind::EVENT);
        assert!(event_sym.is_some(), "event アイテムは SymbolKind::EVENT");
    }

    #[test]
    fn parse_error_returns_empty() {
        let syms = compute_document_symbols("timeline @@@invalid");
        assert!(syms.is_empty(), "パースエラーは空を返す");
    }

    #[test]
    fn empty_source_returns_empty() {
        let syms = compute_document_symbols("");
        assert!(syms.is_empty());
    }

    #[test]
    fn detail_contains_lane_id() {
        let syms = compute_document_symbols(MINI_SRC);
        let lanes = syms[0].children.as_ref().unwrap();
        let han = lanes.iter().find(|l| l.name == "漢").unwrap();
        assert_eq!(
            han.detail.as_deref(),
            Some("han"),
            "detail に lane ID が入る"
        );
    }
}
