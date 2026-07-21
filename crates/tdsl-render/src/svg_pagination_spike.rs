//! Spike prototype for issue #651 (ADR 0005 D2): lane-group-based pagination
//! of the SVG chart body.
//!
//! This module is intentionally **not** wired into `tdsl-cli` or the WebUI.
//! It exists to validate the ADR 0005 D2 recommendation ("prototype lane
//! group pagination first, since it needs no span/event_range clipping")
//! before any CLI flag / production integration is designed. See
//! `docs/adr/0005-timeline-chart-pagination.md` for the write-up of what was
//! learned here.
//!
//! ## Approach
//!
//! The time axis (`Meta::range` and friends) stays common across all pages
//! (ADR 0005 §1 "lane グループで分割" row). Lanes are sorted the same way
//! [`crate::layout::LayoutModel::compute`] orders them (`(order, id)`), then
//! chunked into groups of `lanes_per_page` lanes. For each chunk a filtered
//! `TimelineIr` is built (same `meta`, only the chunk's lanes, only items
//! whose lane belongs to the chunk) and rendered through the existing
//! `LayoutModel::compute` + `render_svg` pipeline unmodified.
//!
//! Because every `Item` belongs to exactly one lane (`Item::lane` is a single
//! `String`, not a range of lanes), and pages are a partition of lanes, this
//! approach never needs to clip a `Span`/`EventRange` bar — every item is
//! wholly contained in exactly one page by construction. This is the
//! structural claim the accompanying tests verify.

use std::collections::HashSet;

use tdsl_core::ir::{Item, Lane, TimelineIr};

use crate::layout::{LayoutModel, RenderOptions};
use crate::svg::render_svg;

/// One rendered page of a lane-group-paginated chart.
#[derive(Debug)]
pub(crate) struct PaginatedPage {
    /// Lane IDs assigned to this page, in the same order used for layout.
    pub(crate) lane_ids: Vec<String>,
    /// Rendered standalone SVG for this page's lane subset.
    pub(crate) svg: String,
}

/// Result of a full lane-group pagination pass.
#[derive(Debug)]
pub(crate) struct PaginationResult {
    pub(crate) pages: Vec<PaginatedPage>,
    /// Group labels (from `Lane::group`) whose contiguous lane run was split
    /// across two or more pages by this chunking. Known limitation (ADR 0005
    /// Spike write-up): a `group_band` (`LayoutStyle::GroupBands`) that
    /// crosses a page boundary is *not* reconstructed as a single visual
    /// band across pages — each page independently recomputes its own
    /// `group_bands` from only the lanes visible on that page, so the band
    /// is truncated (or, if only one lane of the group lands on a page,
    /// rendered as if it were a group of one).
    pub(crate) group_bands_split_across_pages: Vec<String>,
}

/// Error returned by [`paginate_by_lane_groups`].
///
/// `thiserror` is not used here because this module is compiled only under
/// `#[cfg(test)]` (see module docs), and `tdsl-render`'s `thiserror`
/// dependency is gated behind the optional `pdf`/`png` features; pulling it
/// in unconditionally for a test-only spike would widen the crate's default
/// dependency footprint for no production benefit.
#[derive(Debug)]
pub(crate) enum PaginationSpikeError {
    InvalidChunkSize,
    Render(std::fmt::Error),
}

impl std::fmt::Display for PaginationSpikeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidChunkSize => write!(f, "lanes_per_page must be >= 1"),
            Self::Render(e) => write!(f, "SVG rendering failed: {e}"),
        }
    }
}

impl std::error::Error for PaginationSpikeError {}

impl From<std::fmt::Error> for PaginationSpikeError {
    fn from(e: std::fmt::Error) -> Self {
        Self::Render(e)
    }
}

/// Split `ir`'s lanes into groups of `lanes_per_page` (ordered the same way
/// `LayoutModel` orders lanes: `(order, id)`), and render one SVG chart per
/// group. The time axis (`meta.range` and precision fields) is shared across
/// all pages unchanged.
pub(crate) fn paginate_by_lane_groups(
    ir: &TimelineIr,
    opts: &RenderOptions,
    lanes_per_page: usize,
) -> Result<PaginationResult, PaginationSpikeError> {
    if lanes_per_page == 0 {
        return Err(PaginationSpikeError::InvalidChunkSize);
    }

    let mut lanes_ordered: Vec<&Lane> = ir.lanes.iter().collect();
    lanes_ordered.sort_by_key(|l| (l.order, l.id.clone()));

    let chunks: Vec<Vec<&Lane>> = lanes_ordered
        .chunks(lanes_per_page)
        .map(<[&Lane]>::to_vec)
        .collect();

    let group_bands_split_across_pages = find_groups_split_across_chunks(&lanes_ordered, &chunks);

    let mut pages = Vec::with_capacity(chunks.len());
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
        let layout = LayoutModel::compute(&page_ir, opts.clone());
        let svg = render_svg(&layout)?;
        pages.push(PaginatedPage {
            lane_ids: chunk.iter().map(|l| l.id.clone()).collect(),
            svg,
        });
    }

    Ok(PaginationResult {
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
            title: "pagination spike".into(),
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
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        assert_eq!(result.pages.len(), 2, "4 lanes / 2 per page = 2 pages");
        assert_eq!(result.pages[0].lane_ids, vec!["a", "b"]);
        assert_eq!(result.pages[1].lane_ids, vec!["c", "d"]);
    }

    #[test]
    fn each_lane_height_page_covers_only_its_assigned_lanes() {
        let ir = four_lane_ir();
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 2)
            .expect("pagination should succeed");
        for page in &result.pages {
            assert_eq!(page.lane_ids.len(), 2, "each page should hold 2 lanes");
        }
    }

    /// Structural test for the "no span/event_range clipping needed" claim
    /// (ADR 0005 D2): every item's label appears on exactly one page's SVG,
    /// never split or duplicated across pages.
    #[test]
    fn every_item_appears_on_exactly_one_page() {
        let ir = four_lane_ir();
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 2)
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
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 10)
            .expect("pagination should succeed");
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages[0].lane_ids, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn zero_lanes_per_page_is_rejected_explicitly() {
        let ir = four_lane_ir();
        let err = paginate_by_lane_groups(&ir, &RenderOptions::default(), 0)
            .expect_err("lanes_per_page=0 must be a hard error, not a silent no-op");
        assert!(matches!(err, PaginationSpikeError::InvalidChunkSize));
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
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 2)
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
        let result = paginate_by_lane_groups(&ir, &RenderOptions::default(), 3)
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
        let result = paginate_by_lane_groups(&ir, &opts, 2).expect("pagination should succeed");
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
}
