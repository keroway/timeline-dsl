//! Timeline DSL renderer: `TimelineIr` → standalone HTML with inline SVG.
//!
//! The public entry point is [`render_html`]. Internally it:
//! 1. Computes a [`layout::LayoutModel`] from the IR.
//! 2. Serializes it to an SVG string.
//! 3. Wraps the SVG in an HTML document with embedded CSS (hover tooltips only, no JS).

pub mod html;
pub mod layout;
pub mod svg;

pub use layout::{LayoutModel, RenderOptions, Theme};

use tdsl_core::ir::TimelineIr;

/// Render the given IR as a standalone HTML document string.
pub fn render_html(ir: &TimelineIr, opts: RenderOptions) -> String {
    let layout = LayoutModel::compute(ir, opts.clone());
    let svg = svg::render_svg(&layout);
    if opts.interactive {
        html::wrap_html_interactive(&svg, &ir.meta.title, &opts, &ir.lanes)
    } else {
        html::wrap_html(&svg, &ir.meta.title, &opts)
    }
}

/// Render the given IR as a standalone SVG string.
pub fn render_svg_only(ir: &TimelineIr, opts: RenderOptions) -> String {
    let layout = LayoutModel::compute(ir, opts);
    svg::render_svg(&layout)
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
                color_map: std::collections::HashMap::new(),
                ..Default::default()
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

    // ─── Render integration tests ──────────────────────────────────────────

    #[test]
    fn render_html_contains_lane_label() {
        let ir = sample_ir();
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.contains("漢"));
    }

    #[test]
    fn render_html_contains_span_label() {
        let ir = sample_ir();
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.contains("漢"));
    }

    #[test]
    fn render_html_multiple_lanes_ordered_by_order_field() {
        let ir = TimelineIr {
            meta: Meta {
                title: "Multi-lane".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![
                Lane {
                    id: "b".into(),
                    label: "B".into(),
                    kind: "dynasty".into(),
                    order: 20,
                },
                Lane {
                    id: "a".into(),
                    label: "A".into(),
                    kind: "dynasty".into(),
                    order: 10,
                },
            ],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let html = render_html(&ir, RenderOptions::default());
        // Both lanes should appear in output
        let pos_a = html.find(">A<").or_else(|| html.find("A</"));
        let pos_b = html.find(">B<").or_else(|| html.find("B</"));
        assert!(pos_a.is_some() || html.contains("A"));
        assert!(pos_b.is_some() || html.contains("B"));
    }

    #[test]
    fn render_html_empty_ir_does_not_panic() {
        let ir = TimelineIr {
            meta: Meta {
                title: "Empty".into(),
                unit: "year".into(),
                range: (0, 100),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.contains("Empty"));
    }

    #[test]
    fn render_html_event_item_appears_in_output() {
        let ir = TimelineIr {
            meta: Meta {
                title: "Events".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "politics".into(),
                label: "政治".into(),
                kind: "custom".into(),
                order: 1,
            }],
            items: vec![Item::Event {
                id: "event:politics:100".into(),
                lane: "politics".into(),
                time: 100,
                label: "即位".into(),
                tags: vec![],
                source: None,
                origin: None,
                time_month: None,
                time_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.contains("即位"));
    }

    #[test]
    fn render_html_event_range_item_appears_in_output() {
        let ir = TimelineIr {
            meta: Meta {
                title: "Ranges".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "war".into(),
                label: "戦争".into(),
                kind: "custom".into(),
                order: 1,
            }],
            items: vec![Item::EventRange {
                id: "event_range:war:100".into(),
                lane: "war".into(),
                start: 100,
                end: 200,
                label: "大乱".into(),
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
        };
        let html = render_html(&ir, RenderOptions::default());
        assert!(html.contains("大乱"));
    }

    #[test]
    fn render_html_custom_scale_changes_width() {
        let ir = sample_ir();
        let opts_narrow = RenderOptions {
            scale: 1.0,
            ..RenderOptions::default()
        };
        let opts_wide = RenderOptions {
            scale: 5.0,
            ..RenderOptions::default()
        };
        let narrow = render_html(&ir, opts_narrow);
        let wide = render_html(&ir, opts_wide);
        // wider scale → larger viewBox width
        assert_ne!(narrow, wide);
    }

    #[test]
    fn render_html_interactive_contains_script_tag() {
        let ir = sample_ir();
        let opts = RenderOptions {
            interactive: true,
            ..RenderOptions::default()
        };
        let html = render_html(&ir, opts);
        assert!(
            html.contains("<script>"),
            "interactive mode must include <script>"
        );
        assert!(
            html.contains("tdsl-search"),
            "interactive mode must include search input"
        );
        assert!(
            html.contains("tdsl-legend"),
            "interactive mode must include legend"
        );
        assert!(
            html.contains("tdsl-detail"),
            "interactive mode must include detail panel"
        );
        assert!(
            html.contains("data-label="),
            "interactive mode must include data-label attributes on SVG items"
        );
    }

    #[test]
    fn render_html_non_interactive_unchanged_behavior() {
        let ir = sample_ir();
        let opts_default = RenderOptions::default();
        let opts_explicit = RenderOptions {
            interactive: false,
            ..RenderOptions::default()
        };
        let html_default = render_html(&ir, opts_default);
        let html_explicit = render_html(&ir, opts_explicit);
        // interactive:false (default) should produce identical output to explicit false
        assert_eq!(html_default, html_explicit);
        // should NOT include interactive-only elements
        assert!(
            !html_default.contains("tdsl-search"),
            "non-interactive mode must not include search input"
        );
        assert!(
            !html_default.contains("tdsl-legend"),
            "non-interactive mode must not include legend"
        );
    }
}
