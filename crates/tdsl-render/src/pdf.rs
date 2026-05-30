//! Vector PDF output for the timeline SVG.
//!
//! Converts the in-memory SVG produced by [`crate::render_svg_only`] into a PDF
//! byte buffer using `svg2pdf` / `usvg`. System fonts are loaded so that CJK
//! lane labels render correctly on machines that have Noto Sans JP, Hiragino
//! Sans, Yu Gothic, etc. installed.
//!
//! This module is only compiled when the `pdf` Cargo feature is enabled. The
//! feature is opt-in to keep the `tdsl-wasm` build slim — the WASM crate
//! depends on `tdsl-render` without the feature.
//!
//! See ADR-0002 for the rationale behind using `svg2pdf` and the version
//! coupling requirement with `usvg`.

use svg2pdf::usvg::{Options, Tree};
use tdsl_core::ir::TimelineIr;
use thiserror::Error;

use crate::layout::{LayoutModel, RenderOptions};
use crate::svg;

/// Errors that can occur while converting the timeline SVG to a PDF.
#[derive(Debug, Error)]
pub enum PdfError {
    #[error("failed to parse intermediate SVG: {0}")]
    Parse(#[from] svg2pdf::usvg::Error),
    #[error("failed to convert SVG to PDF: {0}")]
    Convert(String),
}

/// Options for PDF output.
///
/// The initial version intentionally has no fields. Future versions may add
/// page size, margin, metadata embedding options, etc. (see ADR-0002).
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {}

/// Render the timeline IR to a vector PDF byte buffer using the given options.
///
/// Internally this:
/// 1. Computes the layout via [`LayoutModel::compute`].
/// 2. Serializes to an SVG string via [`svg::render_svg`].
/// 3. Parses the SVG with `usvg`, loading system fonts so CJK labels can be
///    shaped (ADR-0002 D5).
/// 4. Converts the `usvg::Tree` to a PDF byte buffer via `svg2pdf::to_pdf`.
pub fn render_pdf(
    ir: &TimelineIr,
    opts: RenderOptions,
    pdf_opts: PdfOptions,
) -> Result<Vec<u8>, PdfError> {
    let layout = LayoutModel::compute(ir, opts);
    let svg_str = svg::render_svg(&layout);
    svg_to_pdf(&svg_str, pdf_opts)
}

/// Convert a pre-rendered SVG string to a vector PDF byte buffer.
///
/// Exposed separately so callers that already hold an SVG string (e.g. tests,
/// alternative pipelines) don't need to re-run layout.
pub fn svg_to_pdf(svg_str: &str, _pdf_opts: PdfOptions) -> Result<Vec<u8>, PdfError> {
    let mut opt = Options::default();
    // Load system fonts so CJK lane labels (Noto Sans JP, Hiragino Sans,
    // Yu Gothic, …) are resolved correctly — same strategy as png.rs.
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_str(svg_str, &opt)?;

    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| PdfError::Convert(e.to_string()))
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
                group: None,
                source_span: None,
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

    /// PDF file signature: %PDF-
    const PDF_SIGNATURE: &[u8] = &[0x25, 0x50, 0x44, 0x46, 0x2D];

    #[test]
    fn render_pdf_produces_valid_pdf_bytes() {
        let ir = sample_ir();
        let bytes = render_pdf(&ir, RenderOptions::default(), PdfOptions::default())
            .expect("render_pdf succeeds");
        assert!(
            bytes.starts_with(PDF_SIGNATURE),
            "output should start with the PDF signature %%PDF-, got first 5 bytes = {:?}",
            &bytes[..bytes.len().min(5)]
        );
        assert!(
            bytes.len() > 100,
            "PDF output should be larger than the bare signature, got {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn render_pdf_empty_ir_does_not_panic() {
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
        let bytes = render_pdf(&ir, RenderOptions::default(), PdfOptions::default())
            .expect("render_pdf on empty IR succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
    }

    #[test]
    fn svg_to_pdf_invalid_svg_returns_parse_error() {
        let err =
            svg_to_pdf("not-an-svg", PdfOptions::default()).expect_err("invalid SVG must error");
        assert!(
            matches!(err, PdfError::Parse(_)),
            "expected PdfError::Parse, got: {err}"
        );
    }

    #[test]
    fn render_pdf_cjk_lane_label_does_not_panic() {
        // CJK lane label "漢" in an IR with a span — verifies that system font
        // loading is attempted and the PDF is produced without panic.
        let ir = sample_ir();
        let bytes = render_pdf(&ir, RenderOptions::default(), PdfOptions::default())
            .expect("render_pdf with CJK label succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert!(bytes.len() > 1000, "PDF should be non-trivially sized");
    }
}
