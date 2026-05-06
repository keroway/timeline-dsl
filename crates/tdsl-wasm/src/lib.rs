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

/// Render SVG from TDSL source (static items only).
/// Returns Ok(svg_string) or Err(error_message).
#[wasm_bindgen]
pub fn render_svg_from_source(source: &str) -> Result<String, JsValue> {
    let file = tdsl_parser::parse(source).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ir = lower_static(&file).map_err(|errors| {
        let msgs: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        JsValue::from_str(&msgs.join("\n"))
    })?;
    let svg = render_svg_only(&ir, RenderOptions::default());
    Ok(svg)
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
