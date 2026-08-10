use std::collections::HashMap;

use tdsl_core::ir::{Item, Lane, TimelineIr, end_frac, start_frac};

use crate::RenderError;

/// Colorblind-friendly 8-color palette for per-lane fill colors.
///
/// Single source of truth for palette shared by all emitters.
pub(crate) const LANE_PALETTE: &[&str] = &[
    "#4682B4", // steel blue
    "#E67E22", // orange
    "#27AE60", // green
    "#8E44AD", // purple
    "#E74C3C", // red
    "#1ABC9C", // teal
    "#F39C12", // amber
    "#2980B9", // blue
];

/// Timeline layout orientation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Orientation {
    /// Time axis runs left→right; lanes are stacked top→bottom. (default)
    #[default]
    Horizontal,
    /// Time axis runs top→bottom; lanes are arranged left→right.
    Vertical,
}

/// Color/style theme for HTML output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Default,
    Dark,
    Print,
    Pastel,
}

/// Grid line style for the time axis.
///
/// Auxiliary grid lines are drawn at regular intervals to improve readability
/// on long timelines. `None` disables all grid lines (default, preserves
/// existing SVG output unchanged).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GridStyle {
    /// No grid lines (default). SVG output is identical to pre-grid behavior.
    #[default]
    None,
    /// Grid lines every 10 years.
    Decade,
    /// Grid lines every year.
    Year,
    /// Grid lines every month.
    ///
    /// Note: month-grid uses 1/12-year intervals regardless of item precision.
    /// This is a visual aid only and does not require `unit = "month"`.
    Month,
}

/// High-level visual layout style, orthogonal to [`Orientation`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum LayoutStyle {
    /// Standard lane timeline layout (default).
    #[default]
    Timeline,
    /// Draw background blocks spanning contiguous lane groups/eras.
    GroupBands,
    /// Project-management-style layout (#564): forces an emphasized month grid
    /// (`tdsl-grid-gantt` CSS class) and always-on start〜end period labels on
    /// Span/EventRange bars.
    Gantt,
    /// Alternating up/down (zigzag) placement of items within a single lane
    /// (#565), sorted by start time: even-indexed items sit above the lane
    /// axis, odd-indexed items below. Supported only when the timeline has at
    /// most [`ZIGZAG_MAX_LANES`] lanes; otherwise rendering returns an explicit
    /// error (per CLAUDE.md "No silent fallback"). Mutually exclusive with the
    /// #549 bar sub-row stacking: Zigzag is an alternative
    /// overlap-avoidance strategy, so its cross-axis offset replaces (rather
    /// than combines with) the bar_stack_level offset.
    Zigzag,
}

/// Rendering options. Pixel dimensions and styling parameters.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Pixels per year on the horizontal axis.
    pub scale: f64,
    /// Height of each lane in pixels.
    pub lane_height: f64,
    /// Width of the left-hand gutter that holds lane labels.
    pub left_gutter: f64,
    /// Top margin reserved for the time axis.
    pub top_margin: f64,
    /// Right margin.
    pub right_margin: f64,
    /// Bottom margin.
    pub bottom_margin: f64,
    /// Color/style theme.
    pub theme: Theme,
    /// Optional custom CSS (content, not a file path) injected after the theme CSS.
    pub custom_css: Option<String>,
    /// Tag-to-color overrides. Key: tag name, Value: CSS color string (e.g. "#cc0000").
    pub color_map: std::collections::HashMap<String, String>,
    /// Enable interactive mode (zoom, pan, search, legend, detail panel).
    pub interactive: bool,
    /// Custom font-family CSS value for SVG text. When None, uses the built-in CJK-friendly stack.
    pub font_family: Option<String>,
    /// Timeline layout orientation: horizontal (default) or vertical.
    pub orientation: Orientation,
    /// Auxiliary grid line style. `None` (default) disables grid lines entirely.
    pub grid: GridStyle,
    /// High-level visual layout style, orthogonal to `orientation`.
    pub layout_style: LayoutStyle,
    /// When true, a table listing all items (time period, label, lane, tags) is appended
    /// below the timeline. HTML output uses a native `<table>` element (`html.rs`); SVG,
    /// PNG, and PDF output draw the same columns as SVG `<rect>`/`<text>` elements below
    /// the timeline body, with `total_height` expanded to fit (#536).
    pub show_table: bool,
    /// When true, render a static legend panel listing lane palette colors and tag color
    /// overrides. Interactive HTML keeps its existing side legend; SVG/PNG/PDF reserve
    /// space below the timeline body for this panel (#544).
    pub show_legend: bool,
    /// When true, labels (and optionally dates) are always rendered next to Event and EventRange
    /// dots/bars as SVG text elements.  Disabled by default to keep the chart uncluttered.
    pub show_event_labels: bool,
    /// When true (default), lane palette colours are emitted as CSS custom properties
    /// (`var(--tdsl-lane-N, #hex)`) in SVG inline styles, allowing embedding pages to
    /// override lane colours via `:root { --tdsl-lane-N: … }`. Set to false for raster
    /// renderers (`usvg`-based PNG/PDF) that do not support CSS custom properties.
    pub use_css_vars: bool,
    /// When true, a `Span`/`EventRange` whose true `[start, end]` extent is
    /// clipped by `[year_min, year_max]` gets a continuation-marker glyph
    /// drawn at its clipped edge(s) (issue #734, ADR-0005 §2 strategy 1
    /// "クリップ + 継続マーカー"). `false` by default so ordinary
    /// narrow-`range` renders (unrelated to chart pagination) keep their
    /// existing silent-clamp appearance unchanged;
    /// [`crate::time_range_pagination::paginate_svg_by_time_range`] sets this
    /// to `true` for the per-page chart options it builds.
    pub show_boundary_clip_markers: bool,
}

/// Default lane height in pixels. Bar thickness and intra-lane padding scale
/// relative to this baseline, so a lane_height of 60 reproduces the historical
/// (pre-#507) geometry exactly.
pub(crate) const DEFAULT_LANE_HEIGHT: f64 = 60.0;

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            scale: 2.0,
            lane_height: DEFAULT_LANE_HEIGHT,
            left_gutter: 120.0,
            top_margin: 40.0,
            right_margin: 20.0,
            bottom_margin: 20.0,
            theme: Theme::Default,
            custom_css: None,
            color_map: std::collections::HashMap::new(),
            interactive: false,
            font_family: None,
            orientation: Orientation::Horizontal,
            grid: GridStyle::None,
            layout_style: LayoutStyle::Timeline,
            show_table: false,
            show_legend: false,
            show_event_labels: false,
            use_css_vars: true,
            show_boundary_clip_markers: false,
        }
    }
}

/// Pre-computed lane background band geometry.
#[derive(Debug, Clone)]
pub struct LaneBandModel {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// `true` for even-indexed lanes (0-based), `false` for odd.
    pub even: bool,
}

/// Background block spanning a contiguous lane group/era.
pub struct GroupBandModel {
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// `true` for even-indexed groups (0-based), `false` for odd.
    pub even: bool,
}

/// Item kind in its laid-out form (y offset from lane center already applied).
///
/// `color` is the resolved CSS color string (from tag overrides or lane palette).
/// `tooltip` is the formatted tooltip text before XML escaping.
#[derive(Debug, Clone)]
pub enum LaidItem<'a> {
    Span {
        item: &'a Item,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        /// Resolved CSS color (e.g. `"#4682B4"`).
        color: String,
        /// Formatted tooltip text (XML-unescaped).
        tooltip: String,
        /// #564: collision-avoidance stacking level for this bar's always-on Gantt
        /// period label (0 = no offset). Only meaningful when
        /// `RenderOptions.layout_style == LayoutStyle::Gantt`; computed by
        /// [`assign_period_label_stack_levels`] as a post-processing pass, grouped
        /// by lane *and* the #549 bar sub-row (`bar_stack_level`) so the label
        /// offset is relative to the bar's own sub-row placement.
        period_label_stack_level: u8,
        /// #734: `true` when the item's true `start` is before `year_min`
        /// (the bar was clamped at its start edge). Always `false` unless
        /// `RenderOptions::show_boundary_clip_markers` is set.
        continues_from_previous_page: bool,
        /// #734: `true` when the item's true `end` is after `year_max` (the
        /// bar was clamped at its end edge). Always `false` unless
        /// `RenderOptions::show_boundary_clip_markers` is set.
        continues_to_next_page: bool,
    },
    EventRange {
        item: &'a Item,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        /// Resolved CSS color (base; emitters may add fill-opacity).
        color: String,
        /// Formatted tooltip text (XML-unescaped).
        tooltip: String,
        /// #564: see `LaidItem::Span::period_label_stack_level`.
        period_label_stack_level: u8,
        /// #734: see `LaidItem::Span::continues_from_previous_page`.
        continues_from_previous_page: bool,
        /// #734: see `LaidItem::Span::continues_to_next_page`.
        continues_to_next_page: bool,
    },
    Event {
        item: &'a Item,
        x: f64,
        y_top: f64,
        y_bottom: f64,
        y_dot: f64,
        /// Resolved CSS color.
        color: String,
        /// Formatted tooltip text (XML-unescaped).
        tooltip: String,
        /// #537: collision-avoidance stacking level for this event's always-on label
        /// (0 = no offset, i.e. the item's normal position). Only meaningful when
        /// `RenderOptions.show_event_labels` is true; computed by
        /// [`assign_event_label_stack_levels`] as a post-processing pass over all
        /// laid-out items in the same lane.
        label_stack_level: u8,
    },
}

/// A single row of the "all items" table (#536), shared by the HTML `<table>`
/// emitter and the SVG/PNG/PDF `<text>`/`<rect>` table emitter.
pub(crate) struct TableRow {
    /// Sort key: start/time year for ordering.
    pub sort_year: i64,
    /// Sort secondary key: item type order (0=span, 1=event_range, 2=event).
    pub sort_type: u8,
    /// Formatted time period string (e.g. "206 BC〜220" or "1944 Jun 6").
    pub time_str: String,
    pub label: String,
    pub lane_label: String,
    pub tags: String,
}

/// Column header names for the item table, shared by HTML and SVG/PNG/PDF output.
pub(crate) const TABLE_COL_TIME: &str = "時期";
pub(crate) const TABLE_COL_LABEL: &str = "ラベル";
pub(crate) const TABLE_COL_LANE: &str = "レーン";
pub(crate) const TABLE_COL_TAGS: &str = "タグ";

/// Collect and sort all IR items into table rows.
///
/// Sorted by start/time year ascending, then by item type (span > event_range
/// > event), then by label for ties. `lane_label` resolves a lane ID to its
/// > display label (falls back to the lane ID itself when the caller has no
/// > better mapping).
pub(crate) fn collect_table_rows(
    ir: &TimelineIr,
    lane_label: impl Fn(&str) -> String,
) -> Vec<TableRow> {
    let mut rows: Vec<TableRow> = ir
        .items
        .iter()
        .map(|item| match item {
            Item::Span {
                label,
                lane,
                start,
                end,
                tags,
                start_month,
                start_day,
                start_hour,
                start_minute,
                end_month,
                end_day,
                end_hour,
                end_minute,
                ..
            } => TableRow {
                sort_year: *start,
                sort_type: 0,
                time_str: format!(
                    "{}〜{}",
                    format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                    format_date(*end, *end_month, *end_day, *end_hour, *end_minute),
                ),
                label: label.clone(),
                lane_label: lane_label(lane),
                tags: tags.join(", "),
            },
            Item::EventRange {
                label,
                lane,
                start,
                end,
                tags,
                start_month,
                start_day,
                start_hour,
                start_minute,
                end_month,
                end_day,
                end_hour,
                end_minute,
                ..
            } => TableRow {
                sort_year: *start,
                sort_type: 1,
                time_str: format!(
                    "{}〜{}",
                    format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                    format_date(*end, *end_month, *end_day, *end_hour, *end_minute),
                ),
                label: label.clone(),
                lane_label: lane_label(lane),
                tags: tags.join(", "),
            },
            Item::Event {
                label,
                lane,
                time,
                tags,
                time_month,
                time_day,
                time_hour,
                time_minute,
                ..
            } => TableRow {
                sort_year: *time,
                sort_type: 2,
                time_str: format_date(*time, *time_month, *time_day, *time_hour, *time_minute),
                label: label.clone(),
                lane_label: lane_label(lane),
                tags: tags.join(", "),
            },
        })
        .collect();

    rows.sort_by(|a, b| {
        a.sort_year
            .cmp(&b.sort_year)
            .then(a.sort_type.cmp(&b.sort_type))
            .then(a.label.cmp(&b.label))
    });
    rows
}

/// Row height (px) for the SVG/PNG/PDF item table (#536), including the header row.
pub(crate) const TABLE_ROW_HEIGHT: f64 = 22.0;
/// Row height (px) for the static SVG/PNG/PDF legend panel (#544).
pub(crate) const LEGEND_ROW_HEIGHT: f64 = 22.0;
/// Vertical gap (px) between stacked blocks below the timeline body.
pub(crate) const TABLE_TOP_GAP: f64 = 20.0;

/// Pre-computed layout: every coordinate needed by the renderer.
pub struct LayoutModel<'a> {
    pub ir: &'a TimelineIr,
    pub opts: RenderOptions,
    pub year_min: i64,
    pub year_max: i64,
    pub total_width: f64,
    pub total_height: f64,
    pub lanes_ordered: Vec<&'a Lane>,
    pub lane_y: HashMap<String, f64>,
    pub tick_step: i64,
    pub items: Vec<LaidItem<'a>>,
    /// Pre-computed lane background bands (index-ordered, same order as `lanes_ordered`).
    pub lane_bands: Vec<LaneBandModel>,
    /// Pre-computed background bands spanning contiguous lane groups/eras.
    pub group_bands: Vec<GroupBandModel>,
    /// Mapping from lane ID to resolved CSS color (palette-assigned).
    pub lane_colors: HashMap<String, String>,
    /// #536: pre-sorted table rows, populated only when `opts.show_table` is true.
    pub(crate) table_rows: Vec<TableRow>,
    /// Number of static legend rows (title + lanes + tag color overrides), populated
    /// only when `opts.show_legend` is true.
    pub(crate) legend_row_count: usize,
    /// #544: Y coordinate where the static legend panel begins.
    pub(crate) legend_top_y: f64,
    /// #536: Y coordinate (in the *final*, table-inclusive `total_height`) where the
    /// table's header row begins. Only meaningful when `opts.show_table` is true.
    pub(crate) table_top_y: f64,
}

impl<'a> LayoutModel<'a> {
    pub fn compute(ir: &'a TimelineIr, opts: RenderOptions) -> Result<Self, RenderError> {
        let (year_min, year_max) = ir.meta.range;
        let (year_min, year_max) = if year_max > year_min {
            (year_min, year_max)
        } else if year_max == year_min {
            // 同一年内のレンジ（例: range 1939-09..1939-10）: items から導出せず一年幅を確保
            (year_min, year_max + 1)
        } else {
            // range が degenerate な場合は items から導出する。導出もできない
            // (日付を持つ item が 1 つも無い) 場合は、以前は (0, 2000) という
            // 魔法の既定値に握りつぶしていた。不正な range が「西暦 0〜2000 年の
            // 年表」として静かに描画されるため、明示エラーにする (#765)。
            derive_range_from_items(ir).ok_or(RenderError::DegenerateRange {
                start: year_min,
                end: year_max,
            })?
        };

        let mut lanes_ordered: Vec<&Lane> = ir.lanes.iter().collect();
        lanes_ordered.sort_by_key(|l| (l.order, l.id.clone()));

        let is_vertical = opts.orientation == Orientation::Vertical;
        let time_span = (year_max - year_min) as f64;

        // #565: Zigzag is mutually exclusive with the #549 bar sub-row stacking
        // (an alternative overlap-avoidance strategy), and only applies when the
        // timeline has at most ZIGZAG_MAX_LANES lanes. Exceeding the threshold is
        // an explicit error (implementation-strict.md / CLAUDE.md "No silent fallback");
        // callers (CLI, WASM, WebUI) must surface it and stop rendering.
        let zigzag_requested = opts.layout_style == LayoutStyle::Zigzag;
        if zigzag_requested && lanes_ordered.len() > ZIGZAG_MAX_LANES {
            return Err(RenderError::UnsupportedLayout {
                style: "zigzag".to_string(),
                lane_count: lanes_ordered.len(),
                message: "use --chart-pagination or choose a different layout style".to_string(),
            });
        }
        let zigzag_active = zigzag_requested;
        let zigzag_parity = if zigzag_active {
            assign_zigzag_parity(ir)
        } else {
            Vec::new()
        };

        let bar_stack = assign_bar_stack_levels(ir);
        let lane_effective_heights = if zigzag_active {
            // Zigzag reserves symmetric extra cross-axis space on both sides of
            // the lane center instead of the one-directional #549 stacking
            // extension, so every lane gets the same effective height regardless
            // of per-item stack levels.
            lanes_ordered
                .iter()
                .map(|lane| {
                    (
                        lane.id.clone(),
                        opts.lane_height + zigzag_cross_offset(&opts) * 2.0,
                    )
                })
                .collect()
        } else {
            compute_lane_effective_heights(&lanes_ordered, &bar_stack, &opts)
        };

        // lane_y stores:
        //   horizontal → lane center Y coordinate
        //   vertical   → lane center X coordinate (reusing the same field for "lane primary axis")
        //
        // #549: lanes with overlapping Span/EventRange bars reserve additional
        // cross-axis space. The original lane center stays one base half-height
        // from the lane's top/left edge so level 0 preserves the historical
        // coordinates; extra stack rows extend the lane downward/rightward.
        let mut lane_y = HashMap::new();
        let mut lane_start = HashMap::new();
        let mut cursor = if is_vertical {
            opts.left_gutter
        } else {
            opts.top_margin
        };
        for lane in &lanes_ordered {
            lane_start.insert(lane.id.clone(), cursor);
            lane_y.insert(lane.id.clone(), cursor + opts.lane_height / 2.0);
            cursor += lane_effective_heights
                .get(lane.id.as_str())
                .copied()
                .unwrap_or(opts.lane_height);
        }
        let lanes_extent = cursor
            - if is_vertical {
                opts.left_gutter
            } else {
                opts.top_margin
            };

        let (total_width, body_height) = if is_vertical {
            // vertical: time axis is Y, lanes are X columns.
            // lane_height is reused as the base lane column width; overlap stacks
            // expand the effective column width (#549).
            let w = opts.left_gutter + lanes_extent + opts.right_margin;
            let h = opts.top_margin + time_span * opts.scale + opts.bottom_margin;
            (w, h)
        } else {
            let w = opts.left_gutter + time_span * opts.scale + opts.right_margin;
            let h = opts.top_margin + lanes_extent + opts.bottom_margin;
            (w, h)
        };

        // #536: when show_table is enabled, reserve extra vertical space below the
        // timeline body for the "all items" table (SVG/PNG/PDF output; HTML output
        // uses its own separate <table> element and ignores this reservation).
        let table_rows = if opts.show_table {
            let lane_label_lookup = |lane_id: &str| -> String {
                lanes_ordered
                    .iter()
                    .find(|l| l.id == lane_id)
                    .map(|l| l.label.clone())
                    .unwrap_or_else(|| lane_id.to_string())
            };
            collect_table_rows(ir, lane_label_lookup)
        } else {
            Vec::new()
        };
        let legend_row_count = if opts.show_legend {
            // Title row + one row per lane + one row per tag color override.
            1 + lanes_ordered.len() + opts.color_map.len()
        } else {
            0
        };
        let legend_top_y = body_height + TABLE_TOP_GAP;
        let after_legend_y = if opts.show_legend {
            legend_top_y + legend_row_count as f64 * LEGEND_ROW_HEIGHT
        } else {
            body_height
        };
        let table_top_y = after_legend_y + TABLE_TOP_GAP;
        let total_height = match (opts.show_legend, opts.show_table) {
            (true, true) => {
                table_top_y
                    + (table_rows.len() as f64 + 1.0) * TABLE_ROW_HEIGHT
                    + opts.bottom_margin
            }
            (true, false) => after_legend_y + opts.bottom_margin,
            (false, true) => {
                table_top_y
                    + (table_rows.len() as f64 + 1.0) * TABLE_ROW_HEIGHT
                    + opts.bottom_margin
            }
            (false, false) => body_height,
        };

        let tick_step = pick_tick_step(year_max - year_min, opts.scale, AXIS_LABEL_PX);

        // lane_colors: palette-assigned CSS color per lane ID.
        // When use_css_vars is true the value is a CSS custom property reference
        // (var(--tdsl-lane-N, #hex)) so embedding pages can override lane colours.
        // Raster renderers (PNG/PDF) set use_css_vars=false and receive plain hex
        // values, because usvg does not support CSS custom properties.
        let lane_colors: HashMap<String, String> = lanes_ordered
            .iter()
            .enumerate()
            .map(|(idx, lane)| {
                let palette_idx = idx % LANE_PALETTE.len();
                let hex = LANE_PALETTE[palette_idx];
                let color = if opts.use_css_vars {
                    format!("var(--tdsl-lane-{palette_idx}, {hex})")
                } else {
                    hex.to_string()
                };
                (lane.id.clone(), color)
            })
            .collect();

        // lane_bands: background band geometry per lane.
        let lane_bands: Vec<LaneBandModel> = if is_vertical {
            let content_height = body_height - opts.top_margin - opts.bottom_margin;
            lanes_ordered
                .iter()
                .enumerate()
                .map(|(idx, lane)| LaneBandModel {
                    x: lane_start[&lane.id],
                    y: opts.top_margin,
                    width: lane_effective_heights
                        .get(lane.id.as_str())
                        .copied()
                        .unwrap_or(opts.lane_height),
                    height: content_height,
                    even: idx % 2 == 0,
                })
                .collect()
        } else {
            let content_width = total_width - opts.left_gutter - opts.right_margin;
            lanes_ordered
                .iter()
                .enumerate()
                .map(|(idx, lane)| LaneBandModel {
                    x: opts.left_gutter,
                    y: lane_start[&lane.id],
                    width: content_width,
                    height: lane_effective_heights
                        .get(lane.id.as_str())
                        .copied()
                        .unwrap_or(opts.lane_height),
                    even: idx % 2 == 0,
                })
                .collect()
        };

        let group_bands = compute_group_bands(
            &lanes_ordered,
            &lane_start,
            &lane_effective_heights,
            &opts,
            body_height,
            total_width,
        );

        let mut items = Vec::new();
        for (item_idx, item) in ir.items.iter().enumerate() {
            let lane_id = item_lane_id(item);
            // 未知 lane を読み飛ばすと、アイテムが警告なく描画から消える (#765)。
            // pagination.rs は同じ条件を明示エラーにしており、doc コメントで
            // 「a plain filter would silently drop」と書いている。その "plain filter" が
            // ここに残っていたので、同じ扱いに揃える。
            let Some(&lane_axis) = lane_y.get(lane_id) else {
                return Err(RenderError::UnknownLane {
                    lane: lane_id.to_owned(),
                    item: item_display_name(item),
                });
            };
            let item_tags = get_item_tags(item);
            let color = resolve_item_color(
                item_color(item),
                item_tags,
                &opts.color_map,
                lane_id,
                &lane_colors,
            );
            let tooltip = item_tooltip(item);
            let zigzag_offset = if zigzag_active {
                let sign = if zigzag_parity[item_idx] { 1.0 } else { -1.0 };
                Some(sign * zigzag_cross_offset(&opts))
            } else {
                None
            };
            compute_item(
                item,
                &mut items,
                ItemLayoutArgs {
                    lane_axis,
                    bar_stack_level: bar_stack.item_levels[item_idx],
                    zigzag_offset,
                    year_min,
                    year_max,
                    opts: &opts,
                    orientation: opts.orientation.clone(),
                    color,
                    tooltip,
                },
            );
        }

        // #537: avoid horizontally overlapping always-on Event labels within the
        // same lane by stacking colliding labels away from the timeline row.
        if opts.show_event_labels {
            assign_event_label_stack_levels(&mut items, is_vertical);
        }

        // #564: Gantt layout always shows Span/EventRange period labels; avoid
        // horizontally overlapping labels within the same lane sub-row.
        if opts.layout_style == LayoutStyle::Gantt {
            assign_period_label_stack_levels(&mut items, is_vertical);
        }

        Ok(Self {
            ir,
            opts,
            year_min,
            year_max,
            total_width,
            total_height,
            lanes_ordered,
            lane_y,
            tick_step,
            items,
            lane_bands,
            group_bands,
            lane_colors,
            table_rows,
            legend_row_count,
            legend_top_y,
            table_top_y,
        })
    }

    /// Returns `true` when the layout uses a vertical (top-to-bottom time axis) orientation.
    pub fn is_vertical(&self) -> bool {
        self.opts.orientation == Orientation::Vertical
    }

    /// Convert a year to the primary axis coordinate.
    ///
    /// - Horizontal: returns the X coordinate.
    /// - Vertical:   returns the Y coordinate.
    pub fn year_to_primary(&self, year: i64) -> f64 {
        if self.is_vertical() {
            self.opts.top_margin + (year - self.year_min) as f64 * self.opts.scale
        } else {
            year_to_x(year, self.year_min, self.opts.scale, self.opts.left_gutter)
        }
    }

    pub fn year_to_x(&self, year: i64) -> f64 {
        year_to_x(year, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// Month minor-tick positions for `unit=month` timelines.
    ///
    /// Returns `(year, month)` pairs where month ∈ 2..=12 (month=1 overlaps the year tick).
    /// Empty when `unit != "month"` or when the scale is too small to show sub-year ticks.
    pub fn month_ticks(&self) -> Vec<(i64, u8)> {
        if self.ir.meta.unit != "month" {
            return Vec::new();
        }
        if self.opts.scale / 12.0 < 1.0 {
            return Vec::new();
        }
        let mut ticks = Vec::new();
        for year in self.year_min..=self.year_max {
            for month in 2u8..=12 {
                let frac = to_year_frac(year, Some(month), None, None, None);
                if frac < self.year_max as f64 {
                    ticks.push((year, month));
                }
            }
        }
        ticks
    }

    /// X coordinate for a (year, month) fractional position.
    pub fn frac_year_to_x(&self, year: i64, month: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), None, None, None);
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// X coordinate for a (year, month, day) fractional position.
    pub fn day_frac_to_x(&self, year: i64, month: u8, day: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), Some(day), None, None);
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// Day-level minor-tick positions for `unit=day` timelines.
    ///
    /// Returns `(year, month, day)` triples covering the visible range.
    /// 過密回避のため、1日あたりの pixel-per-day が小さい場合は step を 7/14/30 日に切り替える。
    /// `unit != "day"` または 1 日あたりのピクセルが小さすぎる場合は空配列を返す。
    pub fn day_ticks(&self) -> Vec<(i64, u8, u8)> {
        if self.ir.meta.unit != "day" {
            return Vec::new();
        }

        let pixels_per_day = self.opts.scale / 365.25;
        // 最低でも 1px の間隔を要求。完全に詰まる場合は描画しない（年単位描画に委ねる）
        if pixels_per_day < 0.5 {
            return Vec::new();
        }

        // 1 tick あたり最低 6 px を確保するための step（日数）
        let step = if pixels_per_day >= 6.0 {
            1
        } else if pixels_per_day >= 3.0 {
            2
        } else if pixels_per_day >= 1.5 {
            7
        } else {
            30
        };

        let mut ticks = Vec::new();
        for year in self.year_min..=self.year_max {
            for month in 1u8..=12 {
                let last = tdsl_core::ir::days_in_month(year, month);
                let mut day = 1u8;
                while day <= last {
                    if day == 1 || ((day - 1) as usize).is_multiple_of(step) {
                        let frac = to_year_frac(year, Some(month), Some(day), None, None);
                        if frac < self.year_max as f64 {
                            ticks.push((year, month, day));
                        }
                    }
                    day = day.saturating_add(1);
                    if day == 0 {
                        break;
                    }
                }
            }
        }
        ticks
    }

    /// X coordinate for a (year, month, day, hour) fractional position.
    pub fn hour_frac_to_x(&self, year: i64, month: u8, day: u8, hour: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), Some(day), Some(hour), None);
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// X coordinate for a (year, month, day, hour, minute) fractional position.
    pub fn minute_frac_to_x(&self, year: i64, month: u8, day: u8, hour: u8, minute: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), Some(day), Some(hour), Some(minute));
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// X coordinate for a (year, month, day, hour, minute, second) fractional
    /// position (#614, ADR 0003).
    #[allow(clippy::too_many_arguments)]
    pub fn second_frac_to_x(
        &self,
        year: i64,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> f64 {
        let frac = to_year_frac_with_second(
            year,
            Some(month),
            Some(day),
            Some(hour),
            Some(minute),
            Some(second),
        );
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// Hour-level minor-tick positions for `unit=hour` timelines (#556).
    ///
    /// Returns `(year, month, day, hour)` quadruples covering the visible range.
    /// Density-controlled thinning (1h → 3h → 6h → 12h) mirrors `day_ticks()`'s
    /// pattern. Empty when `unit != "hour"` or the scale is too small to show
    /// even 12h ticks.
    pub fn hour_ticks(&self) -> Vec<(i64, u8, u8, u8)> {
        if self.ir.meta.unit != "hour" {
            return Vec::new();
        }

        let pixels_per_hour = self.opts.scale / (365.25 * 24.0);
        // Require at least ~6px per tick at the coarsest step (12h); below that,
        // don't render (falls back to whatever coarser ticks are visible).
        if pixels_per_hour * 12.0 < 6.0 {
            return Vec::new();
        }

        let step: u8 = if pixels_per_hour >= 6.0 {
            1
        } else if pixels_per_hour >= 2.0 {
            3
        } else if pixels_per_hour >= 1.0 {
            6
        } else {
            12
        };

        let Some((mut current, end)) = self.subday_range_bounds() else {
            return Vec::new();
        };

        let mut ticks = Vec::new();
        while current <= end {
            let (year, month, day, hour, _) = current;
            ticks.push((year, month, day, hour));
            current = advance_time_minutes(current, i64::from(step) * 60);
        }
        ticks
    }

    /// Minute-level minor-tick positions for `unit=minute` timelines (#556).
    ///
    /// Returns `(year, month, day, hour, minute)` quintuples. Density-controlled
    /// thinning (1min → 5min → 15min → 30min) mirrors `hour_ticks()`. Empty when
    /// `unit != "minute"` or the scale is too small to show even 30min ticks.
    pub fn minute_ticks(&self) -> Vec<(i64, u8, u8, u8, u8)> {
        if self.ir.meta.unit != "minute" {
            return Vec::new();
        }

        let pixels_per_minute = self.opts.scale / (365.25 * 24.0 * 60.0);
        if pixels_per_minute * 30.0 < 6.0 {
            return Vec::new();
        }

        let step: u8 = if pixels_per_minute >= 6.0 {
            1
        } else if pixels_per_minute >= 1.2 {
            5
        } else if pixels_per_minute >= 0.4 {
            15
        } else {
            30
        };

        let Some((mut current, end)) = self.subday_range_bounds() else {
            return Vec::new();
        };

        let mut ticks = Vec::new();
        while current <= end {
            let (year, month, day, hour, minute) = current;
            ticks.push((year, month, day, hour, minute));
            current = advance_time_minutes(current, i64::from(step));
        }
        ticks
    }

    /// Second-level minor-tick positions for `unit=second` timelines (#614,
    /// ADR 0003).
    ///
    /// Returns `(year, month, day, hour, minute, second)` sextuples.
    /// Density-controlled thinning (1s → 5s → 15s → 30s) mirrors
    /// `minute_ticks()`. Empty when `unit != "second"` or the scale is too
    /// small to show even 30s ticks.
    pub fn second_ticks(&self) -> Vec<(i64, u8, u8, u8, u8, u8)> {
        if self.ir.meta.unit != "second" {
            return Vec::new();
        }

        let pixels_per_second = self.opts.scale / (365.25 * 24.0 * 60.0 * 60.0);
        if pixels_per_second * 30.0 < 6.0 {
            return Vec::new();
        }

        let step: u8 = if pixels_per_second >= 6.0 {
            1
        } else if pixels_per_second >= 1.2 {
            5
        } else if pixels_per_second >= 0.4 {
            15
        } else {
            30
        };

        let Some((mut current, end)) = self.subday_range_bounds_sec() else {
            return Vec::new();
        };

        let mut ticks = Vec::new();
        while current <= end {
            ticks.push(current);
            current = advance_time_seconds(current, i64::from(step));
        }
        ticks
    }

    /// Declared date/time bounds for sub-day axes. Requires at least month/day
    /// precision on both range endpoints; otherwise `unit hour/minute` has no
    /// meaningful bounded tick domain and returns no sub-day ticks instead of
    /// exploding to an entire padded year.
    fn subday_range_bounds(&self) -> Option<(TimeTuple, TimeTuple)> {
        let meta = &self.ir.meta;
        Some((
            (
                meta.range.0,
                meta.range_start_month?,
                meta.range_start_day?,
                meta.range_start_hour.unwrap_or(0),
                meta.range_start_minute.unwrap_or(0),
            ),
            (
                meta.range.1,
                meta.range_end_month?,
                meta.range_end_day?,
                meta.range_end_hour.unwrap_or(23),
                meta.range_end_minute.unwrap_or(59),
            ),
        ))
    }

    /// Second-precision variant of [`Self::subday_range_bounds`] for `unit
    /// second` (#614). Requires month/day precision on both endpoints; hour
    /// defaults to 0/23, minute defaults to 0/59, second defaults to 0/59 when
    /// unspecified (mirrors `subday_range_bounds`'s hour/minute defaults).
    fn subday_range_bounds_sec(&self) -> Option<(TimeTupleSec, TimeTupleSec)> {
        let meta = &self.ir.meta;
        Some((
            (
                meta.range.0,
                meta.range_start_month?,
                meta.range_start_day?,
                meta.range_start_hour.unwrap_or(0),
                meta.range_start_minute.unwrap_or(0),
                meta.range_start_second.unwrap_or(0),
            ),
            (
                meta.range.1,
                meta.range_end_month?,
                meta.range_end_day?,
                meta.range_end_hour.unwrap_or(23),
                meta.range_end_minute.unwrap_or(59),
                meta.range_end_second.unwrap_or(59),
            ),
        ))
    }

    /// Tick positions (year values) within [year_min, year_max], inclusive of year_min if aligned.
    pub fn ticks(&self) -> Vec<i64> {
        let step = self.tick_step.max(1);
        let first = div_floor(self.year_min, step) * step;
        let mut ticks = Vec::new();
        let mut y = first;
        while y <= self.year_max {
            if y >= self.year_min {
                ticks.push(y);
            }
            y += step;
        }
        ticks
    }

    /// The `GridStyle` actually used for grid-line rendering (#564).
    ///
    /// `LayoutStyle::Gantt` forces at least a `GridStyle::Month`-equivalent grid
    /// so the project-management-style emphasized grid is always visible; an
    /// explicit `--grid` choice finer than month (there is none coarser is
    /// possible, e.g. `Year`/`Decade`) is still honored as-is. Any other layout
    /// style leaves `opts.grid` untouched.
    pub fn effective_grid_style(&self) -> GridStyle {
        if self.opts.layout_style == LayoutStyle::Gantt && self.opts.grid == GridStyle::None {
            GridStyle::Month
        } else {
            self.opts.grid.clone()
        }
    }

    /// Grid line positions for the current `GridStyle`.
    ///
    /// Returns fractional year values (f64) covering [year_min, year_max].
    /// - `GridStyle::None`   → empty (no grid lines drawn)
    /// - `GridStyle::Decade` → one position per 10 years
    /// - `GridStyle::Year`   → one position per year
    /// - `GridStyle::Month`  → one position per 1/12 year (12 per year)
    ///
    /// Positions that coincide with existing axis ticks are included; the SVG
    /// renderer draws grid lines behind tick marks so duplicates are invisible.
    ///
    /// #564: when `layout_style == Gantt` and `grid == GridStyle::None`, this
    /// returns `GridStyle::Month`-equivalent positions (see `effective_grid_style`).
    pub fn grid_positions(&self) -> Vec<f64> {
        match self.effective_grid_style() {
            GridStyle::None => Vec::new(),
            GridStyle::Decade => {
                let first = div_floor(self.year_min, 10) * 10;
                let mut positions = Vec::new();
                let mut y = first;
                while y <= self.year_max {
                    if y >= self.year_min {
                        positions.push(y as f64);
                    }
                    y += 10;
                }
                positions
            }
            GridStyle::Year => (self.year_min..=self.year_max).map(|y| y as f64).collect(),
            GridStyle::Month => {
                let mut positions = Vec::new();
                for year in self.year_min..=self.year_max {
                    for month in 0u8..12 {
                        let frac = year as f64 + month as f64 / 12.0;
                        if frac >= self.year_min as f64 && frac <= self.year_max as f64 {
                            positions.push(frac);
                        }
                    }
                }
                positions
            }
        }
    }
}

type TimeTuple = (i64, u8, u8, u8, u8);

fn advance_time_minutes(
    (mut year, mut month, mut day, mut hour, mut minute): TimeTuple,
    delta_minutes: i64,
) -> TimeTuple {
    let mut remaining = delta_minutes.max(0);
    while remaining > 0 {
        let step = remaining.min(1);
        minute += step as u8;
        if minute >= 60 {
            minute = 0;
            hour += 1;
            if hour >= 24 {
                hour = 0;
                day += 1;
                let last = tdsl_core::ir::days_in_month(year, month);
                if day > last {
                    day = 1;
                    month += 1;
                    if month > 12 {
                        month = 1;
                        year += 1;
                    }
                }
            }
        }
        remaining -= step;
    }
    (year, month, day, hour, minute)
}

type TimeTupleSec = (i64, u8, u8, u8, u8, u8);

/// Like [`advance_time_minutes`] but advances whole seconds, for `unit second`
/// tick generation (#614, ADR 0003).
fn advance_time_seconds(
    (mut year, mut month, mut day, mut hour, mut minute, mut second): TimeTupleSec,
    delta_seconds: i64,
) -> TimeTupleSec {
    let mut remaining = delta_seconds.max(0);
    while remaining > 0 {
        let step = remaining.min(1);
        second += step as u8;
        if second >= 60 {
            second = 0;
            minute += 1;
            if minute >= 60 {
                minute = 0;
                hour += 1;
                if hour >= 24 {
                    hour = 0;
                    day += 1;
                    let last = tdsl_core::ir::days_in_month(year, month);
                    if day > last {
                        day = 1;
                        month += 1;
                        if month > 12 {
                            month = 1;
                            year += 1;
                        }
                    }
                }
            }
        }
        remaining -= step;
    }
    (year, month, day, hour, minute, second)
}

// --- item layout helpers ---

/// Arguments for [`compute_item`].
///
/// Bundling them collapses the orientation-specific compute functions into one
/// and removes the `too_many_arguments` clippy escape that the previous
/// horizontal/vertical pair required.
struct ItemLayoutArgs<'a> {
    /// Lane axis position. For horizontal layouts this is the lane center Y
    /// coordinate; for vertical layouts it is the lane center X coordinate.
    lane_axis: f64,
    /// Greedy interval-coloring level for Span/EventRange bars within the lane (#549).
    /// Ignored when `zigzag_offset` is `Some` (#565: the two strategies are
    /// mutually exclusive; Zigzag's signed offset replaces this level-based one).
    bar_stack_level: usize,
    /// #565: signed cross-axis offset (px) from `LayoutStyle::Zigzag`, applied
    /// instead of `bar_stack_level` when present. Positive/negative alternates
    /// by the item's position (even/odd) among same-lane items sorted by start
    /// time; `None` when Zigzag is inactive (not requested, or the lane-count
    /// fallback applied) or Zigzag is active but this item's lane isn't found.
    zigzag_offset: Option<f64>,
    year_min: i64,
    year_max: i64,
    opts: &'a RenderOptions,
    orientation: Orientation,
    color: String,
    tooltip: String,
}

#[derive(Debug, Clone)]
struct BarStackAssignment {
    item_levels: Vec<usize>,
    lane_max_levels: HashMap<String, usize>,
}

fn assign_bar_stack_levels(ir: &TimelineIr) -> BarStackAssignment {
    let mut item_levels = vec![0; ir.items.len()];
    let mut by_lane: HashMap<&str, Vec<(usize, f64, f64)>> = HashMap::new();
    for (idx, item) in ir.items.iter().enumerate() {
        if let Some((start, end)) = bar_interval(item) {
            by_lane
                .entry(item_lane_id(item))
                .or_default()
                .push((idx, start, end));
        }
    }

    let mut lane_max_levels = HashMap::new();
    for (lane_id, mut intervals) in by_lane {
        intervals.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                .then_with(|| a.0.cmp(&b.0))
        });

        let mut level_end: Vec<f64> = Vec::new();
        let mut max_level = 0usize;
        for (idx, start, end) in intervals {
            let mut level = 0usize;
            loop {
                match level_end.get(level) {
                    None => {
                        level_end.push(end);
                        break;
                    }
                    Some(&occupied_end) if start >= occupied_end => {
                        level_end[level] = end;
                        break;
                    }
                    _ => level += 1,
                }
            }
            item_levels[idx] = level;
            max_level = max_level.max(level);
        }
        if max_level > 0 {
            lane_max_levels.insert(lane_id.to_string(), max_level);
        }
    }

    BarStackAssignment {
        item_levels,
        lane_max_levels,
    }
}

fn compute_lane_effective_heights(
    lanes_ordered: &[&Lane],
    assignment: &BarStackAssignment,
    opts: &RenderOptions,
) -> HashMap<String, f64> {
    let step = bar_stack_step(opts);
    lanes_ordered
        .iter()
        .map(|lane| {
            let max_level = assignment
                .lane_max_levels
                .get(lane.id.as_str())
                .copied()
                .unwrap_or(0);
            (lane.id.clone(), opts.lane_height + max_level as f64 * step)
        })
        .collect()
}

/// Maximum lane count supported by `LayoutStyle::Zigzag` (#565). Beyond this,
/// alternating cross-axis offsets from adjacent lanes would visually collide, so
/// rendering returns [`crate::RenderError::UnsupportedLayout`].
pub const ZIGZAG_MAX_LANES: usize = 2;

/// The item's primary (time-axis) start coordinate, in fractional-year units,
/// used to sort items within a lane for `LayoutStyle::Zigzag` (#565) ordering.
/// Unlike [`bar_interval`] (Span/EventRange only), this covers `Item::Event` too
/// since Zigzag alternates *all* item kinds within a lane by start time.
fn item_start_frac(item: &Item) -> f64 {
    match item {
        Item::Span {
            start,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            ..
        }
        | Item::EventRange {
            start,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            ..
        } => start_frac_with_second(
            *start,
            *start_month,
            *start_day,
            *start_hour,
            *start_minute,
            *start_second,
        ),
        Item::Event {
            time,
            time_month,
            time_day,
            time_hour,
            time_minute,
            time_second,
            ..
        } => to_year_frac_with_second(
            *time,
            *time_month,
            *time_day,
            *time_hour,
            *time_minute,
            *time_second,
        ),
    }
}

/// Per-item Zigzag parity assignment (#565): for each lane, sort items by
/// start time and assign `true` (offset one way) to even indices, `false`
/// (offset the other way) to odd indices. Ties (identical start time) break on
/// item index for determinism.
///
/// Returns one bool per item in `ir.items` order (same convention as
/// [`BarStackAssignment::item_levels`]). Always computed (cheap), but only
/// consulted by `compute_item` when `opts.layout_style == LayoutStyle::Zigzag`
/// and the fallback lane-count gate passes.
fn assign_zigzag_parity(ir: &TimelineIr) -> Vec<bool> {
    let mut parity = vec![false; ir.items.len()];
    let mut by_lane: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, item) in ir.items.iter().enumerate() {
        by_lane.entry(item_lane_id(item)).or_default().push(idx);
    }
    for idxs in by_lane.values_mut() {
        idxs.sort_by(|&a, &b| {
            item_start_frac(&ir.items[a])
                .partial_cmp(&item_start_frac(&ir.items[b]))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.cmp(&b))
        });
        for (order, &idx) in idxs.iter().enumerate() {
            parity[idx] = order.is_multiple_of(2);
        }
    }
    parity
}

fn bar_interval(item: &Item) -> Option<(f64, f64)> {
    match item {
        Item::Span {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            ..
        }
        | Item::EventRange {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            ..
        } => {
            let start = start_frac_with_second(
                *start,
                *start_month,
                *start_day,
                *start_hour,
                *start_minute,
                *start_second,
            );
            let end = end_frac_with_second(
                *end,
                *end_month,
                *end_day,
                *end_hour,
                *end_minute,
                *end_second,
            );
            Some((start, end.max(start)))
        }
        Item::Event { .. } => None,
    }
}

/// Cross-axis distance between stacked bar rows (#549).
///
/// The horizontal legacy layout places EventRange bars below the lane center,
/// while Span bars straddle it. A 40px baseline keeps a stacked Span clear of a
/// level-0 EventRange while preserving every level-0 coordinate.
fn bar_stack_step(opts: &RenderOptions) -> f64 {
    let density = (opts.lane_height / DEFAULT_LANE_HEIGHT).max(0.1);
    40.0 * density
}

/// Cross-axis distance (px) between the lane center and an item offset by
/// `LayoutStyle::Zigzag` (#565). Uses the same density scaling as
/// [`bar_stack_step`] so Zigzag spacing grows with `lane_height` consistently
/// with the #549 sub-row spacing it replaces.
fn zigzag_cross_offset(opts: &RenderOptions) -> f64 {
    bar_stack_step(opts)
}

/// Compute the laid-out coordinates for a single item.
///
/// The orientation-specific projection collapses into a single primary/cross
/// axis pair: the time axis is the *primary* axis (X horizontally, Y
/// vertically) and the lane axis is the *cross* axis. The final
/// [`LaidItem`] fields are populated by mapping (primary, cross) back into
/// (x, y) using [`ItemLayoutArgs::orientation`].
///
/// For [`Item::Event`] in vertical orientation, the `LaidItem::Event` fields
/// are reused with shifted semantics: `x` holds the lane axis, and
/// `y_top`/`y_bottom`/`y_dot` hold time-axis values. The SVG emitter detects
/// this via [`LayoutModel::is_vertical`] and renders the stem horizontally.
fn compute_item<'a>(item: &'a Item, items: &mut Vec<LaidItem<'a>>, args: ItemLayoutArgs<'_>) {
    let ItemLayoutArgs {
        lane_axis,
        bar_stack_level,
        zigzag_offset,
        year_min,
        year_max,
        opts,
        orientation,
        color,
        tooltip,
    } = args;
    let is_vertical = orientation == Orientation::Vertical;
    let primary_anchor = if is_vertical {
        opts.top_margin
    } else {
        opts.left_gutter
    };

    // Bar thickness / intra-lane padding follow lane_height (#507): taller lanes
    // get proportionally thicker bars. At the default lane_height (60) the factor
    // is 1.0, so the geometry is byte-for-byte identical to the pre-#507 output.
    let density = (opts.lane_height / DEFAULT_LANE_HEIGHT).max(0.1);
    let span_half_h = SPAN_HALF_H * density;
    let event_range_h = EVENT_RANGE_H * density;
    let event_range_y_offset = EVENT_RANGE_Y_OFFSET * density;
    let event_stem_h = EVENT_STEM_H * density;
    // #565: Zigzag's signed offset replaces the #549 level-based one (mutually
    // exclusive strategies); when Zigzag is inactive this is identical to the
    // pre-#565 `bar_stack_level as f64 * bar_stack_step(opts)` expression.
    let bar_stack_offset =
        zigzag_offset.unwrap_or_else(|| bar_stack_level as f64 * bar_stack_step(opts));

    match item {
        Item::Span {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            ..
        } => {
            // 仕様 §1.4: start は year/月の頭、end は year/月の末日を採用（混在精度補完）
            let sf = start_frac_with_second(
                *start,
                *start_month,
                *start_day,
                *start_hour,
                *start_minute,
                *start_second,
            );
            let ef = end_frac_with_second(
                *end,
                *end_month,
                *end_day,
                *end_hour,
                *end_minute,
                *end_second,
            );
            let (primary_start, primary_extent) =
                primary_axis_segment(sf, ef, year_min, year_max, opts.scale, primary_anchor);
            let cross_start = lane_axis - span_half_h + bar_stack_offset;
            let cross_extent = span_half_h * 2.0;
            let (x, y, width, height) = if is_vertical {
                (cross_start, primary_start, cross_extent, primary_extent)
            } else {
                (primary_start, cross_start, primary_extent, cross_extent)
            };
            let (continues_from_previous_page, continues_to_next_page) = continuation_marker_flags(
                opts.show_boundary_clip_markers,
                sf,
                ef,
                year_min,
                year_max,
            );
            items.push(LaidItem::Span {
                item,
                x,
                y,
                width,
                height,
                color,
                tooltip,
                period_label_stack_level: 0,
                continues_from_previous_page,
                continues_to_next_page,
            });
        }
        Item::EventRange {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            ..
        } => {
            let sf = start_frac_with_second(
                *start,
                *start_month,
                *start_day,
                *start_hour,
                *start_minute,
                *start_second,
            );
            let ef = end_frac_with_second(
                *end,
                *end_month,
                *end_day,
                *end_hour,
                *end_minute,
                *end_second,
            );
            let (primary_start, primary_extent) =
                primary_axis_segment(sf, ef, year_min, year_max, opts.scale, primary_anchor);
            // Horizontal bands sit just below the lane center
            // (EVENT_RANGE_Y_OFFSET); vertical bands are centered on the lane
            // axis. This asymmetry is preserved verbatim from the original
            // split implementation.
            let (x, y, width, height) = if is_vertical {
                (
                    lane_axis - event_range_h / 2.0 + bar_stack_offset,
                    primary_start,
                    event_range_h,
                    primary_extent,
                )
            } else {
                (
                    primary_start,
                    lane_axis + event_range_y_offset + bar_stack_offset,
                    primary_extent,
                    event_range_h,
                )
            };
            let (continues_from_previous_page, continues_to_next_page) = continuation_marker_flags(
                opts.show_boundary_clip_markers,
                sf,
                ef,
                year_min,
                year_max,
            );
            items.push(LaidItem::EventRange {
                item,
                x,
                y,
                width,
                height,
                color,
                tooltip,
                period_label_stack_level: 0,
                continues_from_previous_page,
                continues_to_next_page,
            });
        }
        Item::Event {
            time,
            time_month,
            time_day,
            time_hour,
            time_minute,
            time_second,
            ..
        } => {
            if !year_in_range(*time, year_min, year_max) {
                return;
            }
            let frac = to_year_frac_with_second(
                *time,
                *time_month,
                *time_day,
                *time_hour,
                *time_minute,
                *time_second,
            );
            let primary = primary_anchor + (frac - year_min as f64) * opts.scale;
            // #565: Zigzag shifts the Event's cross-axis (lane) position by
            // `bar_stack_offset` (which carries the signed zigzag offset here;
            // #549 never stacks Events, so this is 0.0 outside Zigzag mode).
            let event_lane_axis = lane_axis + bar_stack_offset;
            let (x, y_top, y_bottom, y_dot) = if is_vertical {
                // x = lane axis; y_top/y_bottom/y_dot all live on the time axis.
                (
                    event_lane_axis,
                    primary - event_stem_h,
                    primary + event_stem_h,
                    primary,
                )
            } else {
                // x = time axis; y_top/y_bottom/y_dot live on the lane axis.
                (
                    primary,
                    event_lane_axis - event_stem_h,
                    event_lane_axis + event_stem_h,
                    event_lane_axis,
                )
            };
            items.push(LaidItem::Event {
                item,
                x,
                y_top,
                y_bottom,
                y_dot,
                color,
                tooltip,
                label_stack_level: 0,
            });
        }
    }
}

// --- sub-layout constants ---
const SPAN_HALF_H: f64 = 12.0;
/// Approximate rendered width (px) of the longest axis label ("BC9999" at 11 px font-size).
const AXIS_LABEL_PX: f64 = 40.0;
const EVENT_RANGE_Y_OFFSET: f64 = 14.0;
const EVENT_RANGE_H: f64 = 10.0;
const EVENT_STEM_H: f64 = 20.0;
const LABEL_HORIZONTAL_PADDING: f64 = 8.0;
const EVENT_LABEL_GAP: f64 = 6.0;
/// Font-size (px) used for always-on Event/EventRange labels (`.tdsl-event-label`
/// CSS class in svg.rs), used to estimate label width for #537 collision detection.
pub(crate) const EVENT_LABEL_FONT_PX: f64 = 10.0;
/// Vertical spacing (px) between stacked label levels when colliding Event labels
/// are pushed apart (#537).
pub(crate) const EVENT_LABEL_STACK_STEP: f64 = 12.0;
/// Minimum horizontal gap (px) required between two labels' estimated bounding
/// boxes for them to be considered non-overlapping (#537).
const EVENT_LABEL_MIN_GAP: f64 = 4.0;

/// Post-processing pass (#537): detect Event labels that would visually overlap
/// within the same lane (based on estimated text width) and assign each a
/// `label_stack_level` so the renderer can offset colliding labels away from the
/// timeline row, stacked in order of increasing level.
///
/// Only `LaidItem::Event` entries participate — Span/EventRange labels live
/// inside their own bar and don't collide with neighbours the same way.
fn assign_event_label_stack_levels(items: &mut [LaidItem<'_>], is_vertical: bool) {
    let mut by_lane: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, laid) in items.iter().enumerate() {
        if let LaidItem::Event { item, .. } = laid {
            by_lane.entry(item_lane_id(item)).or_default().push(idx);
        }
    }

    for mut idxs in by_lane.into_values() {
        // Sort by the primary (time) coordinate so the sweep below only ever
        // needs to compare against the most recently placed interval per level.
        idxs.sort_by(|&a, &b| {
            event_primary_coord(&items[a], is_vertical)
                .partial_cmp(&event_primary_coord(&items[b], is_vertical))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut level_end: Vec<f64> = Vec::new();
        for idx in idxs {
            let (start, end) = event_label_interval(&items[idx], is_vertical);
            let mut level = 0usize;
            loop {
                match level_end.get(level) {
                    None => {
                        level_end.push(end + EVENT_LABEL_MIN_GAP);
                        break;
                    }
                    Some(&occupied_end) if start >= occupied_end => {
                        level_end[level] = end + EVENT_LABEL_MIN_GAP;
                        break;
                    }
                    _ => level += 1,
                }
            }
            if let LaidItem::Event {
                label_stack_level, ..
            } = &mut items[idx]
            {
                *label_stack_level = level.min(u8::MAX as usize) as u8;
            }
        }
    }
}

/// The Event's primary (time-axis) coordinate, matching the field the SVG
/// renderer treats as the time position for label placement: `x` in horizontal
/// orientation, `y_dot` in vertical orientation (see `svg.rs::render_items`).
fn event_primary_coord(laid: &LaidItem<'_>, is_vertical: bool) -> f64 {
    match laid {
        LaidItem::Event { x, y_dot, .. } => {
            if is_vertical {
                *y_dot
            } else {
                *x
            }
        }
        _ => 0.0,
    }
}

/// Estimated `[start, end]` interval (px, along the primary/time axis) that an
/// Event's always-on label occupies at stack level 0, matching the exact
/// placement `svg.rs::render_items` uses: horizontal labels are centered above
/// the dot; vertical labels start just to the right of the dot.
fn event_label_interval(laid: &LaidItem<'_>, is_vertical: bool) -> (f64, f64) {
    let LaidItem::Event { x, y_dot, item, .. } = laid else {
        return (0.0, 0.0);
    };
    let text = item_label(item);
    let width = estimate_text_width_px(text, EVENT_LABEL_FONT_PX);
    if is_vertical {
        let start = *y_dot + 6.0;
        (start, start + width)
    } else {
        let half = width / 2.0;
        (*x - half, *x + half)
    }
}

/// Format the always-on Gantt period label text for a Span/EventRange item
/// (#564): `"<start>\u301c<end>"` using the same date formatting as tooltips and
/// the item table, with month/day/hour/minute precision preserved when present.
pub(crate) fn gantt_period_label(item: &Item) -> String {
    match item {
        Item::Span {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_open,
            ..
        }
        | Item::EventRange {
            start,
            end,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_open,
            ..
        } => {
            let start_str =
                format_date(*start, *start_month, *start_day, *start_hour, *start_minute);
            let end_str = if *end_open {
                "進行中".to_string()
            } else {
                format_date(*end, *end_month, *end_day, *end_hour, *end_minute)
            };
            format!("{start_str}〜{end_str}")
        }
        Item::Event { .. } => String::new(),
    }
}

/// The primary-axis (time-axis) start/extent of a laid-out Span/EventRange bar,
/// in the same coordinate space `svg.rs` uses to draw the `<rect>`: `(x, width)`
/// in horizontal orientation, `(y, height)` in vertical orientation. Returns
/// `None` for `LaidItem::Event`, which has no bar.
fn bar_primary_axis(laid: &LaidItem<'_>, is_vertical: bool) -> Option<(f64, f64)> {
    match laid {
        LaidItem::Span {
            x,
            y,
            width,
            height,
            ..
        }
        | LaidItem::EventRange {
            x,
            y,
            width,
            height,
            ..
        } => Some(if is_vertical {
            (*y, *height)
        } else {
            (*x, *width)
        }),
        LaidItem::Event { .. } => None,
    }
}

/// Estimated `[start, end]` interval (px, along the primary/time axis) that a
/// Span/EventRange's always-on Gantt period label occupies at stack level 0,
/// matching the placement `svg.rs::render_items` uses for Gantt period labels:
/// left-aligned starting just after the bar's own primary-axis start.
fn period_label_interval(laid: &LaidItem<'_>, is_vertical: bool) -> (f64, f64) {
    let Some((bar_start, _bar_extent)) = bar_primary_axis(laid, is_vertical) else {
        return (0.0, 0.0);
    };
    let text = match laid {
        LaidItem::Span { item, .. } | LaidItem::EventRange { item, .. } => gantt_period_label(item),
        LaidItem::Event { .. } => return (0.0, 0.0),
    };
    let width = estimate_text_width_px(&text, EVENT_LABEL_FONT_PX);
    (bar_start, bar_start + width)
}

/// Post-processing pass (#564): detect Gantt period labels (Span/EventRange
/// start〜end text) that would visually overlap within the same lane *and*
/// the same #549 bar sub-row (bars whose time ranges don't overlap can still
/// have colliding period-label text if they sit close together), and assign
/// each a `period_label_stack_level` so the renderer can offset colliding
/// labels away from the bar, stacked in order of increasing level.
///
/// Grouping by `(lane, bar_stack_level)` keeps this independent of the #549
/// sub-row Y placement: each sub-row gets its own independent label-collision
/// sweep, so a label offset never crosses into a neighbouring sub-row's space.
/// Only called when `RenderOptions.layout_style == LayoutStyle::Gantt`.
fn assign_period_label_stack_levels(items: &mut [LaidItem<'_>], is_vertical: bool) {
    // Group by (lane, cross-axis bucket): bars placed in the same #549 sub-row
    // share the exact same cross-axis pixel offset (computed once during
    // `compute_item`), so bucketing on that value groups bars per sub-row
    // without needing to re-derive `bar_stack_level` here.
    let mut by_lane_and_row: HashMap<(&str, u64), Vec<usize>> = HashMap::new();
    for (idx, laid) in items.iter().enumerate() {
        let lane = match laid {
            LaidItem::Span { item, .. } | LaidItem::EventRange { item, .. } => item_lane_id(item),
            LaidItem::Event { .. } => continue,
        };
        let cross = bar_cross_axis_bucket(laid, is_vertical);
        by_lane_and_row.entry((lane, cross)).or_default().push(idx);
    }

    for mut idxs in by_lane_and_row.into_values() {
        idxs.sort_by(|&a, &b| {
            let (a_start, _) = period_label_interval(&items[a], is_vertical);
            let (b_start, _) = period_label_interval(&items[b], is_vertical);
            a_start
                .partial_cmp(&b_start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut level_end: Vec<f64> = Vec::new();
        for idx in idxs {
            let (start, end) = period_label_interval(&items[idx], is_vertical);
            let mut level = 0usize;
            loop {
                match level_end.get(level) {
                    None => {
                        level_end.push(end + EVENT_LABEL_MIN_GAP);
                        break;
                    }
                    Some(&occupied_end) if start >= occupied_end => {
                        level_end[level] = end + EVENT_LABEL_MIN_GAP;
                        break;
                    }
                    _ => level += 1,
                }
            }
            match &mut items[idx] {
                LaidItem::Span {
                    period_label_stack_level,
                    ..
                }
                | LaidItem::EventRange {
                    period_label_stack_level,
                    ..
                } => {
                    *period_label_stack_level = level.min(u8::MAX as usize) as u8;
                }
                LaidItem::Event { .. } => {}
            }
        }
    }
}

/// Bucket a bar's cross-axis (lane-perpendicular) coordinate to an integer key
/// so bars placed in the same #549 sub-row (identical cross-axis pixel offset)
/// group together for period-label collision detection, while bars in
/// different sub-rows (different `bar_stack_level`) never interfere with each
/// other's label placement.
fn bar_cross_axis_bucket(laid: &LaidItem<'_>, is_vertical: bool) -> u64 {
    let cross = match laid {
        LaidItem::Span { x, y, .. } | LaidItem::EventRange { x, y, .. } => {
            if is_vertical {
                *x
            } else {
                *y
            }
        }
        LaidItem::Event { .. } => 0.0,
    };
    cross.round() as u64
}

/// Estimate rendered text width in CSS pixels for timeline labels.
///
/// This deliberately uses a small, deterministic approximation table instead of
/// font-specific glyph metrics. `RenderOptions::font_family` is ignored: the
/// result is intended for layout heuristics (overflow/collision detection), not
/// exact typography.
pub(crate) fn estimate_text_width_px(text: &str, font_size_px: f64) -> f64 {
    text.chars()
        .map(|ch| char_width_em(ch) * font_size_px)
        .sum()
}

fn char_width_em(ch: char) -> f64 {
    if ch.is_whitespace() {
        0.33
    } else if ch.is_ascii_digit() {
        0.56
    } else if ch.is_ascii_alphabetic() {
        match ch {
            'i' | 'j' | 'l' | 'I' => 0.32,
            'm' | 'w' | 'M' | 'W' => 0.86,
            _ => 0.62,
        }
    } else if ch.is_ascii_punctuation() {
        0.38
    } else if is_cjk_like(ch) {
        1.0
    } else {
        0.75
    }
}

fn is_cjk_like(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff // Hangul Jamo
            | 0x2e80..=0x9fff // CJK radicals, kana, ideographs
            | 0xac00..=0xd7af // Hangul syllables
            | 0xf900..=0xfaff // CJK compatibility ideographs
            | 0xff00..=0xffef // Fullwidth forms
            | 0x20000..=0x2fa1f // CJK extensions
    )
}

/// Whether a laid-out item's label exceeds the inline space available for it.
///
/// For bars (Span/EventRange), available space is the bar's primary-axis extent
/// minus horizontal padding. Point events do not have a bar; their available
/// inline space is the remaining primary-axis space after the label gap.
#[cfg(test)]
pub(crate) fn label_overflows_item(
    item: &LaidItem<'_>,
    opts: &RenderOptions,
    font_size_px: f64,
    total_width: f64,
    total_height: f64,
) -> bool {
    let text_width = estimate_text_width_px(laid_item_label(item), font_size_px);
    text_width > label_available_width_px(item, opts, total_width, total_height)
}

pub(crate) fn label_available_width_px(
    item: &LaidItem<'_>,
    opts: &RenderOptions,
    total_width: f64,
    total_height: f64,
) -> f64 {
    let is_vertical = opts.orientation == Orientation::Vertical;
    match item {
        LaidItem::Span { width, height, .. } | LaidItem::EventRange { width, height, .. } => {
            let primary_extent = if is_vertical { *height } else { *width };
            (primary_extent - LABEL_HORIZONTAL_PADDING * 2.0).max(0.0)
        }
        LaidItem::Event { x, y_dot, .. } => {
            if is_vertical {
                (total_height - *y_dot - opts.bottom_margin - EVENT_LABEL_GAP).max(0.0)
            } else {
                (total_width - *x - opts.right_margin - EVENT_LABEL_GAP).max(0.0)
            }
        }
    }
}

pub(crate) fn laid_item_label<'a>(item: &'a LaidItem<'_>) -> &'a str {
    match item {
        LaidItem::Span { item, .. }
        | LaidItem::EventRange { item, .. }
        | LaidItem::Event { item, .. } => item_label(item),
    }
}

fn item_label(item: &Item) -> &str {
    match item {
        Item::Span { label, .. } | Item::Event { label, .. } | Item::EventRange { label, .. } => {
            label
        }
    }
}

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

/// エラーメッセージ用にアイテムを人間が識別できる形で表す。
/// id が空のときは label にフォールバックする（IR JSON を直接渡す経路では
/// id が省略されうるため、どちらか一方だけでは特定できないことがある）。
fn item_display_name(item: &Item) -> String {
    let (id, label) = match item {
        Item::Span { id, label, .. }
        | Item::Event { id, label, .. }
        | Item::EventRange { id, label, .. } => (id, label),
    };
    if id.is_empty() {
        label.clone()
    } else {
        id.clone()
    }
}

fn get_item_tags(item: &Item) -> &[String] {
    match item {
        Item::Span { tags, .. } | Item::Event { tags, .. } | Item::EventRange { tags, .. } => tags,
    }
}

fn item_color(item: &Item) -> &Option<String> {
    match item {
        Item::Span { color, .. } | Item::Event { color, .. } | Item::EventRange { color, .. } => {
            color
        }
    }
}

fn is_safe_color_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Resolve item fill color: item color overrides tag color_map, which overrides lane palette.
pub(crate) fn resolve_item_color(
    item_color: &Option<String>,
    tags: &[String],
    color_map: &HashMap<String, String>,
    lane_id: &str,
    lane_colors: &HashMap<String, String>,
) -> String {
    if let Some(color) = item_color {
        let color = color.trim();
        if is_safe_color_value(color) {
            return color.to_string();
        }
    }
    for tag in tags {
        if let Some(color) = color_map.get(tag.as_str()) {
            let color = color.trim();
            if is_safe_color_value(color) {
                return color.to_string();
            }
        }
    }
    lane_colors
        .get(lane_id)
        .cloned()
        .unwrap_or_else(|| "#4682B4".to_string())
}

/// Format a year for display: negative years get a "BC" prefix.
pub(crate) fn format_year(year: i64) -> String {
    if year < 0 {
        format!("BC{}", -year)
    } else {
        format!("{year}")
    }
}

/// Short three-letter English month abbreviation.
pub(crate) fn month_abbr(m: u8) -> &'static str {
    match m {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "?",
    }
}

/// Format a date for display, with optional month/day/hour/minute precision.
pub(crate) fn format_date(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
) -> String {
    let y = format_year(year);
    match (month, day, hour, minute) {
        (Some(m), Some(d), Some(h), Some(min)) => {
            format!("{} {} {} {:02}:{:02}", y, month_abbr(m), d, h, min)
        }
        (Some(m), Some(d), _, _) => format!("{} {} {}", y, month_abbr(m), d),
        (Some(m), None, _, _) => format!("{} {}", y, month_abbr(m)),
        _ => y,
    }
}

/// #556: whether `timeline.range` start and end fall on the same calendar day.
/// Used to pick a context-dependent axis label format for hour/minute ticks:
/// `HH:MM` when the whole timeline is a single day, `MM-DD HH:00` otherwise.
pub(crate) fn is_single_day_range(meta: &tdsl_core::ir::Meta) -> bool {
    let (y0, y1) = meta.range;
    y0 == y1
        && meta.range_start_month == meta.range_end_month
        && meta.range_start_day == meta.range_end_day
        && meta.range_start_month.is_some()
        && meta.range_start_day.is_some()
}

/// Format an hour-tick axis label (#556): `HH:00` for a single-day timeline,
/// `MM-DD HH:00` when the timeline spans multiple days.
pub(crate) fn format_hour_tick_label(month: u8, day: u8, hour: u8, single_day: bool) -> String {
    if single_day {
        format!("{hour:02}:00")
    } else {
        format!("{month:02}-{day:02} {hour:02}:00")
    }
}

/// Format a minute-tick axis label (#556): `HH:MM` for a single-day timeline,
/// `MM-DD HH:MM` when the timeline spans multiple days.
pub(crate) fn format_minute_tick_label(
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    single_day: bool,
) -> String {
    if single_day {
        format!("{hour:02}:{minute:02}")
    } else {
        format!("{month:02}-{day:02} {hour:02}:{minute:02}")
    }
}

/// Format a second-tick axis label (#614, ADR 0003): `HH:MM:SS` for a
/// single-day timeline, `MM-DD HH:MM:SS` when the timeline spans multiple days.
pub(crate) fn format_second_tick_label(
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    single_day: bool,
) -> String {
    if single_day {
        format!("{hour:02}:{minute:02}:{second:02}")
    } else {
        format!("{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
    }
}

struct ItemCommon<'a> {
    tags: &'a [String],
    source: &'a Option<String>,
    origin: &'a Option<String>,
    note: &'a Option<String>,
    link: &'a Option<String>,
    color: &'a Option<String>,
    id: &'a str,
}

fn push_common(lines: &mut Vec<String>, common: ItemCommon<'_>) {
    if !common.tags.is_empty() {
        lines.push(format!("tags: {}", common.tags.join(", ")));
    }
    if let Some(src) = common.source {
        lines.push(format!("source: {src}"));
    }
    if let Some(org) = common.origin {
        lines.push(format!("origin: {org}"));
    }
    if let Some(note) = common.note {
        lines.push(format!("note: {note}"));
    }
    if let Some(link) = common.link {
        lines.push(format!("link: {link}"));
    }
    if let Some(color) = common.color {
        lines.push(format!("color: {color}"));
    }
    lines.push(format!("id: {}", common.id));
}

/// Build the tooltip text for an item (XML-unescaped).
/// Compute background bands (#543) spanning contiguous lanes that share the
/// same `group` value. Lanes are already ordered by `(order, id)`; a "contiguous
/// run" of lanes with the same `Some(group)` value becomes one band. Ungrouped
/// lanes (`group == None`) never produce a band. Bands are index-ordered by
/// first occurrence, alternating `even`/`odd` for styling (independent of the
/// underlying lane band parity).
fn compute_group_bands(
    lanes_ordered: &[&Lane],
    lane_start: &HashMap<String, f64>,
    lane_effective_heights: &HashMap<String, f64>,
    opts: &RenderOptions,
    body_height: f64,
    total_width: f64,
) -> Vec<GroupBandModel> {
    if opts.layout_style != LayoutStyle::GroupBands {
        return Vec::new();
    }
    let is_vertical = opts.orientation == Orientation::Vertical;
    let mut bands: Vec<GroupBandModel> = Vec::new();
    let mut idx = 0usize;
    let mut band_idx = 0usize;
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
            let start_lane_id = &lanes_ordered[start_idx].id;
            let end_lane_id = &lanes_ordered[end_idx].id;
            let start = lane_start[start_lane_id];
            let end = lane_start[end_lane_id]
                + lane_effective_heights
                    .get(end_lane_id.as_str())
                    .copied()
                    .unwrap_or(opts.lane_height);
            if is_vertical {
                bands.push(GroupBandModel {
                    label: group_label.to_string(),
                    x: start,
                    y: opts.top_margin,
                    width: end - start,
                    height: body_height - opts.top_margin - opts.bottom_margin,
                    even: band_idx.is_multiple_of(2),
                });
            } else {
                bands.push(GroupBandModel {
                    label: group_label.to_string(),
                    x: opts.left_gutter,
                    y: start,
                    width: total_width - opts.left_gutter - opts.right_margin,
                    height: end - start,
                    even: band_idx.is_multiple_of(2),
                });
            }
            band_idx += 1;
        }
        idx = end_idx + 1;
    }
    bands
}

fn item_tooltip(item: &Item) -> String {
    let mut lines = Vec::new();
    match item {
        Item::Span {
            label,
            start,
            end,
            tags,
            source,
            origin,
            note,
            link,
            color,
            id,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_open,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                open_ended_end_label(
                    *end,
                    *end_month,
                    *end_day,
                    *end_hour,
                    *end_minute,
                    *end_open
                ),
            ));
            push_common(
                &mut lines,
                ItemCommon {
                    tags,
                    source,
                    origin,
                    note,
                    link,
                    color,
                    id,
                },
            );
        }
        Item::Event {
            label,
            time,
            tags,
            source,
            origin,
            note,
            link,
            color,
            id,
            time_month,
            time_day,
            time_hour,
            time_minute,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format_date(
                *time,
                *time_month,
                *time_day,
                *time_hour,
                *time_minute,
            ));
            push_common(
                &mut lines,
                ItemCommon {
                    tags,
                    source,
                    origin,
                    note,
                    link,
                    color,
                    id,
                },
            );
        }
        Item::EventRange {
            label,
            start,
            end,
            tags,
            source,
            origin,
            note,
            link,
            color,
            id,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_open,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                open_ended_end_label(
                    *end,
                    *end_month,
                    *end_day,
                    *end_hour,
                    *end_minute,
                    *end_open
                ),
            ));
            push_common(
                &mut lines,
                ItemCommon {
                    tags,
                    source,
                    origin,
                    note,
                    link,
                    color,
                    id,
                },
            );
        }
    }
    lines.join("\n")
}

/// #550: render `end` as "進行中" (ongoing) in the tooltip when the item is
/// open-ended (`end_open`), instead of the resolved placeholder year.
#[allow(clippy::too_many_arguments)]
fn open_ended_end_label(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    end_open: bool,
) -> String {
    if end_open {
        "進行中".to_string()
    } else {
        format_date(year, month, day, hour, minute)
    }
}

fn year_to_x(year: i64, year_min: i64, scale: f64, left_gutter: f64) -> f64 {
    left_gutter + (year - year_min) as f64 * scale
}

/// Convert year + optional month + optional day to a fractional year value.
fn to_year_frac(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
) -> f64 {
    to_year_frac_with_second(year, month, day, hour, minute, None)
}

/// Like [`to_year_frac`] but also folds in second precision (#614, ADR 0003).
/// `second` is only meaningful once `minute` is `Some`.
fn to_year_frac_with_second(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
) -> f64 {
    let mut frac = year as f64;
    if let Some(m) = month {
        frac += (m.clamp(1, 12) - 1) as f64 / 12.0;
        if let Some(d) = day {
            frac += (d.clamp(1, 31) - 1) as f64 / 365.25;
            if let Some(h) = hour {
                frac += h.min(23) as f64 / 24.0 / 365.25;
                if let Some(min) = minute {
                    frac += min.min(59) as f64 / 1440.0 / 365.25;
                    if let Some(s) = second {
                        frac += s.min(59) as f64 / 86400.0 / 365.25;
                    }
                }
            }
        }
    }
    frac
}

/// Fractional-year position including hour/minute/second precision (#556,
/// #614, ADR 0003). `second` is only meaningful once `minute` is `Some`; a
/// bare `second` without `minute` cannot occur in practice (the AST/IR always
/// set hour+minute together with second, see `TimeValue::DateTimeSecond`).
fn start_frac_with_second(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
) -> f64 {
    start_frac(year, month, day)
        + hour.unwrap_or(0).min(23) as f64 / 24.0 / 365.25
        + minute.unwrap_or(0).min(59) as f64 / 1440.0 / 365.25
        + second.unwrap_or(0).min(59) as f64 / 86400.0 / 365.25
}

/// Like [`start_frac_with_second`] but for the end endpoint of a Span/EventRange
/// (#556, #614).
fn end_frac_with_second(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
) -> f64 {
    match hour {
        Some(_) => start_frac_with_second(year, month, day, hour, minute, second),
        None => end_frac(year, month, day),
    }
}

fn frac_to_x(frac: f64, year_min: i64, scale: f64, left_gutter: f64) -> f64 {
    left_gutter + (frac - year_min as f64) * scale
}

fn year_in_range(year: i64, year_min: i64, year_max: i64) -> bool {
    year >= year_min && year <= year_max
}

/// Compute the (start, extent) of a span/event-range projected onto the time
/// (primary) axis.
///
/// `anchor` is the pixel coordinate where `year_min` falls on the primary
/// axis: `left_gutter` for horizontal layouts, `top_margin` for vertical
/// layouts. The same formula serves both orientations.
fn primary_axis_segment(
    start_frac: f64,
    end_frac: f64,
    year_min: i64,
    year_max: i64,
    scale: f64,
    anchor: f64,
) -> (f64, f64) {
    let s = start_frac.max(year_min as f64);
    let e = end_frac.min(year_max as f64);
    if e < s {
        return (anchor + (start_frac - year_min as f64) * scale, 0.0);
    }
    (anchor + (s - year_min as f64) * scale, (e - s) * scale)
}

/// `(continues_from_previous_page, continues_to_next_page)` for a
/// `Span`/`EventRange` item's fractional `[start_frac, end_frac]` extent
/// against `[year_min, year_max]` (issue #734). Both are always `false` when
/// `enabled` is `false`, or when the item is wholly outside
/// `[year_min, year_max]` — an item entirely past the page's edge is a
/// zero/negative-width `primary_axis_segment` degenerate bar (see
/// `items_wholly_outside_a_page_segment_still_produce_a_laid_item_with_non_positive_extent`
/// in `time_range_pagination.rs`), not a bar actually shown on this page, so
/// it must never get a marker either.
fn continuation_marker_flags(
    enabled: bool,
    start_frac: f64,
    end_frac: f64,
    year_min: i64,
    year_max: i64,
) -> (bool, bool) {
    if !enabled {
        return (false, false);
    }
    let on_this_page = end_frac > year_min as f64 && start_frac < year_max as f64;
    if !on_this_page {
        return (false, false);
    }
    (start_frac < year_min as f64, end_frac > year_max as f64)
}

fn derive_range_from_items(ir: &TimelineIr) -> Option<(i64, i64)> {
    let mut min: Option<i64> = None;
    let mut max: Option<i64> = None;
    for item in &ir.items {
        match item {
            Item::Span { start, end, .. } | Item::EventRange { start, end, .. } => {
                min = Some(min.map_or(*start, |m| m.min(*start)));
                max = Some(max.map_or(*end, |m| m.max(*end)));
            }
            Item::Event { time, .. } => {
                min = Some(min.map_or(*time, |m| m.min(*time)));
                max = Some(max.map_or(*time, |m| m.max(*time)));
            }
        }
    }
    match (min, max) {
        (Some(a), Some(b)) if b > a => Some((a, b)),
        (Some(a), Some(b)) => Some((a - 10, b + 10)),
        _ => None,
    }
}

/// Pick a tick step so that labels do not visually overlap.
/// `step * scale` must be at least `label_px + 8` px (minimum inter-label gap).
fn pick_tick_step(range: i64, scale: f64, label_px: f64) -> i64 {
    if range <= 0 {
        return 1;
    }
    let min_pitch = label_px + 8.0;
    const CANDIDATES: &[i64] = &[
        1, 2, 5, 10, 20, 25, 50, 100, 200, 250, 500, 1000, 2000, 5000,
    ];
    for &step in CANDIDATES {
        if (step as f64) * scale >= min_pitch {
            return step;
        }
    }
    10000
}

fn div_floor(a: i64, b: i64) -> i64 {
    let q = a / b;
    let r = a % b;
    if (r != 0) && ((r < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_meta(range: (i64, i64)) -> tdsl_core::ir::Meta {
        tdsl_core::ir::Meta {
            title: "t".into(),
            unit: "year".into(),
            range,
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn year_to_x_basic() {
        let ir = TimelineIr {
            meta: mk_meta((-500, 2000)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        // With scale=2.0 and left_gutter=120, year -500 → x=120, year 0 → x=120+500*2=1120
        assert_eq!(layout.year_to_x(-500), 120.0);
        assert_eq!(layout.year_to_x(0), 1120.0);
        assert_eq!(layout.year_to_x(2000), 120.0 + 2500.0 * 2.0);
    }

    fn mk_lane(id: &str) -> tdsl_core::ir::Lane {
        tdsl_core::ir::Lane {
            id: id.into(),
            label: id.into(),
            kind: "custom".into(),
            order: 0,
            group: None,
            source_span: None,
        }
    }

    fn mk_event(id: &str, lane: &str, year: i64) -> Item {
        Item::Event {
            id: id.into(),
            lane: lane.into(),
            time: year,
            label: id.into(),
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

    /// #765: 未知 lane を参照する item は黙って読み飛ばさず、明示エラーにする。
    /// `.tdsl` 経由なら lowering が弾くが、IR JSON を直接受ける経路
    /// (WASM / 外部ツール生成 IR) には lowering が挟まらない。
    #[test]
    fn unknown_lane_item_is_an_error_not_a_silent_drop() {
        let ir = TimelineIr {
            meta: mk_meta((1900, 2000)),
            lanes: vec![mk_lane("known")],
            items: vec![mk_event("e1", "nosuchlane", 1950)],
            imports: vec![],
            sources: vec![],
        };

        let err = LayoutModel::compute(&ir, RenderOptions::default())
            .map(|_| ())
            .expect_err("unknown lane must fail instead of dropping the item");
        match err {
            RenderError::UnknownLane { lane, item } => {
                assert_eq!(lane, "nosuchlane");
                assert_eq!(item, "e1");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    /// 既知 lane だけなら従来どおり成功する（上のテストが常に失敗していないことの確認）。
    #[test]
    fn known_lane_item_still_lays_out() {
        let ir = TimelineIr {
            meta: mk_meta((1900, 2000)),
            lanes: vec![mk_lane("known")],
            items: vec![mk_event("e1", "known", 1950)],
            imports: vec![],
            sources: vec![],
        };

        let layout =
            LayoutModel::compute(&ir, RenderOptions::default()).expect("known lane must lay out");
        assert_eq!(layout.items.len(), 1);
    }

    /// #765: range が degenerate で items からも導出できない場合、
    /// 以前は (0, 2000) という魔法の既定値に握りつぶしていた。
    #[test]
    fn degenerate_range_without_items_is_an_error() {
        let ir = TimelineIr {
            meta: mk_meta((2000, 1900)),
            lanes: vec![mk_lane("known")],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };

        let err = LayoutModel::compute(&ir, RenderOptions::default())
            .map(|_| ())
            .expect_err("degenerate range without items must fail");
        assert!(
            matches!(err, RenderError::DegenerateRange { .. }),
            "unexpected error: {err:?}"
        );
    }

    /// degenerate でも items から導出できるなら従来どおり成功する。
    #[test]
    fn degenerate_range_with_items_derives_from_items() {
        let ir = TimelineIr {
            meta: mk_meta((2000, 1900)),
            lanes: vec![mk_lane("known")],
            items: vec![mk_event("e1", "known", 1950)],
            imports: vec![],
            sources: vec![],
        };

        LayoutModel::compute(&ir, RenderOptions::default())
            .expect("range derivable from items must succeed");
    }

    #[test]
    fn tick_step_no_overlap_for_various_scales() {
        // scale=2.0, label_px=40.0 → min_pitch=48 → step=25 (25*2=50 ≥ 48)
        assert_eq!(pick_tick_step(80, 2.0, 40.0), 25);
        // range=79 previously jumped to step=5 (10px pitch) which caused overlap; now stays 25
        assert_eq!(pick_tick_step(79, 2.0, 40.0), 25);
        assert_eq!(pick_tick_step(20, 2.0, 40.0), 25);
        assert_eq!(pick_tick_step(10, 2.0, 40.0), 25);
        // scale=4.0 → step=20 (20*4=80 ≥ 48)
        assert_eq!(pick_tick_step(80, 4.0, 40.0), 20);
        // scale=1.0 → step=50 (50*1=50 ≥ 48)
        assert_eq!(pick_tick_step(100, 1.0, 40.0), 50);
        // scale=0.5 → step=100 (100*0.5=50 ≥ 48)
        assert_eq!(pick_tick_step(2500, 0.5, 40.0), 100);
    }

    #[test]
    fn tick_step_no_overlap_invariant() {
        // Core invariant: step * scale >= label_px + min_gap for all representative combinations.
        let label_px = 40.0_f64;
        let min_gap = 8.0_f64;
        for range in [10_i64, 20, 79, 80] {
            for scale in [0.5_f64, 1.0, 2.0, 4.0] {
                let step = pick_tick_step(range, scale, label_px);
                let pitch = (step as f64) * scale;
                assert!(
                    pitch >= label_px + min_gap,
                    "range={range}, scale={scale}: step={step}, pitch={pitch:.1} < min_pitch={min_pitch}",
                    min_pitch = label_px + min_gap,
                );
            }
        }
    }

    #[test]
    fn div_floor_handles_negative() {
        assert_eq!(div_floor(-500, 100), -5);
        assert_eq!(div_floor(-501, 100), -6);
        assert_eq!(div_floor(501, 100), 5);
    }

    // ─── unit day レンダリング (#248) ─────────────────────────────────

    fn mk_meta_with_unit(unit: &str, range: (i64, i64)) -> tdsl_core::ir::Meta {
        tdsl_core::ir::Meta {
            title: "t".into(),
            unit: unit.into(),
            range,
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn day_ticks_empty_when_unit_not_day() {
        let ir = TimelineIr {
            meta: mk_meta_with_unit("year", (1939, 1945)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.day_ticks().is_empty());
    }

    #[test]
    fn day_ticks_empty_when_unit_month() {
        let ir = TimelineIr {
            meta: mk_meta_with_unit("month", (1939, 1945)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.day_ticks().is_empty());
    }

    #[test]
    fn day_ticks_produced_for_short_unit_day_range() {
        // 1ヶ月分（30日）を大きめスケールで描画 → 1日 step
        let ir = TimelineIr {
            meta: mk_meta_with_unit("day", (1939, 1940)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 6.0, // pixels_per_day = 6 → step=1
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.day_ticks();
        // 1939年内+1940年の日々
        assert!(!ticks.is_empty(), "expected day ticks but got none");
        // 1939-01-01 が含まれる
        assert!(ticks.contains(&(1939, 1, 1)));
        // 1939-12-31 が含まれる
        assert!(ticks.contains(&(1939, 12, 31)));
    }

    #[test]
    fn day_ticks_step_thins_for_lower_density() {
        // 中スケール → 1日あたり 3px (step=2): 月初+奇数日が描画される
        let ir = TimelineIr {
            meta: mk_meta_with_unit("day", (1939, 1940)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 3.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.day_ticks();
        // 月初は常に含まれる
        assert!(ticks.contains(&(1939, 1, 1)));
        assert!(ticks.contains(&(1939, 2, 1)));
        // step=2 のとき、1, 3, 5, ... のみが描画される
        assert!(ticks.contains(&(1939, 1, 3)));
        assert!(!ticks.contains(&(1939, 1, 2)));
    }

    #[test]
    fn day_ticks_thinning_to_weekly_for_low_density() {
        // pixels_per_day ≈ 1.5 → step=7
        let ir = TimelineIr {
            meta: mk_meta_with_unit("day", (1939, 1940)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 2.0, // pixels_per_day=2 → step=7
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.day_ticks();
        // 月初は描画
        assert!(ticks.contains(&(1939, 1, 1)));
        // 1, 8, 15, 22, 29 が含まれる（step=7）
        assert!(ticks.contains(&(1939, 1, 8)));
        // 2, 3, 4 は含まれない
        assert!(!ticks.contains(&(1939, 1, 2)));
        assert!(!ticks.contains(&(1939, 1, 4)));
    }

    #[test]
    fn day_ticks_empty_when_scale_too_small() {
        let ir = TimelineIr {
            meta: mk_meta_with_unit("day", (1900, 2000)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 2.0, // pixels_per_day ≈ 0.0055 → 描画不可
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        assert!(layout.day_ticks().is_empty());
    }

    // ─── unit hour / minute レンダリング (#556) ─────────────────────────────

    fn mk_subday_meta(unit: &str) -> tdsl_core::ir::Meta {
        tdsl_core::ir::Meta {
            title: "t".into(),
            unit: unit.into(),
            range: (1969, 1969),
            range_start_month: Some(1),
            range_start_day: Some(1),
            range_start_hour: Some(0),
            range_start_minute: Some(0),
            range_end_month: Some(1),
            range_end_day: Some(1),
            range_end_hour: Some(23),
            range_end_minute: Some(59),
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        }
    }

    #[test]
    fn hour_ticks_empty_when_unit_not_hour() {
        let ir = TimelineIr {
            meta: mk_subday_meta("day"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.hour_ticks().is_empty());
    }

    #[test]
    fn hour_ticks_produced_for_unit_hour_high_density() {
        // pixels_per_hour = scale / (365.25*24) >= 6 → step=1h
        let ir = TimelineIr {
            meta: mk_subday_meta("hour"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 6.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.hour_ticks();
        assert!(!ticks.is_empty(), "expected hour ticks but got none");
        assert!(ticks.contains(&(1969, 1, 1, 0)));
        assert!(ticks.contains(&(1969, 1, 1, 1)));
    }

    #[test]
    fn hour_ticks_thin_to_3h_for_medium_density() {
        // pixels_per_hour = 2 → step=3h
        let ir = TimelineIr {
            meta: mk_subday_meta("hour"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 2.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.hour_ticks();
        assert!(ticks.contains(&(1969, 1, 1, 0)));
        assert!(ticks.contains(&(1969, 1, 1, 3)));
        assert!(!ticks.contains(&(1969, 1, 1, 1)));
        assert!(!ticks.contains(&(1969, 1, 1, 2)));
    }

    #[test]
    fn hour_ticks_empty_when_scale_too_small() {
        let ir = TimelineIr {
            meta: mk_subday_meta("hour"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 2.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        assert!(layout.hour_ticks().is_empty());
    }

    #[test]
    fn minute_ticks_empty_when_unit_not_minute() {
        let ir = TimelineIr {
            meta: mk_subday_meta("hour"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.minute_ticks().is_empty());
    }

    #[test]
    fn minute_ticks_produced_for_unit_minute_high_density() {
        // pixels_per_minute = scale / (365.25*24*60) >= 6 → step=1min
        let ir = TimelineIr {
            meta: mk_subday_meta("minute"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 60.0 * 6.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.minute_ticks();
        assert!(!ticks.is_empty(), "expected minute ticks but got none");
        assert!(ticks.contains(&(1969, 1, 1, 0, 0)));
        assert!(ticks.contains(&(1969, 1, 1, 0, 1)));
    }

    #[test]
    fn minute_ticks_empty_when_scale_too_small() {
        let ir = TimelineIr {
            meta: mk_subday_meta("minute"),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 2.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        assert!(layout.minute_ticks().is_empty());
    }

    #[test]
    fn is_single_day_range_detects_same_day() {
        let meta = tdsl_core::ir::Meta {
            title: "t".into(),
            unit: "hour".into(),
            range: (1969, 1969),
            range_start_month: Some(7),
            range_start_day: Some(20),
            range_end_month: Some(7),
            range_end_day: Some(20),
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(is_single_day_range(&meta));
    }

    #[test]
    fn is_single_day_range_false_for_multi_day() {
        let meta = tdsl_core::ir::Meta {
            title: "t".into(),
            unit: "hour".into(),
            range: (1969, 1969),
            range_start_month: Some(7),
            range_start_day: Some(20),
            range_end_month: Some(7),
            range_end_day: Some(21),
            calendar: "proleptic_gregorian".into(),
            color_map: std::collections::HashMap::new(),
            ..Default::default()
        };
        assert!(!is_single_day_range(&meta));
    }

    #[test]
    fn format_hour_tick_label_single_vs_multi_day() {
        assert_eq!(format_hour_tick_label(7, 20, 14, true), "14:00");
        assert_eq!(format_hour_tick_label(7, 20, 14, false), "07-20 14:00");
    }

    #[test]
    fn format_minute_tick_label_single_vs_multi_day() {
        assert_eq!(format_minute_tick_label(7, 20, 20, 17, true), "20:17");
        assert_eq!(
            format_minute_tick_label(7, 20, 20, 17, false),
            "07-20 20:17"
        );
    }

    #[test]
    fn span_uses_start_frac_end_frac_for_year_precision() {
        // `span x 1939..1945` は start=1939-01-01, end=1945-12-31 として描画されるべき
        let ir = TimelineIr {
            meta: mk_meta_with_unit("year", (1900, 2000)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Span {
                id: "s1".into(),
                lane: "x".into(),
                start: 1939,
                end: 1945,
                label: "WW2".into(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let span = layout
            .items
            .iter()
            .find_map(|i| match i {
                LaidItem::Span { x, width, .. } => Some((*x, *width)),
                _ => None,
            })
            .expect("span should be laid out");
        // start_frac(1939)=1939.0, end_frac(1945)≈1945.998
        // x = left_gutter(120) + (1939-1900)*scale(2) = 120 + 78 = 198
        // width = (end_frac - start_frac) * scale ≈ 6.998 * 2 ≈ 13.996
        assert!(
            (span.0 - 198.0).abs() < 0.01,
            "expected x ≈ 198, got {}",
            span.0
        );
        // 旧実装 (to_year_frac) なら width = (1945 - 1939) * 2 = 12.0、
        // 新実装 (end_frac) なら ≈ 13.996。明確に差が出る。
        assert!(
            span.1 > 13.0,
            "expected width > 13 (end-of-year extension), got {}",
            span.1
        );
    }

    #[test]
    fn estimate_text_width_handles_ascii_and_cjk_mix() {
        let ascii = estimate_text_width_px("ABC123", 10.0);
        let cjk = estimate_text_width_px("漢字かな", 10.0);
        let mixed = estimate_text_width_px("A漢1", 10.0);

        assert!((ascii - 35.4).abs() < 0.001, "ascii={ascii}");
        assert!((cjk - 40.0).abs() < 0.001, "cjk={cjk}");
        assert!((mixed - 21.8).abs() < 0.001, "mixed={mixed}");
    }

    #[test]
    fn label_overflow_detects_bar_label_exceeding_available_width() {
        let ir = TimelineIr {
            meta: mk_meta((0, 10)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Span {
                id: "s1".into(),
                lane: "x".into(),
                start: 0,
                end: 1,
                label: "Very long label 漢字".into(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 20.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts.clone()).unwrap();
        let item = layout.items.first().expect("span should be laid out");

        assert!(
            label_available_width_px(item, &opts, layout.total_width, layout.total_height) < 25.0
        );
        assert!(label_overflows_item(
            item,
            &opts,
            12.0,
            layout.total_width,
            layout.total_height
        ));
    }

    #[test]
    fn label_overflow_allows_short_bar_label() {
        let ir = TimelineIr {
            meta: mk_meta((0, 10)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::EventRange {
                id: "r1".into(),
                lane: "x".into(),
                start: 0,
                end: 10,
                label: "OK".into(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 40.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts.clone()).unwrap();
        let item = layout
            .items
            .first()
            .expect("event range should be laid out");

        assert!(!label_overflows_item(
            item,
            &opts,
            12.0,
            layout.total_width,
            layout.total_height
        ));
    }

    #[test]
    fn span_thickness_follows_lane_height() {
        // #507: bar thickness scales with lane_height. Default (60) keeps the
        // historical 24px span; doubling lane_height doubles the bar thickness.
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Span {
                id: "s1".into(),
                lane: "x".into(),
                start: 10,
                end: 50,
                label: "S".into(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let span_height = |lane_height: f64| {
            let opts = RenderOptions {
                lane_height,
                ..RenderOptions::default()
            };
            LayoutModel::compute(&ir, opts)
                .unwrap()
                .items
                .iter()
                .find_map(|i| match i {
                    LaidItem::Span { height, .. } => Some(*height),
                    _ => None,
                })
                .expect("span should be laid out")
        };
        // Default lane_height (60) reproduces the historical 24px (2 * SPAN_HALF_H).
        assert!((span_height(60.0) - 24.0).abs() < 0.001);
        // Doubling lane_height doubles bar thickness (density factor 2.0).
        assert!((span_height(120.0) - 48.0).abs() < 0.001);
        // Halving lane_height halves it.
        assert!((span_height(30.0) - 12.0).abs() < 0.001);
    }

    #[test]
    fn overlapping_spans_stack_and_expand_lane_height() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "s1".into(),
                    lane: "x".into(),
                    start: 10,
                    end: 50,
                    label: "S1".into(),
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
                },
                Item::Span {
                    id: "s2".into(),
                    lane: "x".into(),
                    start: 20,
                    end: 60,
                    label: "S2".into(),
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
                },
            ],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let ys: Vec<f64> = layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::Span { y, .. } => Some(*y),
                _ => None,
            })
            .collect();

        assert_eq!(ys.len(), 2);
        assert!((ys[1] - ys[0] - 40.0).abs() < 0.001);
        assert!((layout.lane_bands[0].height - 100.0).abs() < 0.001);
        assert!((layout.total_height - 160.0).abs() < 0.001);
    }

    #[test]
    fn touching_spans_share_base_row_without_expansion() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "s1".into(),
                    lane: "x".into(),
                    start: 10,
                    end: 20,
                    label: "S1".into(),
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
                    end_month: Some(1),
                    end_day: Some(1),
                    end_hour: Some(0),
                    end_minute: Some(0),
                    end_second: None,
                    end_offset_minutes: None,
                    end_open: false,
                    source_span: None,
                },
                Item::Span {
                    id: "s2".into(),
                    lane: "x".into(),
                    start: 20,
                    end: 30,
                    label: "S2".into(),
                    tags: vec![],
                    source: None,
                    origin: None,
                    note: None,
                    link: None,
                    color: None,
                    start_month: Some(1),
                    start_day: Some(1),
                    start_hour: Some(0),
                    start_minute: Some(0),
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
                },
            ],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let ys: Vec<f64> = layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::Span { y, .. } => Some(*y),
                _ => None,
            })
            .collect();

        assert_eq!(ys, vec![58.0, 58.0]);
        assert!((layout.lane_bands[0].height - 60.0).abs() < 0.001);
    }

    #[test]
    fn vertical_overlap_stacking_expands_lane_width() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::EventRange {
                    id: "r1".into(),
                    lane: "x".into(),
                    start: 10,
                    end: 50,
                    label: "R1".into(),
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
                },
                Item::EventRange {
                    id: "r2".into(),
                    lane: "x".into(),
                    start: 20,
                    end: 60,
                    label: "R2".into(),
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
                },
            ],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            orientation: Orientation::Vertical,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let xs: Vec<f64> = layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::EventRange { x, .. } => Some(*x),
                _ => None,
            })
            .collect();

        assert_eq!(xs.len(), 2);
        assert!((xs[1] - xs[0] - 40.0).abs() < 0.001);
        assert!((layout.lane_bands[0].width - 100.0).abs() < 0.001);
        assert!((layout.total_width - 240.0).abs() < 0.001);
    }

    #[test]
    fn group_bands_cover_expanded_lane_heights() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: Some("G".into()),
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "s1".into(),
                    lane: "x".into(),
                    start: 10,
                    end: 50,
                    label: "S1".into(),
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
                },
                Item::Span {
                    id: "s2".into(),
                    lane: "x".into(),
                    start: 20,
                    end: 60,
                    label: "S2".into(),
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
                },
            ],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            layout_style: LayoutStyle::GroupBands,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();

        assert_eq!(layout.group_bands.len(), 1);
        assert!((layout.group_bands[0].height - 100.0).abs() < 0.001);
    }

    // ─── #565 Zigzag layout style tests ────────────────────────────────────────

    fn event_item(id: &str, lane: &str, time: i64, label: &str) -> Item {
        Item::Event {
            id: id.into(),
            lane: lane.into(),
            time,
            label: label.into(),
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

    #[test]
    fn zigzag_single_lane_alternates_cross_axis_offset_by_start_order() {
        // #565: four Events in one lane, sorted by start time, must alternate
        // sides of the lane axis: even index above (offset), odd index below.
        let ir = TimelineIr {
            meta: mk_meta((2000, 2010)),
            lanes: vec![Lane {
                id: "events".into(),
                label: "Events".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                event_item("a", "events", 2001, "A"),
                event_item("b", "events", 2003, "B"),
                event_item("c", "events", 2005, "C"),
                event_item("d", "events", 2007, "D"),
            ],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            layout_style: LayoutStyle::Zigzag,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();

        let lane_axis = layout.lane_y["events"];
        let cross: Vec<f64> = layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::Event { y_dot, .. } => Some(*y_dot),
                _ => None,
            })
            .collect();
        assert_eq!(cross.len(), 4);
        // A (index 0, even) and C (index 2, even) share one side...
        assert!((cross[0] - cross[2]).abs() < 0.001);
        // ...B (index 1, odd) and D (index 3, odd) share the other...
        assert!((cross[1] - cross[3]).abs() < 0.001);
        // ...and the two sides are on opposite sides of the lane axis, each
        // offset by a non-zero amount.
        assert!((cross[0] - lane_axis).abs() > 1.0);
        assert!((cross[1] - lane_axis).abs() > 1.0);
        assert!(
            (cross[0] - lane_axis) * (cross[1] - lane_axis) < 0.0,
            "even/odd items must be offset to opposite sides of the lane axis: {cross:?} (lane_axis={lane_axis})"
        );
    }

    #[test]
    fn zigzag_disabled_by_default_leaves_items_on_lane_axis() {
        let ir = TimelineIr {
            meta: mk_meta((2000, 2010)),
            lanes: vec![Lane {
                id: "events".into(),
                label: "Events".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                event_item("a", "events", 2001, "A"),
                event_item("b", "events", 2003, "B"),
            ],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let lane_axis = layout.lane_y["events"];
        for item in &layout.items {
            if let LaidItem::Event { y_dot, .. } = item {
                assert!((*y_dot - lane_axis).abs() < 0.001);
            }
        }
    }

    #[test]
    fn zigzag_errors_when_lane_count_exceeds_threshold() {
        // #565: with more than ZIGZAG_MAX_LANES lanes, Zigzag must not silently
        // fall back to Timeline positioning. It must return an explicit
        // UnsupportedLayout error (implementation-strict.md / CLAUDE.md
        // "No silent fallback").
        let lanes: Vec<Lane> = (0..(ZIGZAG_MAX_LANES + 1))
            .map(|i| Lane {
                id: format!("lane{i}"),
                label: format!("Lane {i}"),
                kind: "custom".into(),
                order: i as i64,
                group: None,
                source_span: None,
            })
            .collect();
        let items: Vec<Item> = lanes
            .iter()
            .enumerate()
            .map(|(i, lane)| event_item(&format!("e{i}"), &lane.id, 2001 + i as i64, "E"))
            .collect();
        let ir = TimelineIr {
            meta: mk_meta((2000, 2010)),
            lanes,
            items,
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            layout_style: LayoutStyle::Zigzag,
            ..RenderOptions::default()
        };
        let result = LayoutModel::compute(&ir, opts);
        assert!(
            matches!(
                result,
                Err(RenderError::UnsupportedLayout {
                    style,
                    lane_count,
                    ..
                }) if style == "zigzag" && lane_count == ZIGZAG_MAX_LANES + 1
            ),
            "exceeding ZIGZAG_MAX_LANES must return UnsupportedLayout error"
        );
    }

    #[test]
    fn zigzag_and_bar_stacking_are_mutually_exclusive() {
        // #565/#549 interaction: two overlapping Spans in the same (single)
        // lane would normally trigger #549 sub-row stacking (non-zero
        // bar_stack_level cross-axis offset). Under Zigzag, that #549 offset
        // must NOT also apply — the cross-axis position is fully determined by
        // the zigzag parity, not by bar_stack_level.
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "s1".into(),
                    lane: "x".into(),
                    start: 10,
                    end: 50,
                    label: "S1".into(),
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
                },
                Item::Span {
                    id: "s2".into(),
                    lane: "x".into(),
                    start: 20,
                    end: 60,
                    label: "S2".into(),
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
                },
            ],
            imports: vec![],
            sources: vec![],
        };

        // Sanity check: without Zigzag, #549 stacking pushes s2 to level 1
        // (non-zero Y offset from s1), as covered by the pre-existing
        // `vertical_overlap_stacking_expands_lane_width`/horizontal equivalents.
        let timeline_layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let timeline_ys: Vec<f64> = timeline_layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::Span { y, .. } => Some(*y),
                _ => None,
            })
            .collect();
        assert!(
            (timeline_ys[0] - timeline_ys[1]).abs() > 0.001,
            "sanity check: #549 stacking must offset overlapping spans without Zigzag"
        );

        // Under Zigzag, the cross-axis position instead follows even/odd start
        // order (s1 starts first = index 0/even, s2 = index 1/odd), fully
        // replacing the #549 level-based offset.
        let zigzag_opts = RenderOptions {
            layout_style: LayoutStyle::Zigzag,
            ..RenderOptions::default()
        };
        let zigzag_layout = LayoutModel::compute(&ir, zigzag_opts).unwrap();
        let lane_axis = zigzag_layout.lane_y["x"];
        let zigzag_ys: Vec<f64> = zigzag_layout
            .items
            .iter()
            .filter_map(|i| match i {
                LaidItem::Span { y, height, .. } => Some(*y + *height / 2.0),
                _ => None,
            })
            .collect();
        assert!(
            (zigzag_ys[0] - lane_axis) * (zigzag_ys[1] - lane_axis) < 0.0,
            "Zigzag must place overlapping spans on opposite sides of the lane axis: {zigzag_ys:?} (lane_axis={lane_axis})"
        );
    }

    #[test]
    fn lane_y_ordered_by_order_field() {
        let ir = TimelineIr {
            meta: mk_meta((-100, 100)),
            lanes: vec![
                Lane {
                    id: "b".into(),
                    label: "B".into(),
                    kind: "k".into(),
                    order: 20,
                    group: None,
                    source_span: None,
                },
                Lane {
                    id: "a".into(),
                    label: "A".into(),
                    kind: "k".into(),
                    order: 10,
                    group: None,
                    source_span: None,
                },
            ],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        let ya = layout.lane_y["a"];
        let yb = layout.lane_y["b"];
        assert!(
            ya < yb,
            "lane a (order 10) should be above lane b (order 20)"
        );
    }

    #[test]
    fn empty_ir_does_not_panic() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.items.is_empty());
    }

    #[test]
    fn span_clamps_to_range() {
        let (x, w) = primary_axis_segment(-600.0, 300.0, -500, 200, 2.0, 120.0);
        // start clamped to -500 → x=120
        assert_eq!(x, 120.0);
        // end clamped to 200 → width = (200-(-500))*2 = 1400
        assert_eq!(w, 1400.0);
    }

    #[test]
    fn primary_axis_segment_matches_anchor_for_vertical() {
        // Same arithmetic as the horizontal case but with a different anchor
        // (top_margin instead of left_gutter); ensures the unified helper
        // covers the orientation that previously had its own
        // span_y_height_frac_vertical implementation.
        let (y, h) = primary_axis_segment(-600.0, 300.0, -500, 200, 2.0, 40.0);
        assert_eq!(y, 40.0);
        assert_eq!(h, 1400.0);
    }

    #[test]
    fn month_precision_shifts_x_position() {
        // February (month=2) should be 1/12 of a year to the right of January (no month).
        let x_jan = frac_to_x(to_year_frac(100, None, None, None, None), 0, 2.0, 0.0);
        let x_feb = frac_to_x(to_year_frac(100, Some(2), None, None, None), 0, 2.0, 0.0);
        assert!((x_feb - x_jan - 2.0 / 12.0).abs() < 0.001);
    }

    // ─── to_year_frac 精度テスト ──────────────────────────────────────────

    #[test]
    fn to_year_frac_year_only() {
        // 年のみ指定: フラクショナル値 = 整数年
        assert_eq!(to_year_frac(1939, None, None, None, None), 1939.0);
        assert_eq!(to_year_frac(-206, None, None, None, None), -206.0);
        assert_eq!(to_year_frac(0, None, None, None, None), 0.0);
    }

    #[test]
    fn to_year_frac_with_month() {
        // month=1 は +0/12、month=7 は +6/12 ≈ +0.5
        assert_eq!(to_year_frac(1939, Some(1), None, None, None), 1939.0);
        let mid = to_year_frac(1939, Some(7), None, None, None);
        assert!(
            (mid - 1939.5).abs() < 0.001,
            "month=7 should be ~0.5 offset, got {mid}"
        );
        // month=12 は +11/12 ≈ +0.917
        let dec = to_year_frac(1939, Some(12), None, None, None);
        assert!(
            (dec - (1939.0 + 11.0 / 12.0)).abs() < 0.001,
            "month=12 offset wrong, got {dec}"
        );
    }

    #[test]
    fn to_year_frac_with_month_and_day() {
        // month=1, day=1: オフセットなし
        assert_eq!(to_year_frac(1939, Some(1), Some(1), None, None), 1939.0);
        // month=1, day=2: +1/365.25 オフセット
        let d2 = to_year_frac(1939, Some(1), Some(2), None, None);
        assert!(
            (d2 - (1939.0 + 1.0 / 365.25)).abs() < 0.0001,
            "day=2 offset wrong, got {d2}"
        );
        // month=3, day=15: month offset + day offset
        let m3d15 = to_year_frac(1939, Some(3), Some(15), None, None);
        let expected = 1939.0 + 2.0 / 12.0 + 14.0 / 365.25;
        assert!(
            (m3d15 - expected).abs() < 0.0001,
            "month=3,day=15 wrong, got {m3d15}"
        );
    }

    // ─── month_ticks テスト ──────────────────────────────────────────────

    #[test]
    fn month_ticks_empty_when_unit_not_month() {
        let ir = TimelineIr {
            meta: mk_meta_with_unit("year", (1939, 1945)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.month_ticks().is_empty());
    }

    #[test]
    fn month_ticks_empty_when_scale_too_small() {
        // scale/12 < 1.0 のとき空配列
        let ir = TimelineIr {
            meta: mk_meta_with_unit("month", (1939, 1945)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 6.0, // 6/12 = 0.5 < 1.0
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        assert!(layout.month_ticks().is_empty());
    }

    #[test]
    fn month_ticks_produced_for_month_unit_sufficient_scale() {
        // scale/12 >= 1.0 のとき month=2..=12 のティックを返す
        let ir = TimelineIr {
            meta: mk_meta_with_unit("month", (1939, 1940)),
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 24.0, // 24/12 = 2.0 >= 1.0
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts).unwrap();
        let ticks = layout.month_ticks();
        assert!(!ticks.is_empty(), "expected month ticks for month unit");
        // 月初 (month=1) はティックに含まれない（年目盛と重複回避）
        assert!(
            !ticks.contains(&(1939, 1)),
            "month=1 should not appear in month_ticks"
        );
        // February は含まれる
        assert!(
            ticks.contains(&(1939, 2)),
            "expected (1939,2) in month_ticks"
        );
        // December は含まれる
        assert!(
            ticks.contains(&(1939, 12)),
            "expected (1939,12) in month_ticks"
        );
    }

    #[test]
    fn event_outside_range_is_skipped() {
        let ir = TimelineIr {
            meta: mk_meta((0, 100)),
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Event {
                id: "e1".into(),
                lane: "x".into(),
                time: 500,
                label: "outside".into(),
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
            }],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default()).unwrap();
        assert!(layout.items.is_empty());
    }
}
