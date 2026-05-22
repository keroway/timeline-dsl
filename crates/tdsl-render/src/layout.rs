use std::collections::HashMap;

use tdsl_core::ir::{Item, Lane, TimelineIr, end_frac, start_frac};

/// Color/style theme for HTML output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Default,
    Dark,
    Print,
    Pastel,
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
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            scale: 2.0,
            lane_height: 60.0,
            left_gutter: 120.0,
            top_margin: 40.0,
            right_margin: 20.0,
            bottom_margin: 20.0,
            theme: Theme::Default,
            custom_css: None,
            color_map: std::collections::HashMap::new(),
            interactive: false,
            font_family: None,
        }
    }
}

/// Item kind in its laid-out form (y offset from lane center already applied).
#[derive(Debug, Clone)]
pub enum LaidItem<'a> {
    Span {
        item: &'a Item,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    EventRange {
        item: &'a Item,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Event {
        item: &'a Item,
        x: f64,
        y_top: f64,
        y_bottom: f64,
        y_dot: f64,
    },
}

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

        let mut lane_y = HashMap::new();
        for (idx, lane) in lanes_ordered.iter().enumerate() {
            let center = opts.top_margin + (idx as f64 + 0.5) * opts.lane_height;
            lane_y.insert(lane.id.clone(), center);
        }

        let total_width =
            opts.left_gutter + (year_max - year_min) as f64 * opts.scale + opts.right_margin;
        let total_height =
            opts.top_margin + lanes_ordered.len() as f64 * opts.lane_height + opts.bottom_margin;

        let tick_step = pick_tick_step(year_max - year_min, opts.scale, AXIS_LABEL_PX);

        let mut items = Vec::new();
        for item in &ir.items {
            let lane_id = item_lane_id(item);
            let Some(&lane_cy) = lane_y.get(lane_id) else {
                continue;
            };
            match item {
                Item::Span {
                    start,
                    end,
                    start_month,
                    start_day,
                    end_month,
                    end_day,
                    ..
                } => {
                    // 仕様 §1.4: start は year/月の頭、end は year/月の末日を採用（混在精度補完）
                    let sf = start_frac(*start, *start_month, *start_day);
                    let ef = end_frac(*end, *end_month, *end_day);
                    let (x, width) =
                        span_x_width_frac(sf, ef, year_min, year_max, opts.scale, opts.left_gutter);
                    items.push(LaidItem::Span {
                        item,
                        x,
                        y: lane_cy - SPAN_HALF_H,
                        width,
                        height: SPAN_HALF_H * 2.0,
                    });
                }
                Item::EventRange {
                    start,
                    end,
                    start_month,
                    start_day,
                    end_month,
                    end_day,
                    ..
                } => {
                    let sf = start_frac(*start, *start_month, *start_day);
                    let ef = end_frac(*end, *end_month, *end_day);
                    let (x, width) =
                        span_x_width_frac(sf, ef, year_min, year_max, opts.scale, opts.left_gutter);
                    items.push(LaidItem::EventRange {
                        item,
                        x,
                        y: lane_cy + EVENT_RANGE_Y_OFFSET,
                        width,
                        height: EVENT_RANGE_H,
                    });
                }
                Item::Event {
                    time,
                    time_month,
                    time_day,
                    ..
                } => {
                    if !year_in_range(*time, year_min, year_max) {
                        continue;
                    }
                    let frac = to_year_frac(*time, *time_month, *time_day);
                    let x = frac_to_x(frac, year_min, opts.scale, opts.left_gutter);
                    items.push(LaidItem::Event {
                        item,
                        x,
                        y_top: lane_cy - EVENT_STEM_H,
                        y_bottom: lane_cy + EVENT_STEM_H,
                        y_dot: lane_cy,
                    });
                }
            }
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
                let frac = to_year_frac(year, Some(month), None);
                if frac < self.year_max as f64 {
                    ticks.push((year, month));
                }
            }
        }
        ticks
    }

    /// X coordinate for a (year, month) fractional position.
    pub fn frac_year_to_x(&self, year: i64, month: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), None);
        frac_to_x(frac, self.year_min, self.opts.scale, self.opts.left_gutter)
    }

    /// X coordinate for a (year, month, day) fractional position.
    pub fn day_frac_to_x(&self, year: i64, month: u8, day: u8) -> f64 {
        let frac = to_year_frac(year, Some(month), Some(day));
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
                        let frac = to_year_frac(year, Some(month), Some(day));
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
}

// --- sub-layout constants ---
const SPAN_HALF_H: f64 = 12.0;
/// Approximate rendered width (px) of the longest axis label ("BC9999" at 11 px font-size).
const AXIS_LABEL_PX: f64 = 40.0;
const EVENT_RANGE_Y_OFFSET: f64 = 14.0;
const EVENT_RANGE_H: f64 = 10.0;
const EVENT_STEM_H: f64 = 20.0;

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

fn year_to_x(year: i64, year_min: i64, scale: f64, left_gutter: f64) -> f64 {
    left_gutter + (year - year_min) as f64 * scale
}

/// Convert year + optional month + optional day to a fractional year value.
fn to_year_frac(year: i64, month: Option<u8>, day: Option<u8>) -> f64 {
    let mut frac = year as f64;
    if let Some(m) = month {
        frac += (m.clamp(1, 12) - 1) as f64 / 12.0;
        if let Some(d) = day {
            frac += (d.clamp(1, 31) - 1) as f64 / 365.25;
        }
    }
    frac
}

fn frac_to_x(frac: f64, year_min: i64, scale: f64, left_gutter: f64) -> f64 {
    left_gutter + (frac - year_min as f64) * scale
}

fn year_in_range(year: i64, year_min: i64, year_max: i64) -> bool {
    year >= year_min && year <= year_max
}

fn span_x_width_frac(
    start_frac: f64,
    end_frac: f64,
    year_min: i64,
    year_max: i64,
    scale: f64,
    left_gutter: f64,
) -> (f64, f64) {
    let s = start_frac.max(year_min as f64);
    let e = end_frac.min(year_max as f64);
    if e < s {
        return (frac_to_x(start_frac, year_min, scale, left_gutter), 0.0);
    }
    (frac_to_x(s, year_min, scale, left_gutter), (e - s) * scale)
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
                end_month: None,
                end_day: None,
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
    fn lane_y_ordered_by_order_field() {
        let ir = TimelineIr {
            meta: mk_meta((-100, 100)),
            lanes: vec![
                Lane {
                    id: "b".into(),
                    label: "B".into(),
                    kind: "k".into(),
                    order: 20,
                },
                Lane {
                    id: "a".into(),
                    label: "A".into(),
                    kind: "k".into(),
                    order: 10,
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
        let (x, w) = span_x_width_frac(-600.0, 300.0, -500, 200, 2.0, 120.0);
        // start clamped to -500 → x=120
        assert_eq!(x, 120.0);
        // end clamped to 200 → width = (200-(-500))*2 = 1400
        assert_eq!(w, 1400.0);
    }

    #[test]
    fn month_precision_shifts_x_position() {
        // February (month=2) should be 1/12 of a year to the right of January (no month).
        let x_jan = frac_to_x(to_year_frac(100, None, None), 0, 2.0, 0.0);
        let x_feb = frac_to_x(to_year_frac(100, Some(2), None), 0, 2.0, 0.0);
        assert!((x_feb - x_jan - 2.0 / 12.0).abs() < 0.001);
    }

    // ─── to_year_frac 精度テスト ──────────────────────────────────────────

    #[test]
    fn to_year_frac_year_only() {
        // 年のみ指定: フラクショナル値 = 整数年
        assert_eq!(to_year_frac(1939, None, None), 1939.0);
        assert_eq!(to_year_frac(-206, None, None), -206.0);
        assert_eq!(to_year_frac(0, None, None), 0.0);
    }

    #[test]
    fn to_year_frac_with_month() {
        // month=1 は +0/12、month=7 は +6/12 ≈ +0.5
        assert_eq!(to_year_frac(1939, Some(1), None), 1939.0);
        let mid = to_year_frac(1939, Some(7), None);
        assert!(
            (mid - 1939.5).abs() < 0.001,
            "month=7 should be ~0.5 offset, got {mid}"
        );
        // month=12 は +11/12 ≈ +0.917
        let dec = to_year_frac(1939, Some(12), None);
        assert!(
            (dec - (1939.0 + 11.0 / 12.0)).abs() < 0.001,
            "month=12 offset wrong, got {dec}"
        );
    }

    #[test]
    fn to_year_frac_with_month_and_day() {
        // month=1, day=1: オフセットなし
        assert_eq!(to_year_frac(1939, Some(1), Some(1)), 1939.0);
        // month=1, day=2: +1/365.25 オフセット
        let d2 = to_year_frac(1939, Some(1), Some(2));
        assert!(
            (d2 - (1939.0 + 1.0 / 365.25)).abs() < 0.0001,
            "day=2 offset wrong, got {d2}"
        );
        // month=3, day=15: month offset + day offset
        let m3d15 = to_year_frac(1939, Some(3), Some(15));
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
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        assert!(layout.items.is_empty());
    }
}
