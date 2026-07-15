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
    #[error("SVG formatting failed: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("failed to parse intermediate SVG: {0}")]
    Parse(#[from] resvg::usvg::Error),
    #[error("failed to allocate pixmap of size {width}x{height}")]
    PixmapAlloc { width: u32, height: u32 },
    #[error("failed to encode PNG: {0}")]
    Encode(String),
}

/// Resolution and scale options for PNG rasterization.
///
/// SVG user units are defined at 96 DPI. Setting `dpi` to a higher value
/// increases the output pixel dimensions proportionally. Alternatively,
/// `scale_factor` overrides the DPI calculation with a direct multiplier.
/// Specifying both is a logic error — the CLI prevents it with `conflicts_with`.
#[derive(Debug, Clone)]
pub struct PngOptions {
    /// Output DPI (default 96). The pixel scale is computed as `dpi / 96.0`.
    pub dpi: u32,
    /// Fixed pixel scale multiplier. When `Some`, overrides `dpi`.
    pub scale_factor: Option<f64>,
}

impl Default for PngOptions {
    fn default() -> Self {
        Self {
            dpi: 96,
            scale_factor: None,
        }
    }
}

impl PngOptions {
    fn pixel_scale(&self) -> f64 {
        if let Some(sf) = self.scale_factor {
            sf
        } else {
            self.dpi as f64 / 96.0
        }
    }
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
pub fn render_png(
    ir: &TimelineIr,
    mut opts: RenderOptions,
    png_opts: PngOptions,
) -> Result<Vec<u8>, PngError> {
    // usvg does not support CSS custom properties; force plain hex lane colours.
    opts.use_css_vars = false;
    let layout = LayoutModel::compute(ir, opts);
    let svg_str = svg::render_svg(&layout)?;
    svg_to_png(&svg_str, png_opts)
}

/// Convert a pre-rendered SVG string to PNG bytes.
///
/// Exposed separately so callers that already hold an SVG string (e.g. tests,
/// alternative pipelines) don't need to re-run layout.
pub fn svg_to_png(svg_str: &str, png_opts: PngOptions) -> Result<Vec<u8>, PngError> {
    let factor = png_opts.pixel_scale();
    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();

    // Resolve CSS custom property var(--tdsl-lane-N, #hex) → #hex so that usvg
    // (which does not support CSS variables) renders the correct lane colours.
    let resolved = svg::resolve_lane_vars_in_styles(svg_str);
    let tree = Tree::from_data(resolved.as_bytes(), &opt)?;
    let size = tree.size().to_int_size();
    let base_width = size.width();
    let base_height = size.height();
    let width = ((base_width as f64 * factor).round() as u32).max(1);
    let height = ((base_height as f64 * factor).round() as u32).max(1);
    let mut pixmap = Pixmap::new(width, height).ok_or(PngError::PixmapAlloc { width, height })?;
    let transform = Transform::from_scale(factor as f32, factor as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
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
                note: None,
                link: None,
                color: None,
                start_month: None,
                start_day: None,
                start_hour: None,
                start_minute: None,
                start_second: None,
                start_offset_minutes: None,
                end_month: None,
                end_day: None,
                end_hour: None,
                end_minute: None,
                end_second: None,
                end_offset_minutes: None,
                end_open: false,
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
        let bytes = render_png(&ir, RenderOptions::default(), PngOptions::default())
            .expect("render_png succeeds");
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
    fn render_png_show_table_produces_valid_png_bytes() {
        // #536: show_table must work for PNG output too (rendered SVG table is
        // rasterized like the rest of the timeline; no separate handling needed).
        let ir = sample_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let bytes = render_png(&ir, opts, PngOptions::default())
            .expect("render_png succeeds with show_table");
        assert!(bytes.starts_with(PNG_SIGNATURE));
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
        let bytes = render_png(&ir, RenderOptions::default(), PngOptions::default())
            .expect("render_png succeeds");
        assert!(bytes.starts_with(PNG_SIGNATURE));
    }

    #[test]
    fn svg_to_png_invalid_svg_returns_parse_error() {
        let err =
            svg_to_png("not-an-svg", PngOptions::default()).expect_err("invalid SVG must error");
        assert!(matches!(err, PngError::Parse(_)));
    }

    #[test]
    fn png_dpi_300_produces_larger_output_than_default() {
        let ir = sample_ir();
        let default_bytes = render_png(&ir, RenderOptions::default(), PngOptions::default())
            .expect("default render_png succeeds");
        let hires_bytes = render_png(
            &ir,
            RenderOptions::default(),
            PngOptions {
                dpi: 300,
                scale_factor: None,
            },
        )
        .expect("300 DPI render_png succeeds");
        assert!(
            hires_bytes.len() > default_bytes.len(),
            "300 DPI PNG ({} bytes) should be larger than default 96 DPI PNG ({} bytes)",
            hires_bytes.len(),
            default_bytes.len()
        );
        assert!(hires_bytes.starts_with(PNG_SIGNATURE));
    }

    #[test]
    fn png_scale_factor_produces_larger_output_than_default() {
        let ir = sample_ir();
        let default_bytes = render_png(&ir, RenderOptions::default(), PngOptions::default())
            .expect("default render_png succeeds");
        let scaled_bytes = render_png(
            &ir,
            RenderOptions::default(),
            PngOptions {
                dpi: 96,
                scale_factor: Some(2.0),
            },
        )
        .expect("2x scale render_png succeeds");
        assert!(
            scaled_bytes.len() > default_bytes.len(),
            "2x scale PNG ({} bytes) should be larger than default PNG ({} bytes)",
            scaled_bytes.len(),
            default_bytes.len()
        );
        assert!(scaled_bytes.starts_with(PNG_SIGNATURE));
    }
}
