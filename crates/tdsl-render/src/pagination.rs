//! Lane-group-based pagination of the SVG chart body (issue #660, ADR 0005 D2).
//!
//! Promoted from the `#[cfg(test)]`-only spike (`svg_pagination_spike.rs`,
//! issue #651) into a production API. See
//! `docs/adr/0005-timeline-chart-pagination.md` for the design history and
//! `docs/adr/0005-timeline-chart-pagination.md`'s "実装時の決定（issue #660）"
//! section for the finalized CLI/behavior decisions.
//!
//! ## Approach
//!
//! The time axis (`Meta::range` and friends) stays common across all chart
//! pages. Lanes are sorted the same way [`crate::layout::LayoutModel::compute`]
//! orders them (`(order, id)`), then chunked into groups of `lanes_per_page`
//! lanes. For each chunk a filtered `TimelineIr` is built (same `meta`, only
//! the chunk's lanes, only items whose lane belongs to the chunk) and
//! rendered through the existing `LayoutModel::compute` + `render_svg`
//! pipeline unmodified.
//!
//! Because every `Item` belongs to exactly one lane (`Item::lane` is a single
//! `String`, not a range of lanes), and pages are a partition of lanes, this
//! approach never needs to clip a `Span`/`EventRange` bar — every item is
//! wholly contained in exactly one page by construction.
//!
//! If `opts.show_table` is set, one additional table page (covering the
//! *entire* IR's items, not just the last chart page's lanes) is appended
//! after all chart pages.

use std::collections::HashSet;

use tdsl_core::ir::{Item, Lane, TimelineIr};

use crate::RenderError;
use crate::layout::{LayoutModel, RenderOptions, TABLE_ROW_HEIGHT, collect_table_rows};
use crate::svg::{render_svg, render_table_page_svg};

/// Kind of a rendered [`ChartPage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// A page showing a subset of lanes (and their items) as a chart.
    Chart,
    /// The single trailing page listing every IR item as a table (only
    /// produced when `RenderOptions::show_table` is true).
    Table,
}

/// One rendered page produced by [`paginate_svg_by_lane_groups`].
#[derive(Debug)]
pub struct ChartPage {
    /// Lane IDs assigned to this page, in the same order used for layout.
    /// Empty for [`PageKind::Table`] pages.
    pub lane_ids: Vec<String>,
    /// Rendered standalone SVG for this page.
    pub svg: String,
    pub kind: PageKind,
}

/// Result of a full lane-group pagination pass.
#[derive(Debug)]
pub struct ChartPagination {
    pub pages: Vec<ChartPage>,
    /// Group labels (from `Lane::group`) whose contiguous lane run was split
    /// across two or more chart pages by this chunking. Callers MUST warn
    /// (not silently ignore) when this is non-empty (implementation-strict.md
    /// §1 "Explicit error over silent fallback").
    pub group_bands_split_across_pages: Vec<String>,
}

/// Error returned by [`paginate_svg_by_lane_groups`].
#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    #[error("lanes_per_page must be >= 1")]
    InvalidLanesPerPage,
    /// An item referenced a `lane` ID that has no corresponding `Lane`
    /// declaration in `ir.lanes`. Rather than silently dropping the item from
    /// every page (as a plain filter would), this is a hard error
    /// (implementation-strict.md: "Explicit error over silent fallback").
    #[error("item references unknown lane {lane:?}")]
    UnknownLane { lane: String },
    #[error("SVG rendering failed: {0}")]
    Render(#[from] RenderError),
}

impl From<std::fmt::Error> for PaginationError {
    fn from(err: std::fmt::Error) -> Self {
        Self::Render(RenderError::from(err))
    }
}

/// Split `ir`'s lanes into groups of `lanes_per_page` (ordered the same way
/// `LayoutModel` orders lanes: `(order, id)`), and render one SVG chart page
/// per group. The time axis (`meta.range` and precision fields) is shared
/// across all pages unchanged.
///
/// If `opts.show_table` is true, a single trailing [`PageKind::Table`] page
/// listing every item in `ir` (not limited to the last chart page's lanes)
/// is appended.
pub fn paginate_svg_by_lane_groups(
    ir: &TimelineIr,
    opts: &RenderOptions,
    lanes_per_page: usize,
) -> Result<ChartPagination, PaginationError> {
    if lanes_per_page == 0 {
        return Err(PaginationError::InvalidLanesPerPage);
    }

    let mut lanes_ordered: Vec<&Lane> = ir.lanes.iter().collect();
    lanes_ordered.sort_by_key(|l| (l.order, l.id.clone()));

    let chunks: Vec<Vec<&Lane>> = lanes_ordered
        .chunks(lanes_per_page)
        .map(<[&Lane]>::to_vec)
        .collect();

    let group_bands_split_across_pages = find_groups_split_across_chunks(&lanes_ordered, &chunks);

    let defined_lane_ids: HashSet<&str> = ir.lanes.iter().map(|lane| lane.id.as_str()).collect();
    if let Some(item) = ir
        .items
        .iter()
        .find(|item| !defined_lane_ids.contains(item_lane_id(item)))
    {
        return Err(PaginationError::UnknownLane {
            lane: item_lane_id(item).to_owned(),
        });
    }

    let mut pages = Vec::with_capacity(chunks.len());
    // Tracks the chart page width so the trailing table page (if any) can
    // share the same page width; all chart pages have the same width because
    // it derives from `meta.range`/`scale`, not from item content.
    let mut chart_width: f64 = 0.0;
    for chunk in &chunks {
        let lane_ids: HashSet<&str> = chunk.iter().map(|l| l.id.as_str()).collect();
        let page_ir = TimelineIr {
            meta: ir.meta.clone(),
            lanes: chunk.iter().map(|l| (*l).clone()).collect(),
            items: ir
                .items
                .iter()
                .filter(|item| lane_ids.contains(item_lane_id(item)))
                .cloned()
                .collect(),
            imports: ir.imports.clone(),
            sources: ir.sources.clone(),
        };
        let chart_opts = RenderOptions {
            show_table: false,
            ..opts.clone()
        };
        let layout = LayoutModel::compute(&page_ir, chart_opts)?;
        chart_width = layout.total_width;
        let svg = render_svg(&layout)?;
        pages.push(ChartPage {
            lane_ids: chunk.iter().map(|l| l.id.clone()).collect(),
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
        // Single table page (multi-page table splitting is #661 scope);
        // height simply grows to fit every row plus the header row and a
        // footer margin so the "1 / 1" footer never overlaps the last row.
        let table_height = TABLE_ROW_HEIGHT * (table_rows.len() as f64 + 1.0) + 24.0;
        let table_svg =
            render_table_page_svg(&table_rows, chart_width as f32, table_height as f32, 1, 1)?;
        pages.push(ChartPage {
            lane_ids: vec![],
            svg: table_svg,
            kind: PageKind::Table,
        });
    }

    Ok(ChartPagination {
        pages,
        group_bands_split_across_pages,
    })
}

/// Detect `Lane::group` labels whose contiguous run of lanes (in
/// `lanes_ordered`) spans more than one chunk. Mirrors the contiguous-run
/// walk in `layout::compute_group_bands`, but only needs the group label and
/// the chunk index of each lane.
fn find_groups_split_across_chunks(lanes_ordered: &[&Lane], chunks: &[Vec<&Lane>]) -> Vec<String> {
    let mut chunk_of_lane: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for (chunk_idx, chunk) in chunks.iter().enumerate() {
        for lane in chunk {
            chunk_of_lane.insert(lane.id.as_str(), chunk_idx);
        }
    }

    let mut split_groups = Vec::new();
    let mut idx = 0usize;
    while idx < lanes_ordered.len() {
        let group = lanes_ordered[idx].group.as_deref();
        let start_idx = idx;
        let mut end_idx = idx;
        while end_idx + 1 < lanes_ordered.len()
            && lanes_ordered[end_idx + 1].group.as_deref() == group
        {
            end_idx += 1;
        }
        if let Some(group_label) = group {
            // Real chunk assignment for every lane is always present (every
            // lane in `lanes_ordered` was placed into exactly one chunk), so
            // `unwrap_or` here only guards against a chunk map lookup miss
            // that indicates a logic bug, not an expected runtime condition;
            // treating it as "not split" keeps this diagnostic helper
            // non-panicking without hiding a real split.
            let first_chunk = chunk_of_lane
                .get(lanes_ordered[start_idx].id.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            let last_chunk = chunk_of_lane
                .get(lanes_ordered[end_idx].id.as_str())
                .copied()
                .unwrap_or(usize::MAX);
            if first_chunk != last_chunk {
                split_groups.push(group_label.to_string());
            }
        }
        idx = end_idx + 1;
    }
    split_groups
}

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutStyle, RenderOptions};
    use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

    fn lane(id: &str, order: i64, group: Option<&str>) -> Lane {
        Lane {
            id: id.to_string(),
            label: format!("Lane {id}"),
            kind: "custom".into(),
            order,
            group: group.map(str::to_string),
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

    fn base_meta() -> Meta {
        Meta {
            title: "pagination test".into(),
            unit: "year".into(),
            range: (0, 1000),
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        }
    }

    /// 4 lanes, no groups, 2 lanes/page → 2 pages, each with a Span whose
    /// label appears in exactly its own page's SVG and never the other's.
    fn four_lane_ir() -> TimelineIr {
        TimelineIr {
            meta: base_meta(),
            lanes: vec![
                lane("a", 1, None),
                lane("b", 2, None),
                lane("c", 3, None),
                lane("d", 4, None),
            ],
            items: vec![
                span("s-a", "a", 0, 100),
                span("s-b", "b", 100, 200),
                event_range("er-c", "c", 200, 300),
                span("s-d", "d", 300, 400),
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn splits_into_expected_page_count() {
        let ir = four_lane_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        assert_eq!(result.pages.len(), 2, "4 lanes / 2 per page = 2 pages");
        assert_eq!(result.pages[0].lane_ids, vec!["a", "b"]);
        assert_eq!(result.pages[1].lane_ids, vec!["c", "d"]);
    }

    #[test]
    fn each_lane_height_page_covers_only_its_assigned_lanes() {
        let ir = four_lane_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        for page in &result.pages {
            assert_eq!(page.lane_ids.len(), 2, "each page should hold 2 lanes");
            assert_eq!(page.kind, PageKind::Chart);
        }
    }

    /// Structural test for the "no span/event_range clipping needed" claim
    /// (ADR 0005 D2): every item's label appears on exactly one page's SVG,
    /// never split or duplicated across pages.
    #[test]
    fn every_item_appears_on_exactly_one_page() {
        let ir = four_lane_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");

        let item_labels = ["Span s-a", "Span s-b", "EventRange er-c", "Span s-d"];
        for label in item_labels {
            let pages_containing: Vec<usize> = result
                .pages
                .iter()
                .enumerate()
                .filter(|(_, p)| p.svg.contains(label))
                .map(|(i, _)| i)
                .collect();
            assert_eq!(
                pages_containing.len(),
                1,
                "label {label:?} should appear on exactly one page, found on {pages_containing:?}"
            );
        }
    }

    #[test]
    fn single_page_when_lanes_per_page_covers_all_lanes() {
        let ir = four_lane_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 10)
            .expect("pagination should succeed");
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].lane_ids, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn zero_lanes_per_page_is_rejected_explicitly() {
        let ir = four_lane_ir();
        let err = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 0)
            .expect_err("lanes_per_page=0 must be a hard error, not a silent no-op");
        assert!(matches!(err, PaginationError::InvalidLanesPerPage));
    }

    /// An item referencing a lane ID absent from `ir.lanes` must fail loudly
    /// rather than being silently dropped from every page's filter.
    #[test]
    fn item_with_unknown_lane_is_rejected_explicitly() {
        let mut ir = four_lane_ir();
        ir.items.push(span("s-ghost", "no-such-lane", 400, 500));
        let err = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect_err("item referencing an undeclared lane must be a hard error");
        assert!(matches!(
            err,
            PaginationError::UnknownLane { lane } if lane == "no-such-lane"
        ));
    }

    /// group band boundary check (issue #651 acceptance criterion): a
    /// `Lane::group` whose contiguous lane run straddles the chunk boundary
    /// is detected and reported, rather than silently producing a truncated
    /// band with no record of the split.
    fn grouped_ir() -> TimelineIr {
        TimelineIr {
            meta: base_meta(),
            lanes: vec![
                lane("a", 1, Some("王朝")),
                lane("b", 2, Some("王朝")),
                lane("c", 3, Some("王朝")),
                lane("d", 4, None),
            ],
            items: vec![
                span("s-a", "a", 0, 100),
                span("s-b", "b", 100, 200),
                span("s-c", "c", 200, 300),
                span("s-d", "d", 300, 400),
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn group_band_split_across_page_boundary_is_detected() {
        // "王朝" group spans lanes a,b,c; with 2 lanes/page that group's run
        // (a,b,c) crosses the page boundary between chunk 0 (a,b) and chunk 1 (c,d).
        let ir = grouped_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        assert_eq!(
            result.group_bands_split_across_pages,
            vec!["王朝".to_string()],
            "group whose lane run crosses a page boundary must be reported, not silently truncated"
        );
    }

    #[test]
    fn group_band_fully_contained_in_one_page_is_not_reported() {
        // All group lanes fit within a single page (3 lanes/page), so no split occurs.
        let ir = grouped_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 3)
            .expect("pagination should succeed");
        assert!(
            result.group_bands_split_across_pages.is_empty(),
            "group fully contained in one page must not be reported as split"
        );
    }

    /// Confirms the *known limitation*: even when a group is split across
    /// pages, each page independently draws its own (truncated) group band
    /// rather than erroring or omitting the band — this is a documented
    /// visual quirk (ADR 0005 Spike write-up), not a crash.
    #[test]
    fn split_group_band_still_renders_a_truncated_band_on_each_page() {
        let ir = grouped_ir();
        let opts = RenderOptions {
            layout_style: LayoutStyle::GroupBands,
            ..RenderOptions::default()
        };
        let result = paginate_svg_by_lane_groups(&ir, &opts, 2).expect("pagination should succeed");
        assert!(!result.group_bands_split_across_pages.is_empty());
        for page in &result.pages {
            assert!(
                page.svg.contains("tdsl-group-band-even")
                    || page.svg.contains("tdsl-group-band-odd"),
                "each page should still render *a* group band, even if truncated: {}",
                page.svg
            );
        }
    }

    // ─── show_table (#660) ──────────────────────────────────────────────────

    #[test]
    fn show_table_true_appends_single_table_page_at_end() {
        let ir = four_lane_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let result = paginate_svg_by_lane_groups(&ir, &opts, 2).expect("pagination should succeed");
        assert_eq!(result.pages.len(), 3, "2 chart pages + 1 table page");
        assert_eq!(result.pages[0].kind, PageKind::Chart);
        assert_eq!(result.pages[1].kind, PageKind::Chart);
        assert_eq!(result.pages[2].kind, PageKind::Table);
    }

    #[test]
    fn show_table_table_page_contains_all_ir_items_not_just_last_chart_page() {
        let ir = four_lane_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let result = paginate_svg_by_lane_groups(&ir, &opts, 2).expect("pagination should succeed");
        let table_svg = &result.pages.last().expect("table page exists").svg;
        // Lanes a,b are on the *first* chart page (not the last), so their
        // items must still appear in the table page's full-IR listing.
        assert!(table_svg.contains("Span s-a"));
        assert!(table_svg.contains("Span s-b"));
        assert!(table_svg.contains("EventRange er-c"));
        assert!(table_svg.contains("Span s-d"));
    }

    #[test]
    fn show_table_false_has_no_table_page() {
        let ir = four_lane_ir();
        let result = paginate_svg_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        assert!(
            result.pages.iter().all(|p| p.kind == PageKind::Chart),
            "show_table=false must not produce a Table page"
        );
    }

    #[test]
    fn show_legend_true_includes_legend_on_each_chart_page() {
        let mut ir = four_lane_ir();
        ir.meta.color_map.insert("dynasty".into(), "#3366cc".into());
        let opts = RenderOptions {
            color_map: ir.meta.color_map.clone(),
            show_legend: true,
            ..RenderOptions::default()
        };
        let result = paginate_svg_by_lane_groups(&ir, &opts, 2).expect("pagination should succeed");
        for page in result.pages.iter().filter(|p| p.kind == PageKind::Chart) {
            assert!(
                page.svg.contains("tdsl-static-legend"),
                "show_legend=true must render the static legend on every chart page: {}",
                page.svg
            );
        }
    }
}
