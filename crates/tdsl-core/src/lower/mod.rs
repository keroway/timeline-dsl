mod context;
mod declarations;
mod imports;
mod mapping;
mod static_items;

use tdsl_parser::ast;
#[cfg(feature = "wikidata")]
use tdsl_wikidata::WikidataClient;

use crate::error::LoweringError;
use crate::ir::TimelineIr;

use context::LoweringContext;

/// Lower a parsed AST into the canonical IR (static items only, no Wikidata).
pub fn lower_static(file: &ast::File) -> Result<TimelineIr, Vec<LoweringError>> {
    lower_static_with_source(file, None)
}

/// Lower a parsed AST into the canonical IR (static items only, no Wikidata).
/// `source` が与えられた場合、各アイテムに `source_span`（行番号・列番号）を付与する。
pub fn lower_static_with_source(
    file: &ast::File,
    source: Option<&str>,
) -> Result<TimelineIr, Vec<LoweringError>> {
    let line_offsets = source.map(build_line_offsets);
    let mut ctx = LoweringContext::new();
    ctx.pass1_declarations(file, line_offsets.as_deref());
    ctx.pass2_static_items(file, line_offsets.as_deref());
    ctx.finish()
}

/// Lower a parsed AST into the canonical IR with Wikidata resolution.
#[cfg(feature = "wikidata")]
pub async fn lower_with_wikidata(
    file: &ast::File,
    client: &dyn WikidataClient,
) -> Result<TimelineIr, Vec<LoweringError>> {
    lower_with_wikidata_and_source(file, client, None).await
}

/// Lower a parsed AST into the canonical IR with Wikidata resolution.
/// `source` が与えられた場合、各アイテムに `source_span`（行番号・列番号）を付与する。
#[cfg(feature = "wikidata")]
pub async fn lower_with_wikidata_and_source(
    file: &ast::File,
    client: &dyn WikidataClient,
    source: Option<&str>,
) -> Result<TimelineIr, Vec<LoweringError>> {
    let line_offsets = source.map(build_line_offsets);
    let mut ctx = LoweringContext::new();
    ctx.pass1_declarations(file, line_offsets.as_deref());
    ctx.pass2_static_items(file, line_offsets.as_deref());
    // Wikidata フェッチ前に early exit: pass1/pass2 のエラー（未宣言lane等）を即返す。
    // これにより offline でも未宣言 lane を即座に報告でき、不要な API 呼び出しを回避する。
    if !ctx.errors.is_empty() {
        return Err(ctx.errors);
    }
    ctx.pass3_resolve_imports(file, client).await;
    ctx.pass4_apply_maps(file);
    ctx.finish()
}

// ─── Standalone Helpers ─────────────────────────────────────

/// ソーステキストから各行の先頭バイトオフセット配列を構築する（0-indexed）。
/// `line_offsets[0]` は行0（=行番号1）の先頭オフセット（常に0）。
pub(crate) fn build_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// 自動生成 ID 用に時刻を `YYYY` / `YYYY-MM` / `YYYY-MM-DD` 形式に整形する。
pub(crate) fn format_id_time(t: &ast::TimeValue) -> String {
    match t {
        ast::TimeValue::Year(y) => format!("{y}"),
        ast::TimeValue::YearMonth(y, m) => format!("{y:04}-{m:02}"),
        ast::TimeValue::Date(y, m, d) => format!("{y:04}-{m:02}-{d:02}"),
    }
}

/// バイトオフセットから (1-based 行番号, 1-based 列番号) に変換する。
pub(crate) fn offset_to_line_col(offset: usize, line_offsets: &[usize]) -> (u32, u32) {
    let line_idx = line_offsets
        .partition_point(|&o| o <= offset)
        .saturating_sub(1);
    let col = offset - line_offsets[line_idx];
    ((line_idx + 1) as u32, (col + 1) as u32)
}

/// Build a human-readable hint for an unknown lane reference.
/// Shows similar-looking candidates (prefix match or substring) first, then all available.
pub(crate) fn lane_suggestion_hint(unknown: &str, available: &[String]) -> String {
    if available.is_empty() {
        return "定義済みのlaneがありません。先にlane宣言を追加してください".to_string();
    }

    // Find candidates that share a common prefix (>=2 chars), or contain/are contained by unknown
    let u_lower = unknown.to_lowercase();
    let u_prefix: String = u_lower.chars().take(2).collect();
    let similar: Vec<&str> = available
        .iter()
        .filter(|candidate| {
            let c = candidate.to_lowercase();
            c.starts_with(u_prefix.as_str()) || c.contains(&u_lower) || u_lower.contains(c.as_str())
        })
        .map(|s| s.as_str())
        .collect();

    let all: Vec<&str> = available.iter().map(|s| s.as_str()).collect();

    if !similar.is_empty() && similar != all {
        format!(
            "もしかして: {} ？（利用可能なlane: {}）",
            similar.join(", "),
            all.join(", ")
        )
    } else {
        format!("利用可能なlane: {}", all.join(", "))
    }
}

pub(crate) fn source_str(sr: &Option<tdsl_parser::ast::SourceRef>) -> Option<String> {
    sr.as_ref().map(|s| format!("{}:{}", s.prefix, s.qid))
}

pub(crate) fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}
