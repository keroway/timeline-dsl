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
    lower_static_with_diagnostics(file, source).map(|(ir, _)| ir)
}

/// `lower_static_with_source` と同じだが、lowering 中に蓄積した非致命的な警告
/// （必須フィールド未解決でアイテムが生成されなかった等）も返す。
pub fn lower_static_with_diagnostics(
    file: &ast::File,
    source: Option<&str>,
) -> Result<(TimelineIr, Vec<String>), Vec<LoweringError>> {
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
    lower_with_wikidata_and_diagnostics(file, client, source)
        .await
        .map(|(ir, _)| ir)
}

/// `lower_with_wikidata_and_source` と同じだが、lowering 中に蓄積した非致命的な
/// 警告（マップ対象エンティティが必須フィールド未解決でアイテムを生成しなかった等）も返す。
#[cfg(feature = "wikidata")]
pub async fn lower_with_wikidata_and_diagnostics(
    file: &ast::File,
    client: &dyn WikidataClient,
    source: Option<&str>,
) -> Result<(TimelineIr, Vec<String>), Vec<LoweringError>> {
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

/// 自動生成 ID 用に時刻を `YYYY` / `YYYY-MM` / `YYYY-MM-DD` / `YYYY-MM-DDTHH:MM` 形式に整形する。
pub(crate) fn format_id_time(t: &ast::TimeValue) -> String {
    match t {
        ast::TimeValue::Year(y) => format!("{y}"),
        ast::TimeValue::YearMonth(y, m) => format!("{y:04}-{m:02}"),
        ast::TimeValue::Date(y, m, d) => format!("{y:04}-{m:02}-{d:02}"),
        ast::TimeValue::DateTime(y, m, d, h, min) => {
            format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}")
        }
        ast::TimeValue::DateTimeSecond(y, m, d, h, min, s) => {
            format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}")
        }
        ast::TimeValue::DateTimeOffset(y, m, d, h, min, off) => {
            format!(
                "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}{}",
                format_offset_suffix(*off)
            )
        }
        ast::TimeValue::DateTimeSecondOffset(y, m, d, h, min, s, off) => {
            format!(
                "{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}{}",
                format_offset_suffix(*off)
            )
        }
    }
}

/// offset(分単位)を `Z` または `±HH:MM` 形式に整形する（ID用の補助関数）。
pub(crate) fn format_offset_suffix(offset_minutes: i16) -> String {
    if offset_minutes == 0 {
        return "Z".to_string();
    }
    let sign = if offset_minutes < 0 { '-' } else { '+' };
    let abs = offset_minutes.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 60, abs % 60)
}

/// ADR 0003 D2 に準拠した、offset の有無を考慮した時刻比較。
///
/// - offset 付き同士は `offset_minutes` を差し引いて UTC 相当の暦時刻に正規化して比較する。
/// - offset なし同士は、従来どおり暦時刻の値そのもので比較する（`to_sortable()` 相当）。
/// - 片方のみ offset 付きの場合は曖昧な比較として明示エラー（`MixedOffsetComparison`）を返す。
///   offset なしを暗黙に UTC とみなすことはしない（AGENTS.md §4.1）。
pub(crate) fn compare_time_values(
    a: &ast::TimeValue,
    b: &ast::TimeValue,
) -> Result<std::cmp::Ordering, crate::error::LoweringError> {
    match (a.offset_minutes(), b.offset_minutes()) {
        (Some(off_a), Some(off_b)) => {
            let norm_a = normalize_sortable_utc(a, off_a);
            let norm_b = normalize_sortable_utc(b, off_b);
            Ok(norm_a.cmp(&norm_b))
        }
        (None, None) => Ok(a.to_sortable().cmp(&b.to_sortable())),
        _ => Err(crate::error::LoweringError::MixedOffsetComparison(
            a.to_string(),
            b.to_string(),
        )),
    }
}

/// offset付き civil time を UTC 秒へ正規化する。日跨ぎ・月跨ぎ・BCEにも対応する
/// proleptic Gregorian の整数演算だけを使い、外部日時クレートには依存しない（ADR 0003 D6）。
fn normalize_sortable_utc(t: &ast::TimeValue, offset_minutes: i16) -> i128 {
    let (year, month, day) = t.to_sortable();
    let days = days_from_civil(year, month.max(1), day.max(1));
    let seconds = i128::from(t.hour().unwrap_or(0)) * 3600
        + i128::from(t.minute().unwrap_or(0)) * 60
        + i128::from(t.second().unwrap_or(0))
        - i128::from(offset_minutes) * 60;
    days * 86_400 + seconds
}

/// Howard Hinnant の civil-date-to-days 変換を i128/BCE対応に移植したもの。
/// `validate.rs` も（IRプリミティブ値から）同じ正規化を行うためこの関数を共有する（DRY）。
pub(crate) fn days_from_civil(year: i64, month: u8, day: u8) -> i128 {
    let year = i128::from(year) - i128::from((month <= 2) as u8);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = i128::from(month);
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i128::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe
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
    sr.as_ref().map(ToString::to_string)
}

pub(crate) fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}
