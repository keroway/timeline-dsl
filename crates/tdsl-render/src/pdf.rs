//! Vector PDF output for the timeline SVG.
//!
//! Converts the in-memory SVG produced by [`crate::render_svg_only`] into a PDF
//! byte buffer using `svg2pdf` / `usvg` and `pdf-writer`. System fonts are
//! loaded so that CJK lane labels render correctly on machines that have Noto
//! Sans JP, Hiragino Sans, Yu Gothic, etc. installed.
//!
//! This module is only compiled when the `pdf` Cargo feature is enabled. The
//! feature is opt-in to keep the `tdsl-wasm` build slim — the WASM crate
//! depends on `tdsl-render` without the feature.
//!
//! See ADR-0002 for the rationale behind using `svg2pdf` and the version
//! coupling requirement with `usvg`.

use std::collections::HashMap;

use pdf_writer::{Content, Finish, Name, Pdf, Ref, TextStr};
use svg2pdf::usvg::{Options, Tree};
use tdsl_core::ir::TimelineIr;
use thiserror::Error;

use crate::layout::{LayoutModel, RenderOptions};
use crate::svg;

/// Errors that can occur while converting the timeline SVG to a PDF.
#[derive(Debug, Error)]
pub enum PdfError {
    #[error("SVG formatting failed: {0}")]
    Fmt(#[from] std::fmt::Error),
    #[error("failed to parse intermediate SVG: {0}")]
    Parse(#[from] svg2pdf::usvg::Error),
    #[error("failed to convert SVG to PDF: {0}")]
    Convert(String),
    #[error("invalid PDF margin: {0}")]
    InvalidMargin(String),
}

/// Standard page sizes for PDF output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PdfPageSize {
    /// ISO A4 — 210 × 297 mm
    #[default]
    A4,
    /// ISO A3 — 297 × 420 mm
    A3,
    /// US Letter — 8.5 × 11 in
    Letter,
}

impl PdfPageSize {
    /// Returns (width, height) in PDF points for portrait orientation.
    fn portrait_pt(self) -> (f32, f32) {
        match self {
            PdfPageSize::A4 => (595.276, 841.890),  // 210×297 mm
            PdfPageSize::A3 => (841.890, 1190.551), // 297×420 mm
            PdfPageSize::Letter => (612.0, 792.0),  // 8.5×11 in
        }
    }

    /// Human-readable page size name for diagnostics.
    fn name(self) -> &'static str {
        match self {
            PdfPageSize::A4 => "A4",
            PdfPageSize::A3 => "A3",
            PdfPageSize::Letter => "Letter",
        }
    }
}

/// A calendar date for use as PDF CreationDate metadata.
///
/// The caller supplies this value to keep `tdsl-render` clock-free and
/// ensure deterministic output. Typically obtained from
/// `std::time::SystemTime::now()` at the CLI entry point.
#[derive(Debug, Clone, Copy)]
pub struct PdfDate {
    /// Four-digit year (e.g. 2026).
    pub year: u16,
    /// Month 1–12.
    pub month: u8,
    /// Day-of-month 1–31.
    pub day: u8,
}

/// Options for PDF output.
///
/// Controls page size, orientation, margin, and document metadata.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// Output page size. Defaults to [`PdfPageSize::A4`].
    pub page_size: PdfPageSize,
    /// When `true` the page is rotated 90° (landscape).
    pub landscape: bool,
    /// Page margin on all four sides in millimetres. Defaults to `10.0`.
    pub margin_mm: f64,
    /// PDF Title metadata. When `None`, [`render_pdf`] fills in
    /// `ir.meta.title` automatically.
    pub title: Option<String>,
    /// PDF CreationDate metadata. When `None` the CreationDate entry is
    /// omitted from the document information dictionary.
    pub creation_date: Option<PdfDate>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_size: PdfPageSize::default(),
            landscape: false,
            margin_mm: 10.0,
            title: None,
            creation_date: None,
        }
    }
}

/// Render the timeline IR to a vector PDF byte buffer using the given options.
///
/// Internally this:
/// 1. Computes the layout via [`LayoutModel::compute`].
/// 2. Serializes to an SVG string via [`svg::render_svg`].
/// 3. Fills in `pdf_opts.title` from `ir.meta.title` when not already set.
/// 4. Converts the SVG to a PDF byte buffer via [`svg_to_pdf`].
pub fn render_pdf(
    ir: &TimelineIr,
    mut opts: RenderOptions,
    mut pdf_opts: PdfOptions,
) -> Result<Vec<u8>, PdfError> {
    // usvg does not support CSS custom properties; force plain hex lane colours.
    opts.use_css_vars = false;
    let layout = LayoutModel::compute(ir, opts);
    let svg_str = svg::render_svg(&layout)?;

    // Supplement title from IR metadata when the caller did not override it.
    if pdf_opts.title.is_none() && !ir.meta.title.is_empty() {
        pdf_opts.title = Some(ir.meta.title.clone());
    }

    svg_to_pdf(&svg_str, pdf_opts)
}

/// Convert a pre-rendered SVG string to a vector PDF byte buffer.
///
/// Exposed separately so callers that already hold an SVG string (e.g. tests,
/// alternative pipelines) don't need to re-run layout.
///
/// ## Layout
/// The SVG is fit (maintaining aspect ratio) inside the content area defined
/// by `pdf_opts.page_size` minus `pdf_opts.margin_mm` on each side. The
/// fitted graphic is centred within the content area. PDF coordinates use
/// the lower-left origin.
pub fn svg_to_pdf(svg_str: &str, pdf_opts: PdfOptions) -> Result<Vec<u8>, PdfError> {
    let mut opt = Options::default();
    // Load system fonts so CJK lane labels (Noto Sans JP, Hiragino Sans,
    // Yu Gothic, …) are resolved correctly — same strategy as png.rs.
    opt.fontdb_mut().load_system_fonts();

    // Resolve CSS custom property var(--tdsl-lane-N, #hex) → #hex so that usvg
    // (which does not support CSS variables) renders the correct lane colours.
    let resolved = svg::resolve_lane_vars_in_styles(svg_str);
    let tree = Tree::from_str(&resolved, &opt)?;

    // ── 1. Determine page dimensions ──────────────────────────────────────
    let (mut pw, mut ph) = pdf_opts.page_size.portrait_pt();
    if pdf_opts.landscape {
        std::mem::swap(&mut pw, &mut ph);
    }

    // ── 2. Compute content area after margins ──────────────────────────────
    // Reject margins that are not a sensible physical length. A negative or
    // non-finite margin would push the drawing off the page (malformed PDF), so
    // fail explicitly rather than silently producing broken output.
    if !pdf_opts.margin_mm.is_finite() || pdf_opts.margin_mm < 0.0 {
        return Err(PdfError::InvalidMargin(format!(
            "margin must be a non-negative, finite number of millimetres, got {}",
            pdf_opts.margin_mm
        )));
    }
    // 1 inch = 72pt; 1 mm = 72/25.4 pt ≈ 2.8346 pt
    let margin = (pdf_opts.margin_mm * 72.0 / 25.4) as f32;
    // A margin that consumes the whole page leaves no printable area, producing
    // a blank PDF. Require a positive content area on both axes and fail loudly
    // otherwise (the smaller page dimension is the binding constraint).
    if 2.0 * margin >= pw.min(ph) {
        let orientation = if pdf_opts.landscape {
            "landscape"
        } else {
            "portrait"
        };
        return Err(PdfError::InvalidMargin(format!(
            "margin {} mm is too large for the {} {} page; it leaves no printable area",
            pdf_opts.margin_mm,
            pdf_opts.page_size.name(),
            orientation,
        )));
    }
    let content_w = pw - 2.0 * margin;
    let content_h = ph - 2.0 * margin;

    // ── 3. Scale SVG to fit, preserving aspect ratio ───────────────────────
    let svg_size = tree.size();
    let svg_w = svg_size.width();
    let svg_h = svg_size.height();

    let scale = (content_w / svg_w).min(content_h / svg_h);
    let draw_w = svg_w * scale;
    let draw_h = svg_h * scale;

    // Centre the drawing within the content area.
    // PDF origin is bottom-left; margins are symmetric so both axes work out
    // to the same formula.
    let tx = margin + (content_w - draw_w) / 2.0;
    let ty = margin + (content_h - draw_h) / 2.0;

    // ── 4. Convert SVG to a pdf-writer Chunk (1pt × 1pt XObject) ──────────
    let (svg_chunk_raw, svg_old_id) =
        svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default())
            .map_err(|e| PdfError::Convert(e.to_string()))?;

    // ── 5. Allocate PDF indirect object IDs ───────────────────────────────
    // We need 5 fixed IDs before renumbering the SVG chunk.
    let mut alloc = Ref::new(1);
    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();
    let page_id = alloc.bump();
    let content_id = alloc.bump();
    let info_id = alloc.bump();

    // Renumber the SVG chunk so its internal refs don't collide.
    let mut id_map: HashMap<Ref, Ref> = HashMap::new();
    let svg_chunk =
        svg_chunk_raw.renumber(|old| *id_map.entry(old).or_insert_with(|| alloc.bump()));
    let svg_id = *id_map
        .get(&svg_old_id)
        .ok_or_else(|| PdfError::Convert("svg chunk renumber: XObject ID not found".to_string()))?;

    let svg_name = Name(b"S1");

    // ── 6. Build the PDF ───────────────────────────────────────────────────
    let mut pdf = Pdf::new();

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id).kids([page_id]).count(1);

    // Page object
    {
        let mut page = pdf.page(page_id);
        page.media_box(pdf_writer::Rect::new(0.0, 0.0, pw, ph));
        page.parent(page_tree_id);
        page.contents(content_id);
        let mut resources = page.resources();
        resources.x_objects().pair(svg_name, svg_id);
        resources.finish();
        page.finish();
    }

    // Content stream: place the SVG XObject
    // The SVG XObject occupies a 1pt × 1pt space; the transform matrix
    // scales it to draw_w × draw_h and translates it to (tx, ty).
    let mut content = Content::new();
    content
        .save_state()
        .transform([draw_w, 0.0, 0.0, draw_h, tx, ty])
        .x_object(svg_name)
        .restore_state();
    pdf.stream(content_id, &content.finish());

    // Embed the SVG chunk (contains the XObject and all sub-resources)
    pdf.extend(&svg_chunk);

    // Document information dictionary
    {
        let mut info = pdf.document_info(info_id);
        info.producer(TextStr("tdsl (svg2pdf)"));
        if let Some(ref title) = pdf_opts.title {
            info.title(TextStr(title.as_str()));
        }
        if let Some(d) = pdf_opts.creation_date {
            info.creation_date(pdf_writer::Date::new(d.year).month(d.month).day(d.day));
        }
        info.finish();
    }

    Ok(pdf.finish())
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
                start_hour: None,
                start_minute: None,
                end_month: None,
                end_day: None,
                end_hour: None,
                end_minute: None,
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
    fn render_pdf_show_table_produces_valid_single_page_pdf() {
        // #536: show_table must work for PDF output too, embedded in the same
        // single-page vector document (no page-split logic exists yet; see
        // docs/dsl-spec.md 「PDF出力のページ方針」 for the documented rationale).
        let ir = sample_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let bytes = render_pdf(&ir, opts, PdfOptions::default())
            .expect("render_pdf succeeds with show_table");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        // Exactly one /Type /Page object (excluding the /Type /Pages parent, which
        // shares the "/Type/Page" prefix as a substring).
        let text = String::from_utf8_lossy(&bytes);
        let page_count = text
            .matches("/Type/Page")
            .filter(|_| true)
            .collect::<Vec<_>>()
            .len()
            + text.matches("/Type /Page").collect::<Vec<_>>().len();
        let pages_count =
            text.matches("/Type/Pages").count() + text.matches("/Type /Pages").count();
        assert_eq!(
            page_count - pages_count,
            1,
            "show_table=true PDF output must remain a single page (no duplication)"
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

    // ─── New tests for page size, landscape, margin, and metadata ─────────

    #[test]
    fn pdf_a3_produces_valid_pdf() {
        let opts = PdfOptions {
            page_size: PdfPageSize::A3,
            ..PdfOptions::default()
        };
        let bytes = svg_to_pdf(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="blue"/></svg>"#,
            opts,
        )
        .expect("A3 PDF generation succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE), "A3 output must be a PDF");
        assert!(bytes.len() > 100);
        // A3 MediaBox differs from A4: check the byte stream contains a larger box
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/MediaBox"),
            "PDF must contain a /MediaBox entry"
        );
    }

    #[test]
    fn pdf_letter_produces_valid_pdf() {
        let opts = PdfOptions {
            page_size: PdfPageSize::Letter,
            ..PdfOptions::default()
        };
        let bytes = svg_to_pdf(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="red"/></svg>"#,
            opts,
        )
        .expect("Letter PDF generation succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
    }

    #[test]
    fn pdf_landscape_produces_valid_pdf() {
        let opts = PdfOptions {
            page_size: PdfPageSize::A4,
            landscape: true,
            ..PdfOptions::default()
        };
        let bytes = svg_to_pdf(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="200"><rect width="800" height="200" fill="green"/></svg>"#,
            opts,
        )
        .expect("landscape PDF generation succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
    }

    #[test]
    fn pdf_a4_and_a3_media_boxes_differ() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50"/></svg>"#;

        let bytes_a4 = svg_to_pdf(svg, PdfOptions::default()).expect("A4 PDF generation succeeds");
        let bytes_a3 = svg_to_pdf(
            svg,
            PdfOptions {
                page_size: PdfPageSize::A3,
                ..PdfOptions::default()
            },
        )
        .expect("A3 PDF generation succeeds");

        // Both must be valid PDFs.
        assert!(bytes_a4.starts_with(PDF_SIGNATURE));
        assert!(bytes_a3.starts_with(PDF_SIGNATURE));

        // A3 PDF should be different from A4 PDF (different MediaBox dimensions).
        assert_ne!(
            bytes_a4, bytes_a3,
            "A4 and A3 PDFs must differ (different page sizes)"
        );
    }

    const TINY_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;

    #[test]
    fn pdf_oversized_margin_returns_error() {
        // A margin larger than half the page leaves no printable area; this must
        // fail explicitly rather than emit a blank PDF.
        let opts = PdfOptions {
            margin_mm: 500.0,
            ..PdfOptions::default()
        };
        let err = svg_to_pdf(TINY_SVG, opts).expect_err("over-large margin must error");
        assert!(
            matches!(err, PdfError::InvalidMargin(_)),
            "expected PdfError::InvalidMargin, got: {err}"
        );
    }

    #[test]
    fn pdf_negative_margin_returns_error() {
        let opts = PdfOptions {
            margin_mm: -5.0,
            ..PdfOptions::default()
        };
        let err = svg_to_pdf(TINY_SVG, opts).expect_err("negative margin must error");
        assert!(
            matches!(err, PdfError::InvalidMargin(_)),
            "expected PdfError::InvalidMargin, got: {err}"
        );
    }

    #[test]
    fn pdf_non_finite_margin_returns_error() {
        let opts = PdfOptions {
            margin_mm: f64::NAN,
            ..PdfOptions::default()
        };
        let err = svg_to_pdf(TINY_SVG, opts).expect_err("NaN margin must error");
        assert!(matches!(err, PdfError::InvalidMargin(_)));
    }

    #[test]
    fn pdf_large_but_valid_margin_still_renders() {
        // 90 mm on each side still leaves a positive content area on A4
        // (210 mm wide) and must render successfully.
        let opts = PdfOptions {
            margin_mm: 90.0,
            ..PdfOptions::default()
        };
        let bytes = svg_to_pdf(TINY_SVG, opts).expect("valid large margin renders");
        assert!(bytes.starts_with(PDF_SIGNATURE));
    }

    #[test]
    fn pdf_with_title_and_creation_date_produces_valid_pdf() {
        let opts = PdfOptions {
            title: Some("My Timeline".to_string()),
            creation_date: Some(PdfDate {
                year: 2026,
                month: 6,
                day: 7,
            }),
            ..PdfOptions::default()
        };
        let bytes = svg_to_pdf(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="navy"/></svg>"#,
            opts,
        )
        .expect("PDF with metadata succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));

        // The PDF byte stream should contain the /CreationDate key.
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/CreationDate"),
            "PDF must contain /CreationDate when creation_date is set"
        );
    }

    #[test]
    fn render_pdf_title_is_filled_from_ir_meta_when_none() {
        let ir = sample_ir(); // title = "サンプル年表"
        let opts = PdfOptions {
            title: None, // not set — render_pdf should fill from ir.meta.title
            ..PdfOptions::default()
        };
        let bytes = render_pdf(&ir, RenderOptions::default(), opts).expect("render_pdf succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        // The title must appear somewhere in the document information dictionary.
        // pdf-writer encodes non-ASCII TextStr as hex (<FEFF...>), so we cannot
        // reliably grep for the UTF-8 literal. We assert that /Title entry was
        // written by checking for the /Title key in the byte stream.
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/Title"),
            "PDF must contain /Title when ir.meta.title is non-empty"
        );
    }
}
