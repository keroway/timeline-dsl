//! PNG rasterization for the timeline SVG.
//!
//! Converts the in-memory SVG produced by [`crate::render_svg_only`] into a PNG
//! byte buffer using `resvg` / `usvg` / `tiny-skia`. System fonts are loaded so
//! that CJK lane labels render correctly on machines that have Noto Sans JP,
//! Hiragino Sans, Yu Gothic, etc. installed.
//!
//! This module is only compiled when the `png` Cargo feature is enabled. The
//! feature is opt-in to keep the `tdsl-wasm` build slim — the WASM crate
//! depends on `tdsl-render` without the feature.

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{Options, Tree};
use tdsl_core::ir::TimelineIr;
use thiserror::Error;

use crate::layout::{LayoutModel, RenderOptions};
use crate::svg;

/// Errors that can occur while rasterizing the timeline SVG to PNG.
#[derive(Debug, Error)]
pub enum PngError {
    #[error("failed to parse intermediate SVG: {0}")]
    Parse(#[from] resvg::usvg::Error),
    #[error("failed to allocate pixmap of size {width}x{height}")]
    PixmapAlloc { width: u32, height: u32 },
    #[error("failed to encode PNG: {0}")]
    Encode(String),
}

/// Render the timeline IR to PNG bytes using the given options.
///
/// Internally this:
/// 1. Computes the layout via [`LayoutModel::compute`].
/// 2. Serializes to an SVG string via [`svg::render_svg`].
/// 3. Parses the SVG with `usvg`, loading system fonts so CJK labels can be
///    shaped.
/// 4. Rasterizes through `resvg` into a `tiny_skia::Pixmap`.
/// 5. Encodes the pixmap as a PNG byte buffer.
pub fn render_png(ir: &TimelineIr, opts: RenderOptions) -> Result<Vec<u8>, PngError> {
    let layout = LayoutModel::compute(ir, opts);
    let svg_str = svg::render_svg(&layout);
    svg_to_png(&svg_str)
}

/// Convert a pre-rendered SVG string to PNG bytes.
///
/// Exposed separately so callers that already hold an SVG string (e.g. tests,
/// alternative pipelines) don't need to re-run layout.
pub fn svg_to_png(svg_str: &str) -> Result<Vec<u8>, PngError> {
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = Tree::from_data(svg_str.as_bytes(), &opt)?;
    let size = tree.size().to_int_size();
    let width = size.width();
    let height = size.height();
    let mut pixmap = Pixmap::new(width, height).ok_or(PngError::PixmapAlloc { width, height })?;
    resvg::render(&tree, Transform::default(), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| PngError::Encode(e.to_string()))
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

    /// PNG file signature: 89 50 4E 47 0D 0A 1A 0A
    const PNG_SIGNATURE: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn render_png_produces_valid_png_bytes() {
        let ir = sample_ir();
        let bytes = render_png(&ir, RenderOptions::default()).expect("render_png succeeds");
        assert!(
            bytes.starts_with(PNG_SIGNATURE),
            "output should start with the PNG signature, got first 8 bytes = {:?}",
            &bytes[..bytes.len().min(8)]
        );
        assert!(
            bytes.len() > 100,
            "PNG output should be larger than the bare signature, got {} bytes",
            bytes.len()
        );
    }

    #[test]
    fn render_png_empty_ir_does_not_panic() {
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
        let bytes = render_png(&ir, RenderOptions::default()).expect("render_png succeeds");
        assert!(bytes.starts_with(PNG_SIGNATURE));
    }

    #[test]
    fn svg_to_png_invalid_svg_returns_parse_error() {
        let err = svg_to_png("not-an-svg").expect_err("invalid SVG must error");
        assert!(matches!(err, PngError::Parse(_)));
    }
}
