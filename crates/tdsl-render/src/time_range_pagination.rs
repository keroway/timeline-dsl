//! Time-range-axis pagination of the SVG chart body (issue #733, ADR-0005 D3).
//!
//! Promoted from the `#[cfg(test)]`-only spike (issue #709, parent #662).
//! See `docs/adr/0005-timeline-chart-pagination.md` D3 for the GO decision
//! and design history.
//!
//! ## Approach
//!
//! Unlike [`crate::pagination`]'s lane-group axis (which keeps `meta.range`
//! common across pages and partitions `lanes`/`items`), this axis paginates
//! `meta.range` itself into `page_count` contiguous integer-year segments.
//! Every page's `TimelineIr` carries the *full*, unfiltered `lanes`/`items`
//! list with only `meta.range` (and the sub-year precision fields, which no
//! longer describe the synthetic segment boundary and are cleared) rewritten
//! to that segment.
//!
//! Because a `Span`/`EventRange` item's `[start, end]` extent can legitimately
//! straddle a page boundary (e.g. a centuries-long dynasty spanning two
//! 100-year pages), such items are geometrically clipped by the existing
//! [`crate::layout::LayoutModel::compute`] → `primary_axis_segment` clamp (no
//! new geometry needed) and separately *detected* — never silently dropped —
//! via [`items_crossing_boundaries`], so callers can warn.
//!
//! Drawing a continuation marker for a clipped item (ADR-0005 §2 strategy
//! "クリップ + 継続マーカー") is deferred to a follow-up issue; this module
//! only promotes the pure marker-computation primitive
//! ([`clip_with_continuation_markers`]) without wiring it into the SVG
//! output yet.

use tdsl_core::ir::{Item, TimelineIr};

use crate::RenderError;
use crate::layout::{LayoutModel, RenderOptions, TABLE_ROW_HEIGHT, collect_table_rows};
use crate::pagination::{ChartPage, PageKind};
use crate::svg::{render_svg, render_table_page_svg};

/// Error returned by [`split_ir_by_time_range`], [`items_crossing_boundaries`],
/// [`clip_with_continuation_markers`], and [`paginate_svg_by_time_range`].
#[derive(Debug, thiserror::Error)]
pub enum TimeRangePaginationError {
    /// `page_count == 0` would produce zero pages; that's a silent no-op, not
    /// a valid pagination request (implementation-strict.md §1 "Explicit
    /// error over silent fallback").
    #[error("page_count must be >= 1")]
    InvalidPageCount,
    /// `meta.range` is empty/degenerate (`end <= start`), so there is no time
    /// span to divide into `page_count` non-empty segments.
    #[error("meta.range {start}..{end} is empty; cannot split into pages")]
    EmptyRange { start: i64, end: i64 },
    /// The range is non-empty overall but too narrow to produce `page_count`
    /// *non-empty* integer-year segments (e.g. a 2-year range split into 5
    /// pages). Rather than silently emitting zero-width pages, this is
    /// rejected explicitly.
    #[error(
        "range {start}..{end} ({span} year(s)) is too narrow to split into {page_count} non-empty page(s)"
    )]
    RangeTooNarrowForPageCount {
        start: i64,
        end: i64,
        span: i64,
        page_count: usize,
    },
    /// `page_index >= page_count` in [`clip_with_continuation_markers`] would
    /// index past the `page_count + 1` segment boundaries — a hard error
    /// instead of a panic, since this is a public API a caller can invoke
    /// with an out-of-range index.
    #[error("page_index {page_index} is out of range for page_count {page_count}")]
    InvalidPageIndex {
        page_index: usize,
        page_count: usize,
    },
    #[error("SVG rendering failed: {0}")]
    Render(#[from] RenderError),
}

impl From<std::fmt::Error> for TimeRangePaginationError {
    fn from(err: std::fmt::Error) -> Self {
        Self::Render(RenderError::from(err))
    }
}

/// One time-range-axis page: the segment's `(start, end)` boundary and the
/// `TimelineIr` clone whose `meta.range` (and sub-year precision fields) have
/// been rewritten to that segment.
#[derive(Debug)]
pub struct TimeRangePage {
    pub range: (i64, i64),
    pub ir: TimelineIr,
}

/// Split `ir.meta.range` into `page_count` contiguous, non-overlapping,
/// non-empty integer-year segments (as evenly as possible; earlier segments
/// absorb any remainder), and build one `TimelineIr` per segment with
/// `meta.range` rewritten accordingly.
///
/// The segment boundaries are always pure integer years (derived by dividing
/// `meta.range` by `page_count`), so any sub-year precision the original
/// `meta` carried (`range_start_month`, `range_end_day`, etc.) no longer
/// describes the *segment* boundary correctly and is cleared on every page's
/// `meta` — carrying it forward unchanged would silently misrepresent the
/// synthetic boundary's precision. A `.tdsl` file authored with month/day/
/// second-precision `range` values will therefore show plain-year tick marks
/// once split into pages; the original file's declared precision is
/// unaffected (only the in-memory per-page `Meta` used for that page's
/// render is cleared).
///
/// `lanes`/`items`/`imports`/`sources` are duplicated onto every page
/// unchanged (unlike the lane-group axis, this does not filter or clip
/// items — geometric clipping of items outside `[year_min, year_max]` is
/// already handled by `layout::primary_axis_segment`'s clamp and by
/// `layout::year_in_range` for `Event`).
pub fn split_ir_by_time_range(
    ir: &TimelineIr,
    page_count: usize,
) -> Result<Vec<TimeRangePage>, TimeRangePaginationError> {
    if page_count == 0 {
        return Err(TimeRangePaginationError::InvalidPageCount);
    }
    let boundaries = segment_boundaries(ir.meta.range, page_count)?;

    let mut pages = Vec::with_capacity(page_count);
    for window in boundaries.windows(2) {
        let (seg_start, seg_end) = (window[0], window[1]);
        let mut meta = ir.meta.clone();
        meta.range = (seg_start, seg_end);
        meta.range_start_month = None;
        meta.range_start_day = None;
        meta.range_start_hour = None;
        meta.range_start_minute = None;
        meta.range_start_second = None;
        meta.range_start_offset_minutes = None;
        meta.range_end_month = None;
        meta.range_end_day = None;
        meta.range_end_hour = None;
        meta.range_end_minute = None;
        meta.range_end_second = None;
        meta.range_end_offset_minutes = None;

        let page_ir = TimelineIr {
            meta,
            lanes: ir.lanes.clone(),
            items: ir.items.clone(),
            imports: ir.imports.clone(),
            sources: ir.sources.clone(),
        };
        pages.push(TimeRangePage {
            range: (seg_start, seg_end),
            ir: page_ir,
        });
    }
    Ok(pages)
}

/// Compute `page_count + 1` monotonically increasing integer-year boundaries
/// covering `range` (`boundaries[0] == range.0`, `boundaries[page_count] ==
/// range.1`), or a hard error if `range` is empty or too narrow to produce
/// `page_count` non-empty segments.
fn segment_boundaries(
    range: (i64, i64),
    page_count: usize,
) -> Result<Vec<i64>, TimeRangePaginationError> {
    let (start, end) = range;
    if end <= start {
        return Err(TimeRangePaginationError::EmptyRange { start, end });
    }
    let span = end - start;

    let mut boundaries = Vec::with_capacity(page_count + 1);
    for i in 0..=page_count {
        let frac = i as f64 / page_count as f64;
        boundaries.push(start + (span as f64 * frac).round() as i64);
    }
    // Guard against rounding collapsing two adjacent boundaries onto the same
    // year (i.e. a zero-width page) instead of silently rendering an empty
    // page.
    for window in boundaries.windows(2) {
        if window[1] <= window[0] {
            return Err(TimeRangePaginationError::RangeTooNarrowForPageCount {
                start,
                end,
                span,
                page_count,
            });
        }
    }
    Ok(boundaries)
}

/// A `Span`/`EventRange` item whose `[start, end]` straddles one of the
/// interior segment boundaries produced by [`split_ir_by_time_range`] (i.e.
/// `start < boundary < end`, strictly interior — an item that merely touches
/// a boundary at its own start/end is NOT considered crossing).
#[derive(Debug, PartialEq, Eq)]
pub struct BoundaryCrossingItem {
    pub id: String,
    pub start: i64,
    pub end: i64,
    /// The interior boundary year(s) this item straddles.
    pub crossed_boundaries: Vec<i64>,
}

/// Detect every `Span`/`EventRange` item in `ir` whose time extent straddles
/// one of the *interior* boundaries of a `page_count`-way time-range split
/// (the outer boundaries `range.0`/`range.1` are not "crossings" — they are
/// the timeline's own edges).
///
/// `Event` items are structurally excluded: they carry only a single `time`
/// point (no `[start, end]` extent), so `layout::year_in_range` already
/// either shows or skips them per page with no clipping/crossing concept to
/// report.
///
/// Mirrors `pagination::find_groups_split_across_chunks` /
/// `ChartPagination::group_bands_split_across_pages`: rather than silently
/// letting each page draw its own truncated bar with no record of the split,
/// callers get an explicit, non-empty list to warn on.
pub fn items_crossing_boundaries(
    ir: &TimelineIr,
    range: (i64, i64),
    page_count: usize,
) -> Result<Vec<BoundaryCrossingItem>, TimeRangePaginationError> {
    let boundaries = segment_boundaries(range, page_count)?;
    // Interior boundaries only: exclude the outer start/end.
    let interior = &boundaries[1..boundaries.len().saturating_sub(1)];

    let mut crossing = Vec::new();
    for item in &ir.items {
        let extent = match item {
            Item::Span { id, start, end, .. } | Item::EventRange { id, start, end, .. } => {
                Some((id.as_str(), *start, *end))
            }
            Item::Event { .. } => None,
        };
        let Some((id, start, end)) = extent else {
            continue;
        };
        let crossed: Vec<i64> = interior
            .iter()
            .copied()
            .filter(|&b| start < b && b < end)
            .collect();
        if !crossed.is_empty() {
            crossing.push(BoundaryCrossingItem {
                id: id.to_string(),
                start,
                end,
                crossed_boundaries: crossed,
            });
        }
    }
    Ok(crossing)
}

/// A signal, per page, of which side(s) of a `Span`/`EventRange` were
/// clipped by the page boundary — the primitive a future SVG continuation
/// marker (ADR-0005 §2 strategy 1: "クリップ + 継続マーカー") would draw
/// from. Not yet wired into [`paginate_svg_by_time_range`]'s output.
#[derive(Debug, PartialEq, Eq)]
pub struct ClipMarker {
    pub id: String,
    /// The item's `start` is before this page's segment start (i.e. it was
    /// already visible, continuing, on an earlier page).
    pub continues_from_previous_page: bool,
    /// The item's `end` is after this page's segment end (i.e. it continues
    /// onto a later page).
    pub continues_to_next_page: bool,
}

/// Compute clip markers for every `Span`/`EventRange` that intersects
/// `page_index`'s segment (items wholly outside the segment are omitted —
/// they'd clamp to a zero/negative-width bar per `primary_axis_segment` and
/// have no marker to report).
pub fn clip_with_continuation_markers(
    ir: &TimelineIr,
    range: (i64, i64),
    page_count: usize,
    page_index: usize,
) -> Result<Vec<ClipMarker>, TimeRangePaginationError> {
    if page_index >= page_count {
        return Err(TimeRangePaginationError::InvalidPageIndex {
            page_index,
            page_count,
        });
    }
    let boundaries = segment_boundaries(range, page_count)?;
    let (seg_start, seg_end) = (boundaries[page_index], boundaries[page_index + 1]);

    let mut out = Vec::new();
    for item in &ir.items {
        let extent = match item {
            Item::Span { id, start, end, .. } | Item::EventRange { id, start, end, .. } => {
                Some((id.as_str(), *start, *end))
            }
            Item::Event { .. } => None,
        };
        let Some((id, start, end)) = extent else {
            continue;
        };
        if end <= seg_start || start >= seg_end {
            continue; // wholly outside this page's segment
        }
        out.push(ClipMarker {
            id: id.to_string(),
            continues_from_previous_page: start < seg_start,
            continues_to_next_page: end > seg_end,
        });
    }
    Ok(out)
}

/// Result of a full time-range-axis pagination pass.
#[derive(Debug)]
pub struct TimeRangeChartPagination {
    pub pages: Vec<ChartPage>,
    /// `Span`/`EventRange` items whose extent straddles an interior page
    /// boundary. Callers MUST warn (not silently ignore) when this is
    /// non-empty (implementation-strict.md §1 "Explicit error over silent
    /// fallback") — mirrors `ChartPagination::group_bands_split_across_pages`
    /// on the lane-group axis.
    pub items_crossing_boundaries: Vec<BoundaryCrossingItem>,
}

/// Split `ir.meta.range` into `page_count` segments (see
/// [`split_ir_by_time_range`]) and render one SVG chart page per segment
/// through the existing `LayoutModel::compute` + `render_svg` pipeline,
/// unmodified.
///
/// If `opts.show_table` is true, a single trailing [`PageKind::Table`] page
/// listing every item in `ir` is appended, mirroring the lane-group axis
/// (`pagination::paginate_svg_by_lane_groups`) — every page's IR already
/// carries the full unfiltered item list, so per-page tables would be
/// identical, nonsensical duplication.
pub fn paginate_svg_by_time_range(
    ir: &TimelineIr,
    opts: &RenderOptions,
    page_count: usize,
) -> Result<TimeRangeChartPagination, TimeRangePaginationError> {
    let time_pages = split_ir_by_time_range(ir, page_count)?;
    let crossing = items_crossing_boundaries(ir, ir.meta.range, page_count)?;

    let mut pages = Vec::with_capacity(time_pages.len());
    let mut chart_width: f64 = 0.0;
    for time_page in &time_pages {
        let chart_opts = RenderOptions {
            show_table: false,
            ..opts.clone()
        };
        let layout = LayoutModel::compute(&time_page.ir, chart_opts)?;
        chart_width = layout.total_width;
        let svg = render_svg(&layout)?;
        pages.push(ChartPage {
            lane_ids: vec![],
            svg,
            kind: PageKind::Chart,
        });
    }

    if opts.show_table {
        let lane_label_lookup = |lane_id: &str| -> String {
            ir.lanes
                .iter()
                .find(|lane| lane.id == lane_id)
                .map(|lane| lane.label.clone())
                .unwrap_or_else(|| lane_id.to_string())
        };
        let table_rows = collect_table_rows(ir, lane_label_lookup);
        let table_height = TABLE_ROW_HEIGHT * (table_rows.len() as f64 + 1.0) + 24.0;
        let table_svg =
            render_table_page_svg(&table_rows, chart_width as f32, table_height as f32, 1, 1)?;
        pages.push(ChartPage {
            lane_ids: vec![],
            svg: table_svg,
            kind: PageKind::Table,
        });
    }

    Ok(TimeRangeChartPagination {
        pages,
        items_crossing_boundaries: crossing,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LaidItem, LayoutStyle};
    use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

    fn meta(start: i64, end: i64) -> Meta {
        Meta {
            title: "time-range pagination".into(),
            unit: "year".into(),
            range: (start, end),
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        }
    }

    fn lane(id: &str) -> Lane {
        Lane {
            id: id.to_string(),
            label: format!("Lane {id}"),
            kind: "custom".into(),
            order: 1,
            group: None,
            source_span: None,
        }
    }

    fn span(id: &str, lane_id: &str, start: i64, end: i64) -> Item {
        Item::Span {
            id: id.to_string(),
            lane: lane_id.to_string(),
            start,
            end,
            label: format!("Span {id}"),
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
        }
    }

    fn event(id: &str, lane_id: &str, time: i64) -> Item {
        Item::Event {
            id: id.to_string(),
            lane: lane_id.to_string(),
            time,
            label: format!("Event {id}"),
            tags: vec![],
            source: None,
            origin: None,
            note: None,
            link: None,
            color: None,
            time_month: None,
            time_day: None,
            time_hour: None,
            time_minute: None,
            time_second: None,
            time_offset_minutes: None,
            source_span: None,
        }
    }

    fn event_range(id: &str, lane_id: &str, start: i64, end: i64) -> Item {
        Item::EventRange {
            id: id.to_string(),
            lane: lane_id.to_string(),
            start,
            end,
            label: format!("EventRange {id}"),
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
        }
    }

    fn item_id(item: &Item) -> &str {
        match item {
            Item::Span { id, .. } | Item::Event { id, .. } | Item::EventRange { id, .. } => id,
        }
    }

    /// No-crossing fixture: `range 0..400`, one lane, and items that each sit
    /// wholly inside exactly one of 4 100-year pages (0-100, 100-200,
    /// 200-300, 300-400).
    fn no_crossing_ir() -> TimelineIr {
        TimelineIr {
            meta: meta(0, 400),
            lanes: vec![lane("a")],
            items: vec![
                span("s-1", "a", 10, 90),
                span("s-2", "a", 110, 190),
                span("s-3", "a", 210, 290),
                event("e-1", "a", 350),
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    fn crossing_ir() -> TimelineIr {
        let mut ir = no_crossing_ir();
        ir.items.push(span("s-crossing", "a", 80, 220));
        ir
    }

    // ─── split_ir_by_time_range ────────────────────────────────────────────

    #[test]
    fn splits_into_n_pages_and_renders_each_through_existing_pipeline() {
        let ir = no_crossing_ir();
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        assert_eq!(pages.len(), 4, "400-year range / 4 pages = 4 segments");

        let mut svgs = Vec::new();
        for page in &pages {
            let layout = LayoutModel::compute(&page.ir, RenderOptions::default())
                .expect("layout should succeed for a valid page IR");
            let svg = render_svg(&layout).expect("SVG rendering should succeed");
            assert!(svg.starts_with("<svg"));
            assert!(svg.contains("</svg>"));
            svgs.push(svg);
        }
        assert!(svgs[0].contains("Span s-1"));
        assert!(svgs[1].contains("Span s-2"));
        assert!(svgs[2].contains("Span s-3"));
        assert!(svgs[3].contains("Event e-1"));
    }

    #[test]
    fn page_ranges_partition_the_original_range_contiguously() {
        let ir = no_crossing_ir();
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        assert_eq!(pages[0].range, (0, 100));
        assert_eq!(pages[1].range, (100, 200));
        assert_eq!(pages[2].range, (200, 300));
        assert_eq!(pages[3].range, (300, 400));
    }

    #[test]
    fn each_page_time_axis_is_shifted_to_its_own_segment() {
        let ir = no_crossing_ir();
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");

        let layouts: Vec<LayoutModel> = pages
            .iter()
            .map(|page| {
                LayoutModel::compute(&page.ir, RenderOptions::default())
                    .expect("layout should succeed")
            })
            .collect();

        for (layout, page) in layouts.iter().zip(&pages) {
            assert_eq!((layout.year_min, layout.year_max), page.range);
        }

        let tick_sets: Vec<Vec<i64>> = layouts.iter().map(LayoutModel::ticks).collect();
        for i in 0..tick_sets.len() {
            for j in (i + 1)..tick_sets.len() {
                assert_ne!(
                    tick_sets[i], tick_sets[j],
                    "page {i} and page {j} must have distinct tick sets"
                );
            }
        }

        let x_page0_at_50 = layouts[0].year_to_x(50);
        let x_page1_at_150 = layouts[1].year_to_x(150);
        assert_eq!(
            x_page0_at_50, x_page1_at_150,
            "year 50 in page 0's local axis and year 150 in page 1's local axis \
             (both 50 years past their page's year_min) must land at the same x, \
             confirming each page's axis origin shifted to its own segment start"
        );
    }

    #[test]
    fn sub_year_precision_fields_are_cleared_on_each_page() {
        let mut ir = no_crossing_ir();
        ir.meta.range_start_month = Some(6);
        ir.meta.range_end_day = Some(15);
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        for page in &pages {
            assert_eq!(page.ir.meta.range_start_month, None);
            assert_eq!(page.ir.meta.range_end_day, None);
        }
    }

    // ─── zero/empty/too-narrow are hard errors, not silent no-ops ─────────

    #[test]
    fn zero_page_count_is_rejected_explicitly() {
        let ir = no_crossing_ir();
        let err = split_ir_by_time_range(&ir, 0)
            .expect_err("page_count=0 must be a hard error, not a silent no-op");
        assert!(matches!(err, TimeRangePaginationError::InvalidPageCount));
    }

    #[test]
    fn empty_range_is_rejected_explicitly() {
        let mut ir = no_crossing_ir();
        ir.meta.range = (100, 100);
        let err = split_ir_by_time_range(&ir, 4)
            .expect_err("an empty (start == end) range must be a hard error");
        assert!(matches!(
            err,
            TimeRangePaginationError::EmptyRange {
                start: 100,
                end: 100
            }
        ));
    }

    #[test]
    fn inverted_range_is_rejected_explicitly() {
        let mut ir = no_crossing_ir();
        ir.meta.range = (100, 50);
        let err = split_ir_by_time_range(&ir, 4)
            .expect_err("an inverted (end < start) range must be a hard error");
        assert!(matches!(
            err,
            TimeRangePaginationError::EmptyRange {
                start: 100,
                end: 50
            }
        ));
    }

    #[test]
    fn range_too_narrow_for_page_count_is_rejected_explicitly() {
        let mut ir = no_crossing_ir();
        ir.meta.range = (0, 2);
        let err = split_ir_by_time_range(&ir, 5).expect_err(
            "a range too narrow to produce page_count non-empty segments must be a hard error",
        );
        assert!(matches!(
            err,
            TimeRangePaginationError::RangeTooNarrowForPageCount {
                start: 0,
                end: 2,
                span: 2,
                page_count: 5,
            }
        ));
    }

    // ─── items_crossing_boundaries ─────────────────────────────────────────

    #[test]
    fn span_crossing_a_page_boundary_is_detected() {
        let mut ir = no_crossing_ir();
        ir.items.push(span("s-crossing", "a", 80, 220));

        let crossing = items_crossing_boundaries(&ir, ir.meta.range, 4)
            .expect("boundary detection should succeed");
        assert_eq!(crossing.len(), 1);
        assert_eq!(crossing[0].id, "s-crossing");
        assert_eq!(crossing[0].crossed_boundaries, vec![100, 200]);
    }

    #[test]
    fn event_range_crossing_a_page_boundary_is_detected() {
        let mut ir = no_crossing_ir();
        ir.items.push(event_range("er-crossing", "a", 290, 310));

        let crossing = items_crossing_boundaries(&ir, ir.meta.range, 4)
            .expect("boundary detection should succeed");
        assert_eq!(crossing.len(), 1);
        assert_eq!(crossing[0].id, "er-crossing");
        assert_eq!(crossing[0].crossed_boundaries, vec![300]);
    }

    #[test]
    fn no_crossing_items_report_empty_list() {
        let ir = no_crossing_ir();
        let crossing = items_crossing_boundaries(&ir, ir.meta.range, 4)
            .expect("boundary detection should succeed");
        assert!(
            crossing.is_empty(),
            "items wholly contained within one segment must not be reported: {crossing:?}"
        );
    }

    #[test]
    fn event_items_are_not_reported_as_crossing() {
        let mut ir = no_crossing_ir();
        ir.items.push(event("e-on-boundary", "a", 200));
        let crossing = items_crossing_boundaries(&ir, ir.meta.range, 4)
            .expect("boundary detection should succeed");
        assert!(
            crossing.iter().all(|c| c.id != "e-on-boundary"),
            "Event items must never appear in boundary-crossing results"
        );
    }

    #[test]
    fn item_touching_a_boundary_at_its_own_edge_is_not_a_crossing() {
        let mut ir = no_crossing_ir();
        ir.items.push(span("s-touches-edge", "a", 50, 100));
        let crossing = items_crossing_boundaries(&ir, ir.meta.range, 4)
            .expect("boundary detection should succeed");
        assert!(
            crossing.iter().all(|c| c.id != "s-touches-edge"),
            "an item whose end lands exactly ON a boundary (not strictly past it) \
             must not be reported as crossing: {crossing:?}"
        );
    }

    // ─── clip_with_continuation_markers ─────────────────────────────────────

    #[test]
    fn clip_markers_are_present_on_every_page_the_item_intersects() {
        let ir = crossing_ir();
        let range = ir.meta.range;

        let page0 = clip_with_continuation_markers(&ir, range, 4, 0).unwrap();
        let m0 = page0.iter().find(|m| m.id == "s-crossing").unwrap();
        assert!(!m0.continues_from_previous_page, "page 0: item starts here");
        assert!(m0.continues_to_next_page, "page 0: item extends past 100");

        let page1 = clip_with_continuation_markers(&ir, range, 4, 1).unwrap();
        let m1 = page1.iter().find(|m| m.id == "s-crossing").unwrap();
        assert!(
            m1.continues_from_previous_page,
            "page 1: item started on page 0"
        );
        assert!(m1.continues_to_next_page, "page 1: item extends past 200");

        let page2 = clip_with_continuation_markers(&ir, range, 4, 2).unwrap();
        let m2 = page2.iter().find(|m| m.id == "s-crossing").unwrap();
        assert!(
            m2.continues_from_previous_page,
            "page 2: item started earlier"
        );
        assert!(
            !m2.continues_to_next_page,
            "page 2: item ends at 220, before 300"
        );

        let page3 = clip_with_continuation_markers(&ir, range, 4, 3).unwrap();
        assert!(page3.iter().all(|m| m.id != "s-crossing"));
    }

    #[test]
    fn clip_markers_page_index_equal_to_page_count_is_rejected_explicitly() {
        let ir = crossing_ir();
        let range = ir.meta.range;
        let err = clip_with_continuation_markers(&ir, range, 4, 4)
            .expect_err("page_index == page_count must be a hard error, not a panic");
        assert!(matches!(
            err,
            TimeRangePaginationError::InvalidPageIndex {
                page_index: 4,
                page_count: 4,
            }
        ));
    }

    #[test]
    fn clip_markers_page_index_greater_than_page_count_is_rejected_explicitly() {
        let ir = crossing_ir();
        let range = ir.meta.range;
        let err = clip_with_continuation_markers(&ir, range, 4, 100)
            .expect_err("page_index > page_count must be a hard error, not a panic");
        assert!(matches!(
            err,
            TimeRangePaginationError::InvalidPageIndex {
                page_index: 100,
                page_count: 4,
            }
        ));
    }

    // ─── paginate_svg_by_time_range ─────────────────────────────────────────

    #[test]
    fn paginate_produces_expected_page_count_and_kind() {
        let ir = no_crossing_ir();
        let result = paginate_svg_by_time_range(&ir, &RenderOptions::default(), 4)
            .expect("pagination should succeed");
        assert_eq!(result.pages.len(), 4);
        assert!(result.pages.iter().all(|p| p.kind == PageKind::Chart));
    }

    #[test]
    fn paginate_reports_boundary_crossing_items() {
        let ir = crossing_ir();
        let result = paginate_svg_by_time_range(&ir, &RenderOptions::default(), 4)
            .expect("pagination should succeed");
        assert_eq!(result.items_crossing_boundaries.len(), 1);
        assert_eq!(result.items_crossing_boundaries[0].id, "s-crossing");
    }

    #[test]
    fn paginate_no_crossing_reports_empty_list() {
        let ir = no_crossing_ir();
        let result = paginate_svg_by_time_range(&ir, &RenderOptions::default(), 4)
            .expect("pagination should succeed");
        assert!(result.items_crossing_boundaries.is_empty());
    }

    #[test]
    fn paginate_zero_page_count_is_rejected_explicitly() {
        let ir = no_crossing_ir();
        let err = paginate_svg_by_time_range(&ir, &RenderOptions::default(), 0)
            .expect_err("page_count=0 must be a hard error");
        assert!(matches!(err, TimeRangePaginationError::InvalidPageCount));
    }

    #[test]
    fn paginate_show_table_true_appends_single_table_page_at_end() {
        let ir = no_crossing_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let result = paginate_svg_by_time_range(&ir, &opts, 4).expect("pagination should succeed");
        assert_eq!(result.pages.len(), 5, "4 chart pages + 1 table page");
        for page in &result.pages[..4] {
            assert_eq!(page.kind, PageKind::Chart);
        }
        assert_eq!(result.pages[4].kind, PageKind::Table);
        let table_svg = &result.pages[4].svg;
        assert!(table_svg.contains("Span s-1"));
        assert!(table_svg.contains("Span s-3"));
        assert!(
            table_svg.contains("Event e-1"),
            "table page must list every IR item, not just one page's items"
        );
    }

    #[test]
    fn paginate_show_table_false_has_no_table_page() {
        let ir = no_crossing_ir();
        let result = paginate_svg_by_time_range(&ir, &RenderOptions::default(), 4)
            .expect("pagination should succeed");
        assert!(result.pages.iter().all(|p| p.kind == PageKind::Chart));
    }

    // ─── issue #711 findings: group band / zigzag / open-ended are already
    // correct on this axis by construction (retained from the spike as
    // regression coverage for the promoted production code) ────────────────

    fn lane_with_group(id: &str, group: &str) -> Lane {
        Lane {
            id: id.to_string(),
            label: format!("Lane {id}"),
            kind: "custom".into(),
            order: 1,
            group: Some(group.to_string()),
            source_span: None,
        }
    }

    #[test]
    fn group_band_spans_full_page_width_on_every_time_range_page_no_truncation() {
        let mut ir = TimelineIr {
            meta: meta(0, 400),
            lanes: vec![lane_with_group("a", "G"), lane_with_group("b", "G")],
            items: vec![span("s-1", "a", 10, 90), span("s-2", "b", 310, 390)],
            imports: vec![],
            sources: vec![],
        };
        ir.meta.range = (0, 400);
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");

        let opts = RenderOptions {
            layout_style: LayoutStyle::GroupBands,
            ..RenderOptions::default()
        };
        for page in &pages {
            let layout =
                LayoutModel::compute(&page.ir, opts.clone()).expect("layout should succeed");
            assert_eq!(layout.group_bands.len(), 1);
            let band = &layout.group_bands[0];
            assert!(
                (band.x - opts.left_gutter).abs() < 0.001,
                "band x should start at left_gutter regardless of page segment"
            );
        }
    }

    #[test]
    fn items_wholly_outside_a_page_segment_still_produce_a_laid_item_with_non_positive_extent() {
        let ir = no_crossing_ir();
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        let last_page = &pages[3];

        let layout = LayoutModel::compute(&last_page.ir, RenderOptions::default())
            .expect("layout should succeed even for a page an item never touches");
        let s1 = layout
            .items
            .iter()
            .find_map(|laid| match laid {
                LaidItem::Span { item, width, .. } if item_id(item) == "s-1" => Some(*width),
                _ => None,
            })
            .expect("s-1 must still be present as a LaidItem on a page it never touches");
        assert!(s1 <= 0.0, "got {s1}");
    }
}
