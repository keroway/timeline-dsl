use wasm_bindgen::prelude::*;

use tdsl_core::lower::lower_static_with_source;
use tdsl_render::{RenderOptions, render_html, render_svg_only};

/// Initialize the panic hook for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Format TDSL source by re-emitting from the AST.
///
/// Parses `source` and re-emits a normalized form (2-space indent, single blank line
/// between top-level statements). Comments in the original source are **not preserved**
/// because they are skipped at the PEG layer.
/// Returns Ok(formatted_source) on success, Err(parse_error_message) on parse failure.
#[wasm_bindgen]
pub fn format_source(source: &str) -> Result<String, JsValue> {
    tdsl_parser::format_source(source).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compile TDSL source to IR (JSON string), without Wikidata resolution.
/// `source_span` fields are populated for each static item (for bidirectional jump).
/// Returns Ok(json_string) or Err(error_message).
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static_with_source(&file, Some(source)).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    serde_json::to_string_pretty(&ir).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compute auto scale (pixels-per-year) for a given year span.
/// Targets ~1000px of total timeline width.
/// - Upper bound 50 px/year keeps tick labels (~25px wide @ 11px font-size)
///   from overlapping on short ranges (e.g. span < 20 years).
/// - Lower bound 0.5 px/year prevents excessive width on multi-millennia ranges.
fn auto_scale_for_span(span: f64) -> f64 {
    (1000.0 / span).clamp(0.5, 50.0)
}

/// Render SVG from TDSL source (static items only).
/// `scale` controls pixels-per-year. Pass `0.0` (or negative) to auto-calculate
/// from the IR's `meta.range` (clamped to `0.5..=50.0`).
/// `source_span` (line numbers) are embedded as `data-line` attributes in the SVG
/// for bidirectional editor↔preview jump.
/// Returns Ok(svg_string) or Err(error_message).
#[wasm_bindgen]
pub fn render_svg_from_source(source: &str, scale: f64) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static_with_source(&file, Some(source)).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    let computed_scale = if scale > 0.0 {
        scale
    } else {
        let range = ir.meta.range;
        let span = (range.1 - range.0).unsigned_abs().max(1) as f64;
        auto_scale_for_span(span)
    };
    let svg = render_svg_only(
        &ir,
        RenderOptions {
            scale: computed_scale,
            ..RenderOptions::default()
        },
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

    fn make_ir(range_start: i64, range_end: i64) -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "Test".into(),
                unit: "year".into(),
                range: (range_start, range_end),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "a".into(),
                label: "A".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Span {
                id: "span:a:0".into(),
                lane: "a".into(),
                start: range_start,
                end: range_end,
                label: "Item".into(),
                tags: vec![],
                source: None,
                origin: None,
                start_month: None,
                start_day: None,
                end_month: None,
                end_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        }
    }

    fn extract_svg_width(svg: &str) -> f64 {
        let start = svg.find("width=\"").expect("width attr in SVG") + 7;
        let end = svg[start..].find('"').unwrap() + start;
        svg[start..end].parse().expect("numeric width")
    }

    // ─── auto_scale_for_span unit tests ───────────────────────────────────────

    #[test]
    fn auto_scale_short_range_clamped_at_upper() {
        // span=12: 1000/12≈83.3 → clamped to 50.0 (was 8.0 before this fix)
        assert_eq!(auto_scale_for_span(12.0), 50.0);
    }

    #[test]
    fn auto_scale_medium_range_unclamped() {
        // span=125: 1000/125=8.0, within [0.5, 50] — same as before the fix
        assert!((auto_scale_for_span(125.0) - 8.0).abs() < 0.01);
    }

    #[test]
    fn auto_scale_long_range_clamped_at_lower() {
        // span=5000: 1000/5000=0.2 → clamped to 0.5
        assert_eq!(auto_scale_for_span(5000.0), 0.5);
    }

    // ─── SVG output integration tests (no wasm_bindgen involved) ─────────────

    #[test]
    fn svg_width_is_readable_for_short_range() {
        // span=12, scale=50 → width = 120 + 12*50 + 20 = 740px
        let ir = make_ir(2018, 2030);
        let svg = render_svg_only(
            &ir,
            RenderOptions {
                scale: auto_scale_for_span(12.0),
                ..RenderOptions::default()
            },
        )
        .unwrap();
        let w = extract_svg_width(&svg);
        assert!(w >= 700.0, "span=12 SVG width should be ≥700px, got {w}");
    }

    #[test]
    fn svg_width_for_medium_range_unchanged() {
        // span=125, scale=8 → width = 120 + 125*8 + 20 = 1140px
        let ir = make_ir(1900, 2025);
        let svg = render_svg_only(
            &ir,
            RenderOptions {
                scale: auto_scale_for_span(125.0),
                ..RenderOptions::default()
            },
        )
        .unwrap();
        let w = extract_svg_width(&svg);
        assert!(
            (w - 1140.0).abs() < 1.0,
            "span=125 width should be ~1140px, got {w}"
        );
    }

    #[test]
    fn svg_width_for_long_range_uses_lower_clamp() {
        // span=5000, scale=0.5 → width = 120 + 5000*0.5 + 20 = 2640px
        let ir = make_ir(-3000, 2000);
        let svg = render_svg_only(
            &ir,
            RenderOptions {
                scale: auto_scale_for_span(5000.0),
                ..RenderOptions::default()
            },
        )
        .unwrap();
        let w = extract_svg_width(&svg);
        assert!(
            (w - 2640.0).abs() < 1.0,
            "span=5000 width should be ~2640px, got {w}"
        );
    }

    // ─── compile_to_ir / lower logic tests ───────────────────────────────────
    // These tests exercise the same logic as compile_to_ir() but call the
    // underlying pure-Rust functions so they work in native (non-WASM) test runs.

    const VALID_SRC: &str = r#"timeline "Test" {
    unit year;
    range 0..100;
}
lane "A" as a {}
span a 10..50 "A Span" {};
"#;

    #[test]
    fn lower_valid_source_produces_ir_with_expected_fields() {
        let file = tdsl_parser::parse(VALID_SRC).expect("valid source must parse");
        let ir = tdsl_core::lower::lower_static_with_source(&file, Some(VALID_SRC))
            .expect("valid source must lower");
        assert_eq!(ir.lanes.len(), 1);
        assert_eq!(ir.items.len(), 1);
        let json = serde_json::to_string_pretty(&ir).expect("IR must serialize to JSON");
        assert!(
            json.contains(r#""meta""#),
            "IR JSON should contain meta key"
        );
        assert!(
            json.contains(r#""lanes""#),
            "IR JSON should contain lanes key"
        );
        assert!(
            json.contains(r#""items""#),
            "IR JSON should contain items key"
        );
    }

    #[test]
    fn lower_parse_error_returns_err() {
        let result = tdsl_parser::parse("this is !!! not valid tdsl");
        assert!(result.is_err(), "invalid syntax must fail to parse");
    }

    #[test]
    fn lower_unknown_lane_returns_lowering_err() {
        let src = r#"timeline "Test" {
    unit year;
    range 0..100;
}
span nonexistent 10..50 "Bad Span" {};
"#;
        let file = tdsl_parser::parse(src).expect("source must parse");
        let result = tdsl_core::lower::lower_static_with_source(&file, Some(src));
        assert!(
            result.is_err(),
            "unknown lane must produce a lowering error"
        );
    }

    // ─── format_source logic tests ────────────────────────────────────────────

    #[test]
    fn format_valid_source_output_reparses() {
        let formatted = tdsl_parser::format_source(VALID_SRC).expect("valid source must format");
        let reparse = tdsl_parser::parse(&formatted);
        assert!(
            reparse.is_ok(),
            "formatted output should re-parse successfully, got {reparse:?}"
        );
    }

    #[test]
    fn format_invalid_source_returns_err() {
        let result = tdsl_parser::format_source("this is !!! not valid tdsl");
        assert!(result.is_err(), "invalid syntax must fail to format");
    }

    // ─── check_source / validate logic tests ─────────────────────────────────

    #[test]
    fn validate_valid_source_produces_no_warnings() {
        let file = tdsl_parser::parse(VALID_SRC).expect("valid source must parse");
        let ir = tdsl_core::lower::lower_static_with_source(&file, Some(VALID_SRC))
            .expect("valid source must lower");
        let warnings = tdsl_core::validate::validate(&ir);
        assert!(
            warnings.is_empty(),
            "no warnings expected for valid source, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_inverted_span_produces_warning() {
        let src = r#"timeline "Test" {
    unit year;
    range 0..100;
}
lane "A" as a {}
span a 80..10 "Inverted" {};
"#;
        let file = tdsl_parser::parse(src).expect("source must parse");
        let ir = tdsl_core::lower::lower_static_with_source(&file, Some(src))
            .expect("source must lower");
        let warnings = tdsl_core::validate::validate(&ir);
        assert!(
            !warnings.is_empty(),
            "inverted span should produce at least one warning"
        );
    }

    #[test]
    fn check_source_json_is_valid_for_valid_source() {
        let json = check_source(VALID_SRC);
        let result: Result<Vec<serde_json::Value>, _> = serde_json::from_str(&json);
        assert!(
            result.is_ok(),
            "check_source must always return valid JSON array"
        );
        assert!(
            result.unwrap().is_empty(),
            "no diagnostics expected for valid source"
        );
    }

    // ─── check_source line/col population (issue #386) ────────────────────────

    #[test]
    fn check_source_parse_error_has_accurate_line_col() {
        // span のラベルが閉じておらず 4 行目で構文エラーになるソース。
        let src = "timeline \"T\" {\n    unit year;\n    range 0..100;\n}\nlane \"A\" as a {}\nspan a 10..50 \"unterminated;\n";
        let json = check_source(src);
        let diags: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("must return JSON array");
        assert_eq!(diags.len(), 1, "expected a single parse error diagnostic");
        let d = &diags[0];
        assert_eq!(d["severity"], "error");
        let line = d["line"].as_u64().expect("line is a number");
        let col = d["col"].as_u64().expect("col is a number");
        assert!(
            line >= 1,
            "parse error must carry a 1-based line, got {line}"
        );
        assert!(col >= 1, "parse error must carry a 1-based col, got {col}");
    }

    #[test]
    fn check_source_validation_warning_has_span_line() {
        // 6 行目の反転 span（80..10）が validation 警告になる。
        let src = "timeline \"T\" {\n    unit year;\n    range 0..100;\n}\nlane \"A\" as a {}\nspan a 80..10 \"Inverted\" {};\n";
        let json = check_source(src);
        let diags: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("must return JSON array");
        assert!(!diags.is_empty(), "inverted span should warn");
        let warning = diags
            .iter()
            .find(|d| d["severity"] == "warning")
            .expect("expected a warning diagnostic");
        let line = warning["line"].as_u64().expect("line is a number");
        assert_eq!(
            line, 6,
            "inverted span warning should point at its source line (6), got {line}"
        );
        assert!(
            warning["col"].as_u64().expect("col is a number") >= 1,
            "warning col should be 1-based"
        );
    }

    #[test]
    fn check_source_lowering_error_without_span_reports_zero() {
        // 未宣言 lane は LoweringError（span なし）→ line/col は 0 のまま。
        let src = "timeline \"T\" {\n    unit year;\n    range 0..100;\n}\nspan nonexistent 10..50 \"Bad\" {};\n";
        let json = check_source(src);
        let diags: Vec<serde_json::Value> =
            serde_json::from_str(&json).expect("must return JSON array");
        assert!(!diags.is_empty(), "unknown lane should produce an error");
        let err = diags
            .iter()
            .find(|d| d["severity"] == "error")
            .expect("expected an error diagnostic");
        assert_eq!(
            err["line"].as_u64().unwrap(),
            0,
            "position-less lowering error reports line 0"
        );
        assert_eq!(err["col"].as_u64().unwrap(), 0);
    }
}

/// Render standalone HTML from TDSL source (static items only).
/// Returns Ok(html_string) or Err(error_message).
#[wasm_bindgen]
pub fn render_html_from_source(source: &str) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static_with_source(&file, Some(source)).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    render_html(&ir, RenderOptions::default()).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Diagnostic severity level.
#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Error,
    Warning,
}

/// A single diagnostic item returned by `check_source`.
#[derive(serde::Serialize)]
struct Diagnostic {
    severity: Severity,
    message: String,
    line: u32,
    col: u32,
}

/// Check TDSL source and return diagnostics as JSON.
///
/// Returns a JSON array of diagnostic objects: `[{severity, message, line, col}]`.
/// `severity` is `"error"` or `"warning"`.
///
/// `line`/`col` are **1-based** when a source position is available (parse errors via
/// `ParseError::source_location`, validation warnings via the item's `source_span`),
/// matching the IR `SourceSpan` numbering used by `render_svg_from_source`'s `data-line`
/// attributes. Diagnostics that carry no position (lowering errors such as unknown-lane
/// references) report `line: 0, col: 0`; the WebUI treats a `0` line as non-clickable.
///
/// **Note on `import` blocks**: `import wikidata` blocks are not resolved in the browser
/// (no network access). Unresolved imports are **silently skipped** — they produce no
/// diagnostics, but the resulting IR / SVG will omit those items. Use static `span`,
/// `event`, and `event_range` statements for content that must render in the browser.
#[wasm_bindgen]
pub fn check_source(source: &str) -> String {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let file = match tdsl_parser::parse(source) {
        Ok(f) => f,
        Err(e) => {
            // パースエラーは pest の line_col / バイトオフセットから 1-based 位置を取得する。
            // 位置を持たない variant（UnknownPolicy 等）は 0/0 のまま。
            let (line, col) = e
                .source_location(source)
                .map_or((0, 0), |loc| (loc.line, loc.col));
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: e.to_string(),
                line,
                col,
            });
            return serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string());
        }
    };

    match lower_static_with_source(&file, Some(source)) {
        Ok(ir) => {
            // Lowering succeeded — collect validation warnings with their source spans.
            // アイテムに紐付く警告は source_span（1-based line/col_start）を反映し、
            // range 整合性など紐付かない警告は 0/0 のまま。
            for diag in tdsl_core::validate::validate_with_spans(&ir) {
                let (line, col) = diag.span.map_or((0, 0), |span| (span.line, span.col_start));
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: diag.message,
                    line,
                    col,
                });
            }
        }
        Err(errors) => {
            // LoweringError は現状ソース span を保持しないため位置は 0/0。
            for e in errors {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    message: e.to_string(),
                    line: 0,
                    col: 0,
                });
            }
        }
    }

    serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string())
}
