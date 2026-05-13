use wasm_bindgen::prelude::*;

use tdsl_core::lower::lower_static;
use tdsl_render::{RenderOptions, render_html, render_svg_only};

/// Initialize the panic hook for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Compile TDSL source to IR (JSON string), without Wikidata resolution.
/// Returns Ok(json_string) or Err(error_message).
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static(&file).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    serde_json::to_string_pretty(&ir)
        .map_err(|e| JsValue::from_str(&e.to_string()))
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
/// Returns Ok(svg_string) or Err(error_message).
#[wasm_bindgen]
pub fn render_svg_from_source(source: &str, scale: f64) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static(&file).map_err(|errors| {
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
    let svg = render_svg_only(&ir, RenderOptions { scale: computed_scale, ..RenderOptions::default() });
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
            },
            lanes: vec![Lane {
                id: "a".into(),
                label: "A".into(),
                kind: "custom".into(),
                order: 1,
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
        let svg = render_svg_only(&ir, RenderOptions {
            scale: auto_scale_for_span(12.0),
            ..RenderOptions::default()
        });
        let w = extract_svg_width(&svg);
        assert!(w >= 700.0, "span=12 SVG width should be ≥700px, got {w}");
    }

    #[test]
    fn svg_width_for_medium_range_unchanged() {
        // span=125, scale=8 → width = 120 + 125*8 + 20 = 1140px
        let ir = make_ir(1900, 2025);
        let svg = render_svg_only(&ir, RenderOptions {
            scale: auto_scale_for_span(125.0),
            ..RenderOptions::default()
        });
        let w = extract_svg_width(&svg);
        assert!((w - 1140.0).abs() < 1.0, "span=125 width should be ~1140px, got {w}");
    }

    #[test]
    fn svg_width_for_long_range_uses_lower_clamp() {
        // span=5000, scale=0.5 → width = 120 + 5000*0.5 + 20 = 2640px
        let ir = make_ir(-3000, 2000);
        let svg = render_svg_only(&ir, RenderOptions {
            scale: auto_scale_for_span(5000.0),
            ..RenderOptions::default()
        });
        let w = extract_svg_width(&svg);
        assert!((w - 2640.0).abs() < 1.0, "span=5000 width should be ~2640px, got {w}");
    }
}

/// Render standalone HTML from TDSL source (static items only).
/// Returns Ok(html_string) or Err(error_message).
#[wasm_bindgen]
pub fn render_html_from_source(source: &str) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static(&file).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    Ok(render_html(&ir, RenderOptions::default()))
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
/// `severity` is `"error"` or `"warning"`. `line`/`col` are 0-indexed.
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
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: e.to_string(),
                line: 0,
                col: 0,
            });
            return serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string());
        }
    };

    match lower_static(&file) {
        Ok(ir) => {
            // Lowering succeeded — collect validation warnings
            let warnings = tdsl_core::validate::validate(&ir);
            for w in warnings {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: w,
                    line: 0,
                    col: 0,
                });
            }
        }
        Err(errors) => {
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
