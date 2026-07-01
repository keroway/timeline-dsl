use std::collections::HashMap;

use tdsl_core::ir::{Item, Lane, TimelineIr, end_frac, start_frac};

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
    /// When true, labels (and optionally dates) are always rendered next to Event and EventRange
    /// dots/bars as SVG text elements.  Disabled by default to keep the chart uncluttered.
    pub show_event_labels: bool,
    /// When true (default), lane palette colours are emitted as CSS custom properties
    /// (`var(--tdsl-lane-N, #hex)`) in SVG inline styles, allowing embedding pages to
    /// override lane colours via `:root { --tdsl-lane-N: … }`. Set to false for raster
    /// renderers (`usvg`-based PNG/PDF) that do not support CSS custom properties.
    pub use_css_vars: bool,
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
            show_event_labels: false,
            use_css_vars: true,
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
/// Vertical gap (px) between the timeline body and the table.
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
    /// #536: Y coordinate (in the *final*, table-inclusive `total_height`) where the
    /// table's header row begins. Only meaningful when `opts.show_table` is true.
    pub(crate) table_top_y: f64,
}

impl<'a> LayoutModel<'a> {
    pub fn compute(ir: &'a TimelineIr, opts: RenderOptions) -> Self {
        let (year_min, year_max) = ir.meta.range;
        let (year_min, year_max) = if year_max > year_min {
            (year_min, year_max)
        } else if year_max == year_min {
            // 同一年内のレンジ（例: range 1939-09..1939-10）: items から導出せず一年幅を確保
            (year_min, year_max + 1)
        } else {
            // Fallback: if range is degenerate, derive from items.
            derive_range_from_items(ir).unwrap_or((0, 2000))
        };

        let mut lanes_ordered: Vec<&Lane> = ir.lanes.iter().collect();
        lanes_ordered.sort_by_key(|l| (l.order, l.id.clone()));

        let is_vertical = opts.orientation == Orientation::Vertical;
        let n_lanes = lanes_ordered.len();
        let time_span = (year_max - year_min) as f64;

        // lane_y stores:
        //   horizontal → lane center Y coordinate
        //   vertical   → lane center X coordinate (reusing the same field for "lane primary axis")
        let mut lane_y = HashMap::new();
        if is_vertical {
            for (idx, lane) in lanes_ordered.iter().enumerate() {
                // left_gutter is reserved for the time-axis labels on the left; lanes go rightward.
                let center = opts.left_gutter + (idx as f64 + 0.5) * opts.lane_height;
                lane_y.insert(lane.id.clone(), center);
            }
        } else {
            for (idx, lane) in lanes_ordered.iter().enumerate() {
                let center = opts.top_margin + (idx as f64 + 0.5) * opts.lane_height;
                lane_y.insert(lane.id.clone(), center);
            }
        }

        let (total_width, body_height) = if is_vertical {
            // vertical: time axis is Y, lanes are X columns.
            // lane_height is reused as the lane column width.
            let w = opts.left_gutter + n_lanes as f64 * opts.lane_height + opts.right_margin;
            let h = opts.top_margin + time_span * opts.scale + opts.bottom_margin;
            (w, h)
        } else {
            let w = opts.left_gutter + time_span * opts.scale + opts.right_margin;
            let h = opts.top_margin + n_lanes as f64 * opts.lane_height + opts.bottom_margin;
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
        let table_top_y = body_height + TABLE_TOP_GAP;
        let total_height = if opts.show_table {
            // header row + one row per item, plus the top gap already added above.
            table_top_y + (table_rows.len() as f64 + 1.0) * TABLE_ROW_HEIGHT + opts.bottom_margin
        } else {
            body_height
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
                .map(|(idx, _lane)| LaneBandModel {
                    x: opts.left_gutter + idx as f64 * opts.lane_height,
                    y: opts.top_margin,
                    width: opts.lane_height,
                    height: content_height,
                    even: idx % 2 == 0,
                })
                .collect()
        } else {
            let content_width = total_width - opts.left_gutter - opts.right_margin;
            lanes_ordered
                .iter()
                .enumerate()
                .map(|(idx, _lane)| LaneBandModel {
                    x: opts.left_gutter,
                    y: opts.top_margin + idx as f64 * opts.lane_height,
                    width: content_width,
                    height: opts.lane_height,
                    even: idx % 2 == 0,
                })
                .collect()
        };

        let group_bands =
            compute_group_bands(&lanes_ordered, &lane_y, &opts, body_height, total_width);

        let mut items = Vec::new();
        for item in &ir.items {
            let lane_id = item_lane_id(item);
            let Some(&lane_axis) = lane_y.get(lane_id) else {
                continue;
            };
            let item_tags = get_item_tags(item);
            let color = resolve_item_color(item_tags, &opts.color_map, lane_id, &lane_colors);
            let tooltip = item_tooltip(item);
            compute_item(
                item,
                &mut items,
                ItemLayoutArgs {
                    lane_axis,
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

        Self {
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
            table_top_y,
        }
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
    pub fn grid_positions(&self) -> Vec<f64> {
        match self.opts.grid {
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
    year_min: i64,
    year_max: i64,
    opts: &'a RenderOptions,
    orientation: Orientation,
    color: String,
    tooltip: String,
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
            ..
        } => {
            // 仕様 §1.4: start は year/月の頭、end は year/月の末日を採用（混在精度補完）
            let sf =
                start_frac_with_time(*start, *start_month, *start_day, *start_hour, *start_minute);
            let ef = end_frac_with_time(*end, *end_month, *end_day, *end_hour, *end_minute);
            let (primary_start, primary_extent) =
                primary_axis_segment(sf, ef, year_min, year_max, opts.scale, primary_anchor);
            let cross_start = lane_axis - span_half_h;
            let cross_extent = span_half_h * 2.0;
            let (x, y, width, height) = if is_vertical {
                (cross_start, primary_start, cross_extent, primary_extent)
            } else {
                (primary_start, cross_start, primary_extent, cross_extent)
            };
            items.push(LaidItem::Span {
                item,
                x,
                y,
                width,
                height,
                color,
                tooltip,
            });
        }
        Item::EventRange {
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
            ..
        } => {
            let sf =
                start_frac_with_time(*start, *start_month, *start_day, *start_hour, *start_minute);
            let ef = end_frac_with_time(*end, *end_month, *end_day, *end_hour, *end_minute);
            let (primary_start, primary_extent) =
                primary_axis_segment(sf, ef, year_min, year_max, opts.scale, primary_anchor);
            // Horizontal bands sit just below the lane center
            // (EVENT_RANGE_Y_OFFSET); vertical bands are centered on the lane
            // axis. This asymmetry is preserved verbatim from the original
            // split implementation.
            let (x, y, width, height) = if is_vertical {
                (
                    lane_axis - event_range_h / 2.0,
                    primary_start,
                    event_range_h,
                    primary_extent,
                )
            } else {
                (
                    primary_start,
                    lane_axis + event_range_y_offset,
                    primary_extent,
                    event_range_h,
                )
            };
            items.push(LaidItem::EventRange {
                item,
                x,
                y,
                width,
                height,
                color,
                tooltip,
            });
        }
        Item::Event {
            time,
            time_month,
            time_day,
            time_hour,
            time_minute,
            ..
        } => {
            if !year_in_range(*time, year_min, year_max) {
                return;
            }
            let frac = to_year_frac(*time, *time_month, *time_day, *time_hour, *time_minute);
            let primary = primary_anchor + (frac - year_min as f64) * opts.scale;
            let (x, y_top, y_bottom, y_dot) = if is_vertical {
                // x = lane axis; y_top/y_bottom/y_dot all live on the time axis.
                (
                    lane_axis,
                    primary - event_stem_h,
                    primary + event_stem_h,
                    primary,
                )
            } else {
                // x = time axis; y_top/y_bottom/y_dot live on the lane axis.
                (
                    primary,
                    lane_axis - event_stem_h,
                    lane_axis + event_stem_h,
                    lane_axis,
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

fn get_item_tags(item: &Item) -> &[String] {
    match item {
        Item::Span { tags, .. } | Item::Event { tags, .. } | Item::EventRange { tags, .. } => tags,
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

/// Resolve item fill color: tag overrides take priority over lane palette.
pub(crate) fn resolve_item_color(
    tags: &[String],
    color_map: &HashMap<String, String>,
    lane_id: &str,
    lane_colors: &HashMap<String, String>,
) -> String {
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

fn push_common(
    lines: &mut Vec<String>,
    tags: &[String],
    source: &Option<String>,
    origin: &Option<String>,
    id: &str,
) {
    if !tags.is_empty() {
        lines.push(format!("tags: {}", tags.join(", ")));
    }
    if let Some(src) = source {
        lines.push(format!("source: {src}"));
    }
    if let Some(org) = origin {
        lines.push(format!("origin: {org}"));
    }
    lines.push(format!("id: {id}"));
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
    lane_y: &HashMap<String, f64>,
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
            let start_center = lane_y[&lanes_ordered[start_idx].id];
            let end_center = lane_y[&lanes_ordered[end_idx].id];
            let half = opts.lane_height / 2.0;
            if is_vertical {
                bands.push(GroupBandModel {
                    label: group_label.to_string(),
                    x: start_center - half,
                    y: opts.top_margin,
                    width: (end_center + half) - (start_center - half),
                    height: body_height - opts.top_margin - opts.bottom_margin,
                    even: band_idx.is_multiple_of(2),
                });
            } else {
                bands.push(GroupBandModel {
                    label: group_label.to_string(),
                    x: opts.left_gutter,
                    y: start_center - half,
                    width: total_width - opts.left_gutter - opts.right_margin,
                    height: (end_center + half) - (start_center - half),
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
            id,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                format_date(*end, *end_month, *end_day, *end_hour, *end_minute),
            ));
            push_common(&mut lines, tags, source, origin, id);
        }
        Item::Event {
            label,
            time,
            tags,
            source,
            origin,
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
            push_common(&mut lines, tags, source, origin, id);
        }
        Item::EventRange {
            label,
            start,
            end,
            tags,
            source,
            origin,
            id,
            start_month,
            start_day,
            start_hour,
            start_minute,
            end_month,
            end_day,
            end_hour,
            end_minute,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day, *start_hour, *start_minute),
                format_date(*end, *end_month, *end_day, *end_hour, *end_minute),
            ));
            push_common(&mut lines, tags, source, origin, id);
        }
    }
    lines.join("\n")
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
    let mut frac = year as f64;
    if let Some(m) = month {
        frac += (m.clamp(1, 12) - 1) as f64 / 12.0;
        if let Some(d) = day {
            frac += (d.clamp(1, 31) - 1) as f64 / 365.25;
            if let Some(h) = hour {
                frac += h.min(23) as f64 / 24.0 / 365.25;
                if let Some(min) = minute {
                    frac += min.min(59) as f64 / 1440.0 / 365.25;
                }
            }
        }
    }
    frac
}

fn start_frac_with_time(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
) -> f64 {
    start_frac(year, month, day)
        + hour.unwrap_or(0).min(23) as f64 / 24.0 / 365.25
        + minute.unwrap_or(0).min(59) as f64 / 1440.0 / 365.25
}

fn end_frac_with_time(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
) -> f64 {
    match hour {
        Some(_) => start_frac_with_time(year, month, day, hour, minute),
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        // With scale=2.0 and left_gutter=120, year -500 → x=120, year 0 → x=120+500*2=1120
        assert_eq!(layout.year_to_x(-500), 120.0);
        assert_eq!(layout.year_to_x(0), 1120.0);
        assert_eq!(layout.year_to_x(2000), 120.0 + 2500.0 * 2.0);
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        let layout = LayoutModel::compute(&ir, opts);
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
        let layout = LayoutModel::compute(&ir, opts);
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
        let layout = LayoutModel::compute(&ir, opts);
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
        let layout = LayoutModel::compute(&ir, opts);
        assert!(layout.day_ticks().is_empty());
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
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        };
        let opts = RenderOptions {
            scale: 20.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts.clone());
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
        };
        let opts = RenderOptions {
            scale: 40.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts.clone());
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
        };
        let span_height = |lane_height: f64| {
            let opts = RenderOptions {
                lane_height,
                ..RenderOptions::default()
            };
            LayoutModel::compute(&ir, opts)
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
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
        let layout = LayoutModel::compute(&ir, opts);
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
        let layout = LayoutModel::compute(&ir, opts);
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
                time_month: None,
                time_day: None,
                time_hour: None,
                time_minute: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        assert!(layout.items.is_empty());
    }
}
