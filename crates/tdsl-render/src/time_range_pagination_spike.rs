//! Spike (issue #709, parent #662, ADR-0005 D2) for time-range-axis chart
//! pagination: split `meta.range` into N segments and render one chart page
//! per segment.
//!
//! This is a design-verification spike only, **not** wired into any
//! production entry point (mirrors the #651 spike pattern that later became
//! `pagination.rs`'s lane-group axis in #660). Everything here is
//! `#[cfg(test)]`-only and `pub(crate)`.
//!
//! ## Why the lane-group approach doesn't transfer directly
//!
//! `pagination.rs::paginate_svg_by_lane_groups` keeps `meta` (and therefore
//! the time axis) identical across every page and only partitions `lanes` /
//! `items`. Because every item belongs to exactly one lane, that partition is
//! always a clean split — no item ever needs clipping.
//!
//! The time-range axis is different: `meta.range` (plus the `range_start_*` /
//! `range_end_*` precision fields) IS the thing being paginated, and a
//! `Span`/`EventRange` item can legitimately straddle a page boundary (e.g. a
//! centuries-long dynasty spanning two 100-year pages). This module therefore:
//!
//! 1. Builds one `TimelineIr` per segment with `meta.range` rewritten to that
//!    segment's `(start, end)` (see [`split_ir_by_time_range`]). The full
//!    item set is duplicated onto every page IR unchanged — geometric
//!    clipping of items outside `[year_min, year_max]` is already handled by
//!    `layout::primary_axis_segment`'s clamp (see
//!    `layout::tests::span_clamps_to_range`) and by `layout::year_in_range`
//!    for `Event`, so this spike doesn't need to duplicate that logic.
//! 2. Separately *detects* (never silently drops) which `Span`/`EventRange`
//!    items cross an interior segment boundary, via
//!    [`items_crossing_boundaries`] — the time-axis analogue of
//!    `pagination::find_groups_split_across_chunks` /
//!    `ChartPagination::group_bands_split_across_pages`.

use tdsl_core::ir::{Item, TimelineIr};

use crate::layout::LayoutModel;
use crate::layout::LayoutStyle;
use crate::layout::RenderOptions;

/// Error returned by [`split_ir_by_time_range`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum TimeRangePaginationError {
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
}

/// One time-range-axis page: the segment's `(start, end)` boundary and the
/// `TimelineIr` clone whose `meta.range` (and sub-year precision fields) have
/// been rewritten to that segment.
#[derive(Debug)]
pub(crate) struct TimeRangePage {
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
/// synthetic boundary's precision.
///
/// `lanes`/`items`/`imports`/`sources` are duplicated onto every page
/// unchanged (unlike the lane-group axis, this spike does not filter or clip
/// items — see the module doc for why that's deferred to the existing
/// layout-level clamp/skip logic).
pub(crate) fn split_ir_by_time_range(
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
pub(crate) struct BoundaryCrossingItem {
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
/// report — see `event_items_are_not_reported_as_crossing` below for the
/// structural confirmation.
///
/// Mirrors `pagination::find_groups_split_across_chunks` /
/// `ChartPagination::group_bands_split_across_pages`: rather than silently
/// letting each page draw its own truncated bar with no record of the split,
/// callers get an explicit, non-empty list to warn on.
pub(crate) fn items_crossing_boundaries(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::render_svg;
    use tdsl_core::ir::{Lane, Meta, TimelineIr};

    fn meta(start: i64, end: i64) -> Meta {
        Meta {
            title: "time-range pagination spike".into(),
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

    // ─── AC1: split into N segments and render each through the existing
    // LayoutModel::compute → render_svg pipeline ──────────────────────────

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
        // Each page's own span label must actually appear in its own SVG
        // (confirms the pipeline actually laid the item out, not just that
        // rendering didn't crash).
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

    // ─── AC2: per-page time axis ticks are structurally shifted per the
    // segment's range ────────────────────────────────────────────────────

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

        // year_min/year_max must exactly match each page's own segment, not
        // the original 0..400 range.
        for (layout, page) in layouts.iter().zip(&pages) {
            assert_eq!((layout.year_min, layout.year_max), page.range);
        }

        // Tick sets must differ page to page (structurally distinct axes,
        // not four copies of the same 0..400 axis).
        let tick_sets: Vec<Vec<i64>> = layouts.iter().map(LayoutModel::ticks).collect();
        for i in 0..tick_sets.len() {
            for j in (i + 1)..tick_sets.len() {
                assert_ne!(
                    tick_sets[i], tick_sets[j],
                    "page {i} and page {j} must have distinct tick sets"
                );
            }
        }

        // The same absolute year (e.g. year 150, only present on page 1)
        // must map to different pixel x-coordinates across pages that both
        // contain it in their own local coordinate space — confirms the axis
        // origin actually moved, not just the item's own year value.
        let x_page0_at_50 = layouts[0].year_to_x(50);
        let x_page1_at_150 = layouts[1].year_to_x(150);
        assert_eq!(
            x_page0_at_50, x_page1_at_150,
            "year 50 in page 0's local axis and year 150 in page 1's local axis \
             (both 50 years past their page's year_min) must land at the same x, \
             confirming each page's axis origin shifted to its own segment start"
        );
    }

    // ─── AC3: boundary-crossing span/event_range detection ────────────────

    #[test]
    fn span_crossing_a_page_boundary_is_detected() {
        let mut ir = no_crossing_ir();
        // Straddles the 100/200 boundary (interior boundaries for 4 pages of
        // 0..400 are 100, 200, 300).
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
        ir.items.push(Item::EventRange {
            id: "er-crossing".into(),
            lane: "a".into(),
            start: 290,
            end: 310,
            label: "ER crossing".into(),
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
        });

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

    /// AC3's explicit "Event is out of scope" requirement: an `Event` placed
    /// exactly at a boundary year is structurally never reported by
    /// `items_crossing_boundaries` (it has no `[start, end]` extent to
    /// straddle a boundary with) — confirming the exclusion is by
    /// construction, not just untested.
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
        // s-2 ends exactly at 190 (inside page 1), s-3 starts exactly at
        // page-boundary-adjacent 210 — neither touches a boundary AT its
        // start/end, but this test pins the strict-interior semantics
        // (start < boundary < end) using an item whose end lands exactly on
        // a boundary.
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

    // ─── AC4: N == 0 / empty range are hard errors, not silent no-ops ─────

    #[test]
    fn zero_page_count_is_rejected_explicitly() {
        let ir = no_crossing_ir();
        let err = split_ir_by_time_range(&ir, 0)
            .expect_err("page_count=0 must be a hard error, not a silent no-op");
        assert_eq!(err, TimeRangePaginationError::InvalidPageCount);
    }

    #[test]
    fn empty_range_is_rejected_explicitly() {
        let mut ir = no_crossing_ir();
        ir.meta.range = (100, 100);
        let err = split_ir_by_time_range(&ir, 4)
            .expect_err("an empty (start == end) range must be a hard error");
        assert_eq!(
            err,
            TimeRangePaginationError::EmptyRange {
                start: 100,
                end: 100
            }
        );
    }

    #[test]
    fn inverted_range_is_rejected_explicitly() {
        let mut ir = no_crossing_ir();
        ir.meta.range = (100, 50);
        let err = split_ir_by_time_range(&ir, 4)
            .expect_err("an inverted (end < start) range must be a hard error");
        assert_eq!(
            err,
            TimeRangePaginationError::EmptyRange {
                start: 100,
                end: 50
            }
        );
    }

    #[test]
    fn range_too_narrow_for_page_count_is_rejected_explicitly() {
        // 2-year range split into 5 pages cannot produce 5 non-empty
        // integer-year segments; must not silently render zero-width pages.
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

    // ─── AC5 (spike hygiene): the range/precision-field rewrite itself ─────

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

    // ─── issue #711: group band / gantt / zigzag / open-ended interaction
    // findings for ADR-0005 §3 ───────────────────────────────────────────

    fn lane_with_group(id: &str, group: &str) -> tdsl_core::ir::Lane {
        tdsl_core::ir::Lane {
            id: id.to_string(),
            label: format!("Lane {id}"),
            kind: "custom".into(),
            order: 1,
            group: Some(group.to_string()),
            source_span: None,
        }
    }

    /// Unlike the lane-group axis (`pagination::paginate_svg_by_lane_groups`,
    /// which filters `lanes` per page and therefore truncates a group band
    /// whose member lanes land on different pages — ADR-0005 §"Spike 実施結果"
    /// / issue #660's `group_bands_split_across_pages` warning), the
    /// time-range axis never filters `lanes` at all: every page's `TimelineIr`
    /// carries the full, unmodified lane list (see `split_ir_by_time_range`).
    /// So a group band's lane membership is identical on every page, and its
    /// primary-axis extent is derived purely from `total_width` /
    /// `left_gutter` / `right_margin` (`layout::compute_group_bands`), not
    /// from any item's time extent — it is drawn full-width on every page by
    /// construction, with no split/truncation concept to warn about.
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
            assert_eq!(
                layout.group_bands.len(),
                1,
                "both lanes stay in group G on every page (lanes are never filtered \
                 on the time-range axis)"
            );
            let band = &layout.group_bands[0];
            // Horizontal orientation: band.x == left_gutter, band spans to
            // total_width - right_margin regardless of which 100-year segment
            // this page covers or whether either lane has an item on it.
            assert!(
                (band.x - opts.left_gutter).abs() < 0.001,
                "band x should start at left_gutter regardless of page segment"
            );
        }
    }

    /// `Span`/`EventRange` layout (`layout::compute_item`) never excludes an
    /// item whose extent falls wholly outside `[year_min, year_max]` from the
    /// laid-item list the way `Event` does (`year_in_range` early-return) —
    /// it always pushes a `LaidItem`, relying on `primary_axis_segment`'s
    /// clamp to collapse it to a non-positive-width bar. On the time-range
    /// axis, every page's `TimelineIr` carries every item unchanged (no
    /// per-page item filtering), so a page whose segment an item never
    /// touches still computes (and would, unless the SVG emitter itself
    /// skips zero/negative-width bars) a degenerate `LaidItem` for it. This
    /// is a previously-undocumented cost/risk for ADR-0005 §3's GO/NO-GO
    /// material: no crash and no incorrect on-page visual (the emitted
    /// bar has zero or negative width), but every page still runs full
    /// layout math for every off-page item.
    #[test]
    fn items_wholly_outside_a_page_segment_still_produce_a_laid_item_with_non_positive_extent() {
        let ir = no_crossing_ir(); // s-1 sits in [10, 90], entirely within page 0 (0..100)
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        let last_page = &pages[3]; // covers 300..400; s-1 never touches this segment

        let layout = LayoutModel::compute(&last_page.ir, RenderOptions::default())
            .expect("layout should succeed even for a page an item never touches");
        let s1 = layout
            .items
            .iter()
            .find_map(|laid| match laid {
                crate::layout::LaidItem::Span { item, width, .. } if item_id(item) == "s-1" => {
                    Some(*width)
                }
                _ => None,
            })
            .expect("s-1 must still be present as a LaidItem on a page it never touches");
        assert!(
            s1 <= 0.0,
            "an item wholly outside the page's segment must clamp to non-positive width, \
             not a positive/garbage width: got {s1}"
        );
    }

    fn item_id(item: &Item) -> &str {
        match item {
            Item::Span { id, .. } | Item::Event { id, .. } | Item::EventRange { id, .. } => id,
        }
    }

    /// `layout::assign_zigzag_parity` sorts by `(lane, start_frac)` over the
    /// *full* `ir.items` — and because the time-range axis duplicates the
    /// full unfiltered item list onto every page (unlike the lane-group axis,
    /// which filters both `lanes` and `items` per page), a given item's
    /// zigzag parity is computed from the same global chronological order on
    /// every page. This is a positive finding for ADR-0005 §3: zigzag does
    /// NOT suffer the lane-axis's "silently recomputed on a page subset"
    /// problem — parity for a shared item is provably identical across pages.
    #[test]
    fn zigzag_parity_for_a_shared_item_is_identical_across_time_range_pages() {
        let mut ir = no_crossing_ir();
        // Add a second item in lane "a" so zigzag parity is non-trivial
        // (alternates true/false by chronological order within the lane).
        ir.items.push(span("s-4", "a", 250, 260));
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");

        let opts = RenderOptions {
            layout_style: LayoutStyle::Zigzag,
            ..RenderOptions::default()
        };
        // s-3 (lane a, [210, 290]) lives wholly on page 2 (200..300) but is
        // duplicated (as an off-page degenerate item) onto every page's IR;
        // its zigzag cross-axis offset sign must not depend on which page
        // computed it.
        let mut offsets = Vec::new();
        for page in &pages {
            let layout =
                LayoutModel::compute(&page.ir, opts.clone()).expect("layout should succeed");
            let offset = layout.items.iter().find_map(|laid| match laid {
                crate::layout::LaidItem::Span { item, y, .. } if item_id(item) == "s-3" => Some(*y),
                _ => None,
            });
            offsets.push(offset.expect("s-3 must be present on every page"));
        }
        for w in offsets.windows(2) {
            assert!(
                (w[0] - w[1]).abs() < 0.001,
                "s-3's zigzag cross-axis offset must be identical across pages: {offsets:?}"
            );
        }
    }

    /// `end_open` is a static per-item bool duplicated unchanged onto every
    /// page's `TimelineIr` (`split_ir_by_time_range` doesn't touch item
    /// fields) — so an open-ended span's tooltip reads "進行中" (ongoing) on
    /// every page it's laid out on, including pages whose segment sits
    /// entirely before the resolved `now` end year. This confirms the "進行中"
    /// arrow/label collision-with-page-boundary question raised in ADR-0005
    /// §3's open-ended bullet is really the same already-solved problem as
    /// §2 (page-boundary clipping of a long-running span): the label is a
    /// per-item tooltip property, not something that needs new per-boundary
    /// logic.
    #[test]
    fn open_ended_span_reads_ongoing_on_every_page_it_is_laid_out_on() {
        let mut ir = no_crossing_ir();
        ir.items.push(Item::Span {
            id: "s-open".into(),
            lane: "a".into(),
            start: 50,
            end: 999, // resolved-at-parse-time placeholder year; end_open governs display
            label: "Open span".into(),
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
            end_open: true,
            source_span: None,
        });
        let pages = split_ir_by_time_range(&ir, 4).expect("split should succeed");
        // Page 0 (0..100) is entirely before the resolved end (999); the span
        // is still clamped/drawn there (it starts at year 50) and must still
        // read "進行中", not a resolved end date.
        let layout =
            LayoutModel::compute(&pages[0].ir, RenderOptions::default()).expect("layout ok");
        let tooltip = layout
            .items
            .iter()
            .find_map(|laid| match laid {
                crate::layout::LaidItem::Span { item, tooltip, .. }
                    if item_id(item) == "s-open" =>
                {
                    Some(tooltip.clone())
                }
                _ => None,
            })
            .expect("s-open must be present on page 0");
        assert!(
            tooltip.contains("進行中"),
            "open-ended span tooltip must read 進行中 on every page it appears on: {tooltip:?}"
        );
    }
}
