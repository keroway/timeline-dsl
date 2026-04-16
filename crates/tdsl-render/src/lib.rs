//! Timeline DSL renderer: `TimelineIr` → standalone HTML with inline SVG.
//!
//! The public entry point is [`render_html`]. Internally it:
//! 1. Computes a [`LayoutModel`](layout::LayoutModel) from the IR.
//! 2. Serializes it to an SVG string.
//! 3. Wraps the SVG in an HTML document with embedded CSS (hover tooltips only, no JS).

pub mod html;
pub mod layout;
pub mod svg;

pub use layout::{LayoutModel, RenderOptions};

use tdsl_core::ir::TimelineIr;

/// Render the given IR as a standalone HTML document string.
pub fn render_html(ir: &TimelineIr, opts: RenderOptions) -> String {
    let layout = LayoutModel::compute(ir, opts);
    let svg = svg::render_svg(&layout);
    html::wrap_html(&svg, &ir.meta.title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

    fn sample_ir() -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "サンプル年表".into(),
                unit: "year".into(),
                range: (-300, 300),
                calendar: "proleptic_gregorian".into(),
            },
            lanes: vec![Lane {
                id: "han".into(),
                label: "漢".into(),
                kind: "dynasty".into(),
                order: 10,
            }],
            items: vec![Item::Span {
                id: "span:han".into(),
                lane: "han".into(),
                start: -206,
                end: 220,
                label: "漢".into(),
                tags: vec!["dynasty".into()],
                source: Some("wd:Q7209".into()),
                origin: None,
            }],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn render_html_produces_complete_document() {
        let ir = sample_ir();
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<svg"));
        assert!(html.contains("</svg>"));
        assert!(html.contains("サンプル年表"));
        assert!(html.ends_with("</html>\n") || html.ends_with("</html>"));
    }
}
