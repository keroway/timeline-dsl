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

use crate::layout::{LayoutModel, RenderOptions, TABLE_ROW_HEIGHT, collect_table_rows};
use crate::pagination::{self, PaginationError};
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
    #[error("PDF pagination requires RenderOptions::show_table to be enabled")]
    PaginationRequiresTable,
    #[error("chart pagination failed: {0}")]
    ChartPagination(#[from] PaginationError),
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
    /// Split the item table onto separate PDF pages. This is opt-in; when
    /// disabled the historical single-page SVG-to-PDF path is retained.
    pub pagination: bool,
    /// Split the timeline chart into multiple PDF pages by lane group, `N`
    /// lanes per page (issue #661, following #660's SVG-only implementation
    /// in [`crate::pagination`]). `None` (the default) retains the historical
    /// single-chart-page behavior byte-for-byte, regardless of `pagination`.
    ///
    /// ## Page ordering when combined with `pagination`
    ///
    /// When both `chart_pagination` and `pagination` are set, the resulting
    /// PDF pages are ordered as: all chart pages (in lane-group order) first,
    /// followed by all table pages (in row-chunk order). Table page footers
    /// (`i / N`) count only the table pages, not the chart pages that precede
    /// them — this mirrors the pre-existing table-only pagination footer
    /// numbering and keeps it independent of how many chart pages exist.
    ///
    /// When `chart_pagination` is set but `pagination` is not, and
    /// `RenderOptions::show_table` is enabled, a single trailing table page
    /// (covering every IR item, unsplit) is appended after the chart pages —
    /// the chart can no longer share one SVG with the table once it is split
    /// into multiple pages, so this single table page takes over the role the
    /// combined chart+table SVG played in the non-chart-paginated case.
    pub chart_pagination: Option<usize>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            page_size: PdfPageSize::default(),
            landscape: false,
            margin_mm: 10.0,
            title: None,
            creation_date: None,
            pagination: false,
            chart_pagination: None,
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
    opts: RenderOptions,
    pdf_opts: PdfOptions,
) -> Result<Vec<u8>, PdfError> {
    let (bytes, _warnings) = render_pdf_with_warnings(ir, opts, pdf_opts)?;
    Ok(bytes)
}

/// Same as [`render_pdf`], but also returns diagnostic warnings that must not
/// be silently dropped (implementation-strict.md §1 "Explicit error over
/// silent fallback").
///
/// Currently the only warnings produced are `Lane::group` labels whose
/// contiguous lane run was split across chart pages by `pdf_opts.chart_pagination`
/// — the same diagnostic [`crate::pagination::paginate_svg_by_lane_groups`]
/// returns for the SVG-only pagination path (issue #660). When
/// `pdf_opts.chart_pagination` is `None` this always returns an empty `Vec`.
pub fn render_pdf_with_warnings(
    ir: &TimelineIr,
    opts: RenderOptions,
    mut pdf_opts: PdfOptions,
) -> Result<(Vec<u8>, Vec<String>), PdfError> {
    // Supplement title from IR metadata when the caller did not override it.
    if pdf_opts.title.is_none() && !ir.meta.title.is_empty() {
        pdf_opts.title = Some(ir.meta.title.clone());
    }

    let (pages, warnings) = render_pdf_svg_pages(ir, opts, &pdf_opts)?;
    let bytes = if pages.len() == 1 {
        svg_to_pdf(&pages[0], pdf_opts)?
    } else {
        svg_pages_to_pdf(&pages, pdf_opts)?
    };
    Ok((bytes, warnings))
}

/// Compute the raw per-page SVG strings that [`render_pdf`] would convert to a
/// PDF, without performing the SVG→PDF conversion itself, plus any chart
/// group-band split warnings (see [`render_pdf_with_warnings`]).
///
/// This is the single source of truth for the pagination branches (ADR-0004
/// D1/D2, ADR-0005/#661):
///
/// - `chart_pagination: None`, `pagination: false` — exactly one page (the
///   combined timeline+table SVG), byte-for-byte identical to the pre-#661
///   behavior (regression-tested).
/// - `chart_pagination: None`, `pagination: true` — `[timeline_page,
///   table_page_1, ..., table_page_N]`, where the timeline page is computed
///   identically to the non-paginated single page (`show_table` forced to
///   `false`, everything else unchanged) so that table pagination cannot
///   alter the timeline chart (ADR-0004 D5). Unchanged by #661.
/// - `chart_pagination: Some(n)` — `[chart_page_1, ..., chart_page_M,
///   table_page_1, ..., table_page_N]` (chart pages always precede table
///   pages). `N` is `0` when `RenderOptions::show_table` is disabled, `1`
///   when it is enabled but `pagination` is not, or the row-chunked count
///   from `pagination: true` otherwise. Table page footers count only the
///   table pages, matching the `chart_pagination: None` + `pagination: true`
///   numbering.
///
/// Exposed at `pub(crate)` so tests can assert on the exact page SVGs through
/// the real code path instead of re-implementing it.
fn render_pdf_svg_pages(
    ir: &TimelineIr,
    mut opts: RenderOptions,
    pdf_opts: &PdfOptions,
) -> Result<(Vec<String>, Vec<String>), PdfError> {
    // usvg does not support CSS custom properties; force plain hex lane colours.
    opts.use_css_vars = false;

    let Some(lanes_per_page) = pdf_opts.chart_pagination else {
        // ── No chart pagination: preserve the pre-#661 behavior verbatim ──
        if !pdf_opts.pagination {
            let layout = LayoutModel::compute(ir, opts);
            let svg_str = svg::render_svg(&layout)?;
            return Ok((vec![svg_str], vec![]));
        }
        if !opts.show_table {
            return Err(PdfError::PaginationRequiresTable);
        }

        // The chart remains a single, unmodified timeline page. Only the item
        // table is paginated, per ADR-0004 D1/D2.
        let lane_label_lookup = lane_label_lookup(ir);
        let table_rows = collect_table_rows(ir, lane_label_lookup);
        opts.show_table = false;
        let timeline_layout = LayoutModel::compute(ir, opts);
        let timeline_svg = svg::render_svg(&timeline_layout)?;

        let (_, _, _, content_w, content_h) = pdf_page_geometry(pdf_opts)?;
        let table_pages = table_pages_by_row_chunks(&table_rows, content_w, content_h)?;
        let mut pages = vec![timeline_svg];
        pages.extend(table_pages);
        return Ok((pages, vec![]));
    };

    // ── Chart pagination (#661): the chart is always split, never combined
    // with the table into one SVG. ──────────────────────────────────────
    if pdf_opts.pagination && !opts.show_table {
        return Err(PdfError::PaginationRequiresTable);
    }

    let chart_opts = RenderOptions {
        show_table: false,
        ..opts.clone()
    };
    let chart_pagination =
        pagination::paginate_svg_by_lane_groups(ir, &chart_opts, lanes_per_page)?;
    let warnings = chart_pagination.group_bands_split_across_pages;
    let mut pages: Vec<String> = chart_pagination
        .pages
        .into_iter()
        .map(|page| page.svg)
        .collect();

    if opts.show_table {
        let lane_label_lookup = lane_label_lookup(ir);
        let table_rows = collect_table_rows(ir, lane_label_lookup);
        let (_, _, _, content_w, content_h) = pdf_page_geometry(pdf_opts)?;
        if pdf_opts.pagination {
            pages.extend(table_pages_by_row_chunks(
                &table_rows,
                content_w,
                content_h,
            )?);
        } else {
            // Single, unsplit table page: this takes over the role the
            // combined chart+table SVG played when the chart was not split
            // (see the `PdfOptions::chart_pagination` doc comment).
            let table_height = TABLE_ROW_HEIGHT * (table_rows.len() as f64 + 1.0) + 24.0;
            pages.push(svg::render_table_page_svg(
                &table_rows,
                content_w,
                table_height as f32,
                1,
                1,
            )?);
        }
    }

    Ok((pages, warnings))
}

/// Build a lane-id → lane-label lookup closure shared by every table
/// rendering path in this module.
fn lane_label_lookup(ir: &TimelineIr) -> impl Fn(&str) -> String + '_ {
    move |lane_id: &str| -> String {
        ir.lanes
            .iter()
            .find(|lane| lane.id == lane_id)
            .map(|lane| lane.label.clone())
            .unwrap_or_else(|| lane_id.to_string())
    }
}

/// Split `table_rows` across as many table pages as fit `content_h` per page
/// (see [`table_rows_per_page`]), rendering each with a `i / N` footer that
/// counts only table pages.
fn table_pages_by_row_chunks(
    table_rows: &[crate::layout::TableRow],
    content_w: f32,
    content_h: f32,
) -> Result<Vec<String>, PdfError> {
    let rows_per_page = table_rows_per_page(content_h)?;
    let row_chunks: Vec<&[crate::layout::TableRow]> = if table_rows.is_empty() {
        vec![&[]]
    } else {
        table_rows.chunks(rows_per_page).collect()
    };
    let total_table_pages = row_chunks.len();
    row_chunks
        .into_iter()
        .enumerate()
        .map(|(index, rows)| {
            svg::render_table_page_svg(rows, content_w, content_h, index + 1, total_table_pages)
                .map_err(PdfError::from)
        })
        .collect()
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

/// Return the physical page and printable-area dimensions in PDF points.
fn pdf_page_geometry(pdf_opts: &PdfOptions) -> Result<(f32, f32, f32, f32, f32), PdfError> {
    let (mut page_width, mut page_height) = pdf_opts.page_size.portrait_pt();
    if pdf_opts.landscape {
        std::mem::swap(&mut page_width, &mut page_height);
    }
    if !pdf_opts.margin_mm.is_finite() || pdf_opts.margin_mm < 0.0 {
        return Err(PdfError::InvalidMargin(format!(
            "margin must be a non-negative, finite number of millimetres, got {}",
            pdf_opts.margin_mm
        )));
    }
    let margin = (pdf_opts.margin_mm * 72.0 / 25.4) as f32;
    if 2.0 * margin >= page_width.min(page_height) {
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
    Ok((
        page_width,
        page_height,
        margin,
        page_width - 2.0 * margin,
        page_height - 2.0 * margin,
    ))
}

/// Calculate how many complete table data rows fit below a repeated header.
///
/// Reserves one row's worth of height for the repeated column header and
/// requires at least one more complete row for the page-number footer
/// (rendered separately, see `svg::render_table_page_svg`), so the minimum
/// viable printable area fits header + 1 data row + footer margin. Returning
/// `0` here would make callers `chunks(0)` and panic, so this is rejected as
/// an explicit error instead (CLAUDE.md "No silent fallback" 原則).
fn table_rows_per_page(content_height: f32) -> Result<usize, PdfError> {
    let complete_rows = (f64::from(content_height) / TABLE_ROW_HEIGHT).floor() as usize;
    // -1 for the header row that is repeated on every page.
    let rows_after_header = complete_rows.saturating_sub(1);
    if rows_after_header == 0 {
        return Err(PdfError::InvalidMargin(
            "printable area is too short to fit a table header and one complete row".to_string(),
        ));
    }
    Ok(rows_after_header)
}

/// Convert one or more independently rendered SVG pages into one PDF document.
///
/// Every SVG is fit into the same printable area and gets a distinct page
/// object. The SVG-to-PDF conversion remains vector-based for every page.
fn svg_pages_to_pdf(svg_pages: &[String], pdf_opts: PdfOptions) -> Result<Vec<u8>, PdfError> {
    let (page_width, page_height, margin, content_width, content_height) =
        pdf_page_geometry(&pdf_opts)?;

    let mut opt = Options::default();
    opt.fontdb_mut().load_system_fonts();
    let mut trees = Vec::with_capacity(svg_pages.len());
    for svg_page in svg_pages {
        let resolved = svg::resolve_lane_vars_in_styles(svg_page);
        trees.push(Tree::from_str(&resolved, &opt)?);
    }

    let mut alloc = Ref::new(1);
    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();
    let info_id = alloc.bump();
    let page_ids: Vec<(Ref, Ref)> = (0..trees.len())
        .map(|_| (alloc.bump(), alloc.bump()))
        .collect();

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id)
        .kids(page_ids.iter().map(|(page_id, _)| *page_id))
        .count(page_ids.len() as i32);

    for (tree, (page_id, content_id)) in trees.iter().zip(&page_ids) {
        let svg_size = tree.size();
        let scale = (content_width / svg_size.width()).min(content_height / svg_size.height());
        let draw_width = svg_size.width() * scale;
        let draw_height = svg_size.height() * scale;
        let tx = margin + (content_width - draw_width) / 2.0;
        let ty = margin + (content_height - draw_height) / 2.0;

        let (svg_chunk_raw, svg_old_id) =
            svg2pdf::to_chunk(tree, svg2pdf::ConversionOptions::default())
                .map_err(|error| PdfError::Convert(error.to_string()))?;
        let mut id_map: HashMap<Ref, Ref> = HashMap::new();
        let svg_chunk =
            svg_chunk_raw.renumber(|old| *id_map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_id = *id_map.get(&svg_old_id).ok_or_else(|| {
            PdfError::Convert("svg chunk renumber: XObject ID not found".to_string())
        })?;
        let svg_name = Name(b"S1");

        {
            let mut page = pdf.page(*page_id);
            page.media_box(pdf_writer::Rect::new(0.0, 0.0, page_width, page_height));
            page.parent(page_tree_id);
            page.contents(*content_id);
            let mut resources = page.resources();
            resources.x_objects().pair(svg_name, svg_id);
            resources.finish();
            page.finish();
        }
        let mut content = Content::new();
        content
            .save_state()
            .transform([draw_width, 0.0, 0.0, draw_height, tx, ty])
            .x_object(svg_name)
            .restore_state();
        pdf.stream(*content_id, &content.finish());
        pdf.extend(&svg_chunk);
    }

    {
        let mut info = pdf.document_info(info_id);
        info.producer(TextStr("tdsl (svg2pdf)"));
        if let Some(ref title) = pdf_opts.title {
            info.title(TextStr(title.as_str()));
        }
        if let Some(date) = pdf_opts.creation_date {
            info.creation_date(
                pdf_writer::Date::new(date.year)
                    .month(date.month)
                    .day(date.day),
            );
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

    /// PDF file signature: %PDF-
    const PDF_SIGNATURE: &[u8] = &[0x25, 0x50, 0x44, 0x46, 0x2D];

    fn page_object_count(bytes: &[u8]) -> usize {
        let text = String::from_utf8_lossy(bytes);
        let page_objects = text.matches("/Type/Page").count() + text.matches("/Type /Page").count();
        let page_trees = text.matches("/Type/Pages").count() + text.matches("/Type /Pages").count();
        page_objects - page_trees
    }

    fn ir_with_table_rows(row_count: usize) -> TimelineIr {
        let mut ir = sample_ir();
        let template = ir.items[0].clone();
        ir.items = (0..row_count)
            .map(|index| {
                let mut item = template.clone();
                if let Item::Span {
                    id,
                    label,
                    start,
                    end,
                    ..
                } = &mut item
                {
                    *id = format!("span:{index}");
                    *label = format!("Item {index}");
                    *start = index as i64;
                    *end = index as i64 + 1;
                }
                item
            })
            .collect();
        ir
    }

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
    fn paginated_table_uses_complete_rows_and_expected_page_count() {
        // Default A4 printable height fits 34 data rows: one repeated header
        // consumes the 35th 22pt row. Seventy rows therefore produce three
        // table pages plus the unchanged chart page.
        let ir = ir_with_table_rows(70);
        let bytes = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            PdfOptions {
                pagination: true,
                ..PdfOptions::default()
            },
        )
        .expect("paginated PDF renders");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert_eq!(page_object_count(&bytes), 4);
    }

    #[test]
    fn paginated_table_respects_landscape_printable_height() {
        // A4 landscape fits 23 data rows with its repeated header, so the same
        // seventy rows require four table pages and one chart page.
        let ir = ir_with_table_rows(70);
        let bytes = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            PdfOptions {
                pagination: true,
                landscape: true,
                ..PdfOptions::default()
            },
        )
        .expect("landscape paginated PDF renders");
        assert_eq!(page_object_count(&bytes), 5);
    }

    #[test]
    fn paginated_table_respects_margin_row_capacity() {
        // A4 with 50mm margins leaves 558pt of printable height: 25 physical
        // table rows, of which one is the repeated header. Fifty data rows
        // thus require three table pages plus the timeline page.
        let ir = ir_with_table_rows(50);
        let bytes = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            PdfOptions {
                pagination: true,
                margin_mm: 50.0,
                ..PdfOptions::default()
            },
        )
        .expect("paginated PDF with custom margins renders");
        assert_eq!(page_object_count(&bytes), 4);
    }

    #[test]
    fn pagination_requires_table_in_render_options() {
        let err = render_pdf(
            &sample_ir(),
            RenderOptions::default(),
            PdfOptions {
                pagination: true,
                ..PdfOptions::default()
            },
        )
        .expect_err("pagination without a table must fail");
        assert!(matches!(err, PdfError::PaginationRequiresTable));
    }

    #[test]
    fn pagination_with_narrow_printable_area_is_explicit_error_not_panic() {
        // Regression: landscape + a very large margin leaves a printable area
        // that fits the header but zero complete data rows. This must be a
        // clear `InvalidMargin` error, not a `chunks(0)` panic.
        let ir = ir_with_table_rows(3);
        let err = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            PdfOptions {
                pagination: true,
                landscape: true,
                margin_mm: 100.0,
                ..PdfOptions::default()
            },
        )
        .expect_err("a printable area too short for one data row must error");
        assert!(
            matches!(err, PdfError::InvalidMargin(_)),
            "expected InvalidMargin, got: {err}"
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
    fn paginated_pdf_title_metadata_is_set_once_regardless_of_page_count() {
        // ADR-0004 D4: a single /Title entry for the whole document, independent
        // of how many chart/table pages exist.
        let ir = ir_with_table_rows(70);
        let bytes = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            PdfOptions {
                pagination: true,
                title: Some("Paginated Title".to_string()),
                ..PdfOptions::default()
            },
        )
        .expect("paginated PDF with title renders");
        let text = String::from_utf8_lossy(&bytes);
        assert_eq!(
            text.matches("/Title").count(),
            1,
            "exactly one /Title entry must exist regardless of page count"
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

    // ─── #620: pagination と既存機能(show-legend/group band/gantt/zigzag/
    // open-ended)の整合検証 ───────────────────────────────────────────────
    //
    // ADR-0004 D1/D5: group band / gantt / zigzag / open-ended range はいずれも
    // タイムライン本体(チャート、1ページ目)の描画に関わるものであり、D1により
    // タイムライン本体はページ分割の対象外である。したがって pagination の
    // 有効/無効は、これらのレイアウトのタイムラインページの描画結果に
    // 一切影響を与えてはならない。以下のテストはこの不変条件を検証する。

    /// `render_pdf_svg_pages`(本体の実コードパス)を実際に呼び出し、タイムラインページ
    /// (先頭要素)を返す。
    ///
    /// `pagination=false` では `show_table` を強制的に `false` にして呼び出すことで、
    /// 「テーブルなしのチャート単体ページ」（#618以前から存在する基準形態）を得る。
    /// `pagination=true` では `show_table=true` で呼び出し、pages[0] はADR-0004 D1/D2に
    /// より「チャートのみ(テーブルなし)のタイムラインページ」であるべき。両者の
    /// `pages[0]` が一致することを確認すれば、「 pagination はタイムライン描画に一切
    /// 影響しない」という不変条件を、実際の分岐(`render_pdf_svg_pages`)を経由して
    /// 検証できる。
    fn timeline_page_svg_via_render_pdf(
        ir: &TimelineIr,
        mut opts: RenderOptions,
        pagination: bool,
    ) -> String {
        opts.show_table = pagination;
        let pdf_opts = PdfOptions {
            pagination,
            ..PdfOptions::default()
        };
        let (pages, _warnings) = render_pdf_svg_pages(ir, opts, &pdf_opts)
            .expect("render_pdf_svg_pages must succeed for a valid IR");
        pages[0].clone()
    }

    fn ir_with_group_bands_and_table_rows(row_count: usize) -> TimelineIr {
        let mut ir = ir_with_table_rows(row_count);
        ir.lanes = vec![Lane {
            id: "han".into(),
            label: "漢".into(),
            kind: "dynasty".into(),
            order: 10,
            group: Some("グループ1".into()),
            source_span: None,
        }];
        ir
    }

    fn ir_with_open_ended_span_and_table_rows(row_count: usize) -> TimelineIr {
        let mut ir = ir_with_table_rows(row_count);
        if let Some(Item::Span { end_open, .. }) = ir.items.first_mut() {
            *end_open = true;
        }
        ir
    }

    #[test]
    fn pagination_does_not_change_timeline_page_svg_with_group_bands() {
        let ir = ir_with_group_bands_and_table_rows(5);
        let opts = RenderOptions {
            show_table: true,
            layout_style: crate::layout::LayoutStyle::GroupBands,
            ..RenderOptions::default()
        };
        let without_pagination = timeline_page_svg_via_render_pdf(&ir, opts.clone(), false);
        let with_pagination = timeline_page_svg_via_render_pdf(&ir, opts, true);
        assert_eq!(
            without_pagination, with_pagination,
            "group-bands timeline page SVG must be identical regardless of pagination (ADR-0004 D1/D5)"
        );
        assert!(
            with_pagination.contains("tdsl-group-band-even")
                || with_pagination.contains("tdsl-group-label"),
            "sanity check: group-bands styling must actually be present in the timeline page"
        );
    }

    #[test]
    fn pagination_does_not_change_timeline_page_svg_with_gantt_layout() {
        let ir = ir_with_table_rows(5);
        let opts = RenderOptions {
            show_table: true,
            layout_style: crate::layout::LayoutStyle::Gantt,
            ..RenderOptions::default()
        };
        let without_pagination = timeline_page_svg_via_render_pdf(&ir, opts.clone(), false);
        let with_pagination = timeline_page_svg_via_render_pdf(&ir, opts, true);
        assert_eq!(
            without_pagination, with_pagination,
            "gantt timeline page SVG must be identical regardless of pagination (ADR-0004 D1/D5)"
        );
        assert!(
            with_pagination.contains("tdsl-grid-gantt"),
            "sanity check: gantt styling must actually be present in the timeline page"
        );
    }

    #[test]
    fn pagination_does_not_change_timeline_page_svg_with_zigzag_layout() {
        let ir = ir_with_table_rows(5);
        let opts = RenderOptions {
            show_table: true,
            layout_style: crate::layout::LayoutStyle::Zigzag,
            ..RenderOptions::default()
        };
        let without_pagination = timeline_page_svg_via_render_pdf(&ir, opts.clone(), false);
        let with_pagination = timeline_page_svg_via_render_pdf(&ir, opts, true);
        assert_eq!(
            without_pagination, with_pagination,
            "zigzag timeline page SVG must be identical regardless of pagination (ADR-0004 D1/D5)"
        );
    }

    #[test]
    fn pagination_does_not_change_timeline_page_svg_with_open_ended_span() {
        let ir = ir_with_open_ended_span_and_table_rows(5);
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let without_pagination = timeline_page_svg_via_render_pdf(&ir, opts.clone(), false);
        let with_pagination = timeline_page_svg_via_render_pdf(&ir, opts, true);
        assert_eq!(
            without_pagination, with_pagination,
            "open-ended span timeline page SVG must be identical regardless of pagination (ADR-0004 D1/D5)"
        );
        assert!(
            with_pagination.contains("tdsl-item-open-ended"),
            "sanity check: open-ended hook class must actually be present in the timeline page"
        );
    }

    #[test]
    fn pagination_does_not_affect_single_page_output_when_disabled_group_gantt_zigzag_open_ended() {
        // ADR-0004 D3: default (pagination:false) output for these layouts must
        // be byte-for-byte identical to what render_pdf produced before #618/#619
        // introduced the pagination code path at all (non-regression).
        for (ir, layout_style) in [
            (
                ir_with_group_bands_and_table_rows(3),
                crate::layout::LayoutStyle::GroupBands,
            ),
            (ir_with_table_rows(3), crate::layout::LayoutStyle::Gantt),
            (ir_with_table_rows(3), crate::layout::LayoutStyle::Zigzag),
            (
                ir_with_open_ended_span_and_table_rows(3),
                crate::layout::LayoutStyle::default(),
            ),
        ] {
            let opts = RenderOptions {
                show_table: true,
                layout_style,
                ..RenderOptions::default()
            };
            let bytes = render_pdf(&ir, opts, PdfOptions::default())
                .expect("non-paginated render_pdf must succeed for every layout style");
            assert!(bytes.starts_with(PDF_SIGNATURE));
            // Non-paginated output is always exactly one page (timeline + table
            // combined into the single existing SVG, per ADR-0004 D1/D3).
            assert_eq!(
                page_object_count(&bytes),
                1,
                "non-paginated PDF must have exactly one page regardless of layout style"
            );
        }
    }

    #[test]
    fn show_legend_appears_only_on_timeline_page_not_table_pages_when_paginated() {
        // ADR-0004 D5: --show-legend is rendered on the timeline page (page 1)
        // only; it must not leak into any table page.
        let ir = ir_with_table_rows(40);
        let opts = RenderOptions {
            show_table: true,
            show_legend: true,
            ..RenderOptions::default()
        };
        let pdf_opts = PdfOptions {
            pagination: true,
            ..PdfOptions::default()
        };
        let (pages, _warnings) = render_pdf_svg_pages(&ir, opts, &pdf_opts)
            .expect("render_pdf_svg_pages with show_legend + pagination succeeds");
        assert!(
            pages.len() > 2,
            "expected a timeline page plus multiple table pages for 40 rows, got {} pages",
            pages.len()
        );
        assert!(
            pages[0].contains("tdsl-static-legend"),
            "sanity check: show_legend must actually render the static legend on the timeline page"
        );
        // Directly assert on every table page's own SVG (not the timeline page)
        // that the legend never leaks into it — a real check, not just a
        // structural argument about `render_table_page_svg`'s signature.
        for (index, table_page) in pages[1..].iter().enumerate() {
            assert!(
                !table_page.contains("tdsl-static-legend"),
                "table page {} must not contain the static legend, got: {table_page}",
                index + 1
            );
        }

        // End-to-end guard: the full paginated PDF must still convert cleanly.
        let bytes = render_pdf(
            &ir,
            RenderOptions {
                show_table: true,
                show_legend: true,
                ..RenderOptions::default()
            },
            pdf_opts,
        )
        .expect("paginated render_pdf with show_legend succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert!(page_object_count(&bytes) > 1);
    }

    #[test]
    fn show_table_without_pagination_keeps_existing_single_page_behavior() {
        // Non-regression: --show-table alone (no --pdf-pagination) must keep
        // producing a single combined page, exactly as before #618.
        let ir = ir_with_table_rows(10);
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let bytes = render_pdf(&ir, opts, PdfOptions::default())
            .expect("non-paginated show_table render_pdf succeeds");
        assert_eq!(page_object_count(&bytes), 1);
    }

    #[test]
    fn show_table_with_pagination_splits_into_chart_plus_table_pages() {
        // ADR-0004 D2: pagination:true produces exactly 1 chart page + N table
        // pages, regardless of layout_style.
        let ir = ir_with_table_rows(45); // enough rows to force multiple table pages
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let bytes = render_pdf(
            &ir,
            opts,
            PdfOptions {
                pagination: true,
                ..PdfOptions::default()
            },
        )
        .expect("paginated render_pdf succeeds");
        assert!(
            page_object_count(&bytes) >= 2,
            "paginated output with many rows must have at least a chart page and one table page"
        );
    }

    // ─── #621: 決定的レイアウトテスト — 用紙サイズ × 縦横向きのマトリックス (ADR-0004 D7) ───
    //
    // 以下の期待ページ数は、TABLE_ROW_HEIGHT(22pt)と各用紙サイズのポイント寸法、デフォルト
    // margin(10mm)から導出した値をハードコードしたものであり、`table_rows_per_page`の
    // 実装(pdf.rs 中の同名関数)を二重実装してはいない(下記の各ケースは `render_pdf` を
    // 実際に呼び出して得た `page_object_count` と比較する)。
    //
    //   A4 portrait : content_h=785.20pt → 34 rows/page
    //   A4 landscape: content_h=538.58pt → 23 rows/page
    //   A3 portrait : content_h=1133.86pt → 50 rows/page
    //   A3 landscape: content_h=785.20pt → 34 rows/page
    //   Letter portrait : content_h=735.31pt → 32 rows/page
    //   Letter landscape: content_h=555.31pt → 24 rows/page
    //
    // 70 rows での期待 total pages (1 chart page + ceil(70/rows_per_page) table pages):
    #[test]
    fn pagination_page_count_matrix_across_page_size_and_orientation() {
        let cases: &[(PdfPageSize, bool, usize)] = &[
            (PdfPageSize::A4, false, 4),     // ceil(70/34)=3 + 1
            (PdfPageSize::A4, true, 5),      // ceil(70/23)=4 + 1
            (PdfPageSize::A3, false, 3),     // ceil(70/50)=2 + 1
            (PdfPageSize::A3, true, 4),      // ceil(70/34)=3 + 1
            (PdfPageSize::Letter, false, 4), // ceil(70/32)=3 + 1
            (PdfPageSize::Letter, true, 4),  // ceil(70/24)=3 + 1
        ];
        let ir = ir_with_table_rows(70);
        for (page_size, landscape, expected_total_pages) in cases.iter().copied() {
            let bytes = render_pdf(
                &ir,
                RenderOptions {
                    show_table: true,
                    ..RenderOptions::default()
                },
                PdfOptions {
                    pagination: true,
                    page_size,
                    landscape,
                    ..PdfOptions::default()
                },
            )
            .unwrap_or_else(|e| {
                panic!("render_pdf must succeed for {page_size:?} landscape={landscape}: {e}")
            });
            assert_eq!(
                page_object_count(&bytes),
                expected_total_pages,
                "{page_size:?} landscape={landscape}: expected {expected_total_pages} total pages (1 chart + table pages)"
            );
        }
    }

    /// CJKテキスト(長いラベル、漢字・ひらがな・カタカナ混在)を含む行でページ分割した場合、
    /// 各テーブルページの見出し・ページ番号フッタが存在し、ページ数が期待通りであることを
    /// `render_pdf_svg_pages` 経由で検証する(中間SVG文字列を直接assertし、バイナリ埋め込み後のPDFAssertionの
    /// 限界を回避する)。
    fn ir_with_cjk_table_rows(row_count: usize) -> TimelineIr {
        let mut ir = sample_ir();
        ir.lanes = vec![Lane {
            id: "han".into(),
            label: "漢・唐・宋・元・明・清（中国历代王朝）".into(),
            kind: "dynasty".into(),
            order: 10,
            group: None,
            source_span: None,
        }];
        let template = ir.items[0].clone();
        ir.items = (0..row_count)
            .map(|index| {
                let mut item = template.clone();
                if let Item::Span {
                    id,
                    lane,
                    label,
                    tags,
                    start,
                    end,
                    ..
                } = &mut item
                {
                    *id = format!("span:{index}");
                    *lane = "han".into();
                    *label = format!(
                        "長いラベル {index}: ひらがな・カタカナ・漢字が混在するテーブル行テキスト"
                    );
                    *tags = vec!["王朝".into()];
                    *start = index as i64;
                    *end = index as i64 + 1;
                }
                item
            })
            .collect();
        ir
    }

    #[test]
    fn cjk_table_rows_paginate_with_expected_page_count_and_repeated_header() {
        let ir = ir_with_cjk_table_rows(70); // same 70 rows as the A4-portrait matrix case above
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let pdf_opts = PdfOptions {
            pagination: true,
            ..PdfOptions::default()
        };
        let (pages, _warnings) = render_pdf_svg_pages(&ir, opts, &pdf_opts)
            .expect("render_pdf_svg_pages with CJK rows succeeds");
        assert_eq!(
            pages.len(),
            4,
            "CJK content must not change the page count derived purely from row count/geometry"
        );
        for (index, table_page) in pages[1..].iter().enumerate() {
            for col in [
                crate::layout::TABLE_COL_TIME,
                crate::layout::TABLE_COL_LABEL,
                crate::layout::TABLE_COL_LANE,
                crate::layout::TABLE_COL_TAGS,
            ] {
                assert!(
                    table_page.contains(col),
                    "table page {} must repeat the '{col}' CJK column header, got: {table_page}",
                    index + 1
                );
            }
            let expected_footer = format!("{} / {}", index + 1, pages.len() - 1);
            assert!(
                table_page.contains(&expected_footer),
                "table page {} must contain the '{expected_footer}' page-number footer",
                index + 1
            );
        }
        // Sanity check: the long CJK label text actually made it into at least
        // one table page (not silently dropped/truncated to empty).
        assert!(
            pages[1..].iter().any(|page| page.contains("長いラベル")),
            "CJK label text must appear verbatim in a table page"
        );
    }

    /// ADR-0004 D1/D5: `RenderOptions.color_map`によるテーマ(タグ→色)の切り替えは
    /// タイムライン本体(1ページ目)の描画にしか影響しない。paginationの有効/無効とは
    /// 直交であり、ページ数を変えず、タイムラインページの描画にも差分が出ないことを
    /// 検証する（CLIは `ir.meta.color_map` を `RenderOptions.color_map` にコピーして渡すが、
    /// `tdsl-render` 自体は後者のみを参照する）。
    #[test]
    fn color_map_theme_does_not_affect_pagination_or_page_count() {
        let ir = ir_with_table_rows(40);
        let opts = RenderOptions {
            show_table: true,
            color_map: std::collections::HashMap::from([(
                "dynasty".to_string(),
                "#cc0000".to_string(),
            )]),
            ..RenderOptions::default()
        };
        let pdf_opts = PdfOptions {
            pagination: true,
            ..PdfOptions::default()
        };
        let (with_theme, _warnings) = render_pdf_svg_pages(&ir, opts, &pdf_opts)
            .expect("render_pdf_svg_pages with color_map theme succeeds");

        let (without_theme, _warnings) = render_pdf_svg_pages(
            &ir,
            RenderOptions {
                show_table: true,
                ..RenderOptions::default()
            },
            &pdf_opts,
        )
        .expect("render_pdf_svg_pages without color_map theme succeeds");

        assert_eq!(
            with_theme.len(),
            without_theme.len(),
            "color_map theme must not change the number of pages"
        );
        // The theme color must actually appear in the timeline page (sanity
        // check that the color_map was applied at all)...
        assert!(
            with_theme[0].contains("fill:#cc0000;"),
            "sanity check: color_map theme color must appear in the timeline page, got: {}",
            with_theme[0]
        );
        // ...but table pages must be byte-for-byte identical regardless of the
        // theme, since `render_table_page_svg` never receives `RenderOptions`
        // (or any lane/tag color information) at all.
        for (index, (themed, plain)) in with_theme[1..]
            .iter()
            .zip(without_theme[1..].iter())
            .enumerate()
        {
            assert_eq!(
                themed,
                plain,
                "table page {} must be identical regardless of color_map theme",
                index + 1
            );
        }
    }

    // ─── #661: chart pagination integrated into PDF output ──────────────────

    /// `lane_count` lanes, each with exactly one 1-year span in its own lane,
    /// ordered `lane0, lane1, ...` so chunking by `lanes_per_page` is
    /// deterministic.
    fn ir_with_lanes(lane_count: usize) -> TimelineIr {
        let mut ir = sample_ir();
        ir.lanes = (0..lane_count)
            .map(|i| Lane {
                id: format!("lane{i}"),
                label: format!("Lane {i}"),
                kind: "custom".into(),
                order: i as i64,
                group: None,
                source_span: None,
            })
            .collect();
        ir.items = (0..lane_count)
            .map(|i| Item::Span {
                id: format!("span:{i}"),
                lane: format!("lane{i}"),
                start: i as i64,
                end: i as i64 + 1,
                label: format!("Span {i}"),
                tags: vec![],
                source: None,
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
            })
            .collect();
        ir
    }

    /// Same lane layout as [`ir_with_lanes`] but with `row_count` table rows,
    /// all placed in `lane0` so lane-group chart pagination (which only cares
    /// about `ir.lanes`) still has 4 lanes to split while the table has many
    /// rows to paginate.
    fn ir_with_lanes_and_table_rows(lane_count: usize, row_count: usize) -> TimelineIr {
        let mut ir = ir_with_lanes(lane_count);
        let template = ir.items[0].clone();
        ir.items = (0..row_count)
            .map(|index| {
                let mut item = template.clone();
                if let Item::Span {
                    id,
                    lane,
                    label,
                    start,
                    end,
                    ..
                } = &mut item
                {
                    *id = format!("span:{index}");
                    *lane = "lane0".into();
                    *label = format!("Item {index}");
                    *start = index as i64;
                    *end = index as i64 + 1;
                }
                item
            })
            .collect();
        ir
    }

    #[test]
    fn chart_pagination_none_does_not_change_default_pdf_options() {
        // Regression: the new field defaults to `None`, so `PdfOptions::default()`
        // must still describe "no chart pagination" exactly as before #661.
        assert_eq!(PdfOptions::default().chart_pagination, None);
    }

    #[test]
    fn chart_pagination_splits_into_multiple_chart_pages_without_table() {
        let ir = ir_with_lanes(4);
        let bytes = render_pdf(
            &ir,
            RenderOptions::default(),
            PdfOptions {
                chart_pagination: Some(2),
                ..PdfOptions::default()
            },
        )
        .expect("chart-paginated PDF without a table renders");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert_eq!(
            page_object_count(&bytes),
            2,
            "4 lanes / 2 per page = 2 chart pages; show_table is false so no table page"
        );
    }

    #[test]
    fn chart_pagination_with_show_table_appends_single_unsplit_table_page() {
        let ir = ir_with_lanes(4);
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let bytes = render_pdf(
            &ir,
            opts,
            PdfOptions {
                chart_pagination: Some(2),
                ..PdfOptions::default()
            },
        )
        .expect("chart-paginated PDF with show_table renders");
        assert_eq!(
            page_object_count(&bytes),
            3,
            "2 chart pages + 1 unsplit table page (pdf_pagination not requested)"
        );
    }

    #[test]
    fn chart_pagination_combined_with_pdf_pagination_orders_chart_pages_before_table_pages() {
        // 4 lanes / 2 per page = 2 chart pages. 70 rows at the default A4
        // portrait geometry (34 rows/page, see the table-only matrix test
        // above) split into 3 table pages, so total = 5 pages.
        let ir = ir_with_lanes_and_table_rows(4, 70);
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let (pages, _warnings) = render_pdf_svg_pages(
            &ir,
            opts,
            &PdfOptions {
                chart_pagination: Some(2),
                pagination: true,
                ..PdfOptions::default()
            },
        )
        .expect("combined chart + table pagination succeeds");
        assert_eq!(pages.len(), 2 + 3, "2 chart pages then 3 table pages");
        // Table footers must count only the table pages (1/3..3/3), never
        // including the 2 preceding chart pages in the denominator or offset.
        assert!(
            pages[2].contains("1 / 3"),
            "first table page footer must be '1 / 3', got: {}",
            pages[2]
        );
        assert!(
            pages[3].contains("2 / 3"),
            "second table page footer must be '2 / 3', got: {}",
            pages[3]
        );
        assert!(
            pages[4].contains("3 / 3"),
            "third table page footer must be '3 / 3', got: {}",
            pages[4]
        );
    }

    #[test]
    fn chart_pagination_zero_lanes_per_page_is_explicit_error() {
        let ir = ir_with_lanes(2);
        let err = render_pdf(
            &ir,
            RenderOptions::default(),
            PdfOptions {
                chart_pagination: Some(0),
                ..PdfOptions::default()
            },
        )
        .expect_err("lanes_per_page=0 must be a hard error, not a silent no-op");
        assert!(
            matches!(
                err,
                PdfError::ChartPagination(PaginationError::InvalidLanesPerPage)
            ),
            "expected PdfError::ChartPagination(InvalidLanesPerPage), got: {err}"
        );
    }

    #[test]
    fn chart_pagination_and_pdf_pagination_without_show_table_is_explicit_error() {
        let ir = ir_with_lanes(4);
        let err = render_pdf(
            &ir,
            RenderOptions::default(),
            PdfOptions {
                chart_pagination: Some(2),
                pagination: true,
                ..PdfOptions::default()
            },
        )
        .expect_err(
            "pdf table pagination without show_table must fail even with chart_pagination set",
        );
        assert!(matches!(err, PdfError::PaginationRequiresTable));
    }

    #[test]
    fn chart_pagination_group_band_split_warning_propagates_through_render_pdf_with_warnings() {
        let mut ir = ir_with_lanes(4);
        ir.lanes[0].group = Some("グループ".to_string());
        ir.lanes[1].group = Some("グループ".to_string());
        ir.lanes[2].group = Some("グループ".to_string());
        // "グループ" spans lanes 0,1,2; with 2 lanes/page its run crosses the
        // boundary between chunk 0 (lanes 0,1) and chunk 1 (lanes 2,3).
        let (bytes, warnings) = render_pdf_with_warnings(
            &ir,
            RenderOptions::default(),
            PdfOptions {
                chart_pagination: Some(2),
                ..PdfOptions::default()
            },
        )
        .expect("chart-paginated PDF with a split group band still renders");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert_eq!(
            warnings,
            vec!["グループ".to_string()],
            "group band split across chart pages must be reported, not silently dropped"
        );
    }

    #[test]
    fn chart_pagination_none_warnings_are_always_empty() {
        let ir = sample_ir();
        let (bytes, warnings) =
            render_pdf_with_warnings(&ir, RenderOptions::default(), PdfOptions::default())
                .expect("default PDF render succeeds");
        assert!(bytes.starts_with(PDF_SIGNATURE));
        assert!(
            warnings.is_empty(),
            "chart_pagination: None must never produce warnings"
        );
    }

    // Deterministic layout test matrix (ADR-0004 D7 pattern), extended with a
    // chart-pagination case: 4 lanes / 2 per page always yields 2 chart pages
    // (chart pagination is lane-count driven, independent of PDF page
    // geometry), while the table-page count still follows the same
    // per-page-size row capacity as the table-only matrix test above.
    #[test]
    fn chart_pagination_page_count_matrix_across_page_size_and_orientation() {
        let cases: &[(PdfPageSize, bool, usize)] = &[
            (PdfPageSize::A4, false, 2 + 3),
            (PdfPageSize::A4, true, 2 + 4),
            (PdfPageSize::A3, false, 2 + 2),
            (PdfPageSize::A3, true, 2 + 3),
            (PdfPageSize::Letter, false, 2 + 3),
            (PdfPageSize::Letter, true, 2 + 3),
        ];
        let ir = ir_with_lanes_and_table_rows(4, 70);
        for (page_size, landscape, expected_total_pages) in cases.iter().copied() {
            let bytes = render_pdf(
                &ir,
                RenderOptions {
                    show_table: true,
                    ..RenderOptions::default()
                },
                PdfOptions {
                    chart_pagination: Some(2),
                    pagination: true,
                    page_size,
                    landscape,
                    ..PdfOptions::default()
                },
            )
            .unwrap_or_else(|e| {
                panic!("render_pdf must succeed for {page_size:?} landscape={landscape}: {e}")
            });
            assert_eq!(
                page_object_count(&bytes),
                expected_total_pages,
                "{page_size:?} landscape={landscape}: expected {expected_total_pages} total pages (2 chart + table pages)"
            );
        }
    }
}
