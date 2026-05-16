use std::collections::HashMap;
use std::fmt::Write;

use tdsl_core::ir::Item;

use crate::layout::{LaidItem, LayoutModel};

/// Colorblind-friendly 8-color palette for per-lane fill colors.
const LANE_PALETTE: &[&str] = &[
    "#4682B4", // steel blue
    "#E67E22", // orange
    "#27AE60", // green
    "#8E44AD", // purple
    "#E74C3C", // red
    "#1ABC9C", // teal
    "#F39C12", // amber
    "#2980B9", // blue
];

/// Render the SVG for a laid-out timeline. Pure string builder, no external deps.
pub fn render_svg(layout: &LayoutModel) -> String {
    let mut s = String::new();
    let w = layout.total_width;
    let h = layout.total_height;

    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="timeline">"#,
        w = fmt_f(w),
        h = fmt_f(h)
    )
    .unwrap();

    // Embed font-family and axis text size for standalone SVG viewers (no CDN dependency).
    writeln!(
        s,
        r#"  <style>text {{ font-family: "Noto Sans JP", "Noto Sans CJK JP", "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif; }} .tdsl-axis-text {{ font-size: 11px; }} .tdsl-axis-month-tick {{ stroke: #ccc; stroke-width: 1; }} .tdsl-axis-day-tick {{ stroke: #ddd; stroke-width: 1; }} .tdsl-axis-day-text {{ font-size: 9px; fill: #888; }}</style>"#
    )
    .unwrap();

    // Build lane_id → palette color map from ordered lane list.
    let lane_color: HashMap<&str, &str> = layout
        .lanes_ordered
        .iter()
        .enumerate()
        .map(|(idx, lane)| (lane.id.as_str(), LANE_PALETTE[idx % LANE_PALETTE.len()]))
        .collect();

    render_lane_bands(&mut s, layout);
    render_axis(&mut s, layout);
    render_lane_labels(&mut s, layout);
    render_items(
        &mut s,
        layout,
        &lane_color,
        &layout.opts.color_map,
        layout.opts.interactive,
    );

    writeln!(s, "</svg>").unwrap();
    s
}

fn render_lane_bands(s: &mut String, layout: &LayoutModel) {
    let x = layout.opts.left_gutter;
    let width = layout.total_width - x - layout.opts.right_margin;
    for (idx, lane) in layout.lanes_ordered.iter().enumerate() {
        let y = layout.opts.top_margin + idx as f64 * layout.opts.lane_height;
        let class = if idx % 2 == 0 {
            "tdsl-lane-band-even"
        } else {
            "tdsl-lane-band-odd"
        };
        writeln!(
            s,
            r#"  <rect class="{class}" x="{x}" y="{y}" width="{w}" height="{h}"/>"#,
            x = fmt_f(x),
            y = fmt_f(y),
            w = fmt_f(width),
            h = fmt_f(layout.opts.lane_height),
        )
        .unwrap();
        // Invisible bottom border for subtle lane separation.
        let _ = lane;
    }
}

fn render_axis(s: &mut String, layout: &LayoutModel) {
    let top = layout.opts.top_margin;
    let bottom = layout.total_height - layout.opts.bottom_margin;

    // Horizontal baseline at the top.
    let baseline_y = top - 4.0;
    writeln!(
        s,
        r#"  <line class="tdsl-axis-baseline" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}"/>"#,
        x1 = fmt_f(layout.opts.left_gutter),
        y = fmt_f(baseline_y),
        x2 = fmt_f(layout.total_width - layout.opts.right_margin),
    )
    .unwrap();

    for year in layout.ticks() {
        let x = layout.year_to_x(year);
        // Vertical grid line across the full chart body.
        writeln!(
            s,
            r#"  <line class="tdsl-axis-tick" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(top),
            y2 = fmt_f(bottom),
        )
        .unwrap();
        let label = format_year(year);
        writeln!(
            s,
            r#"  <text class="tdsl-axis-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
            x = fmt_f(x),
            y = fmt_f(top - 8.0),
            label = escape_xml(&label),
        )
        .unwrap();
    }

    // Month minor ticks (unit=month only, hidden when scale too small).
    let px_per_month = layout.opts.scale / 12.0;
    for (year, month) in layout.month_ticks() {
        let x = layout.frac_year_to_x(year, month);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-month-tick" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 3.0),
            y2 = fmt_f(baseline_y),
        )
        .unwrap();
        if px_per_month >= 20.0 {
            let label = month_abbr(month);
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-month-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
            )
            .unwrap();
        }
    }

    // Day minor ticks (unit=day only, hidden when scale too small).
    // 月初には `YYYY-MM` ラベルを表示し、それ以外は短い tick のみ。
    let pixels_per_day = layout.opts.scale / 365.25;
    for (year, month, day) in layout.day_ticks() {
        let x = layout.day_frac_to_x(year, month, day);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-day-tick" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 2.0),
            y2 = fmt_f(baseline_y),
        )
        .unwrap();
        if day == 1 && pixels_per_day >= 1.5 {
            // 月またぎラベル: YYYY-MM
            let label = format!("{year:04}-{month:02}");
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-day-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
                label = escape_xml(&label),
            )
            .unwrap();
        } else if pixels_per_day >= 8.0 {
            // 日番号ラベル（密度が十分なときのみ）
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-day-text" x="{x}" y="{y}" text-anchor="middle">{day}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
            )
            .unwrap();
        }
    }
}

fn render_lane_labels(s: &mut String, layout: &LayoutModel) {
    for lane in &layout.lanes_ordered {
        let y = layout.lane_y[&lane.id];
        writeln!(
            s,
            r#"  <text class="tdsl-lane-label" data-lane="{lane_id}" x="{x}" y="{y}" text-anchor="end" dominant-baseline="middle">{label}</text>"#,
            lane_id = escape_xml_attr(&lane.id),
            x = fmt_f(layout.opts.left_gutter - 8.0),
            y = fmt_f(y),
            label = escape_xml(&lane.label),
        )
        .unwrap();
    }
}

fn resolve_item_color(
    tags: &[String],
    color_map: &HashMap<String, String>,
    lane_fallback: &str,
    lane_color: &HashMap<&str, &str>,
) -> String {
    // Tag-based override takes priority: use the first matching tag.
    for tag in tags {
        if let Some(color) = color_map.get(tag.as_str()) {
            return color.clone();
        }
    }
    lane_color
        .get(lane_fallback)
        .copied()
        .unwrap_or("#4682B4")
        .to_string()
}

fn render_items(
    s: &mut String,
    layout: &LayoutModel,
    lane_color: &HashMap<&str, &str>,
    color_map: &HashMap<String, String>,
    interactive: bool,
) {
    for laid in &layout.items {
        match laid {
            LaidItem::Span {
                item,
                x,
                y,
                width,
                height,
            } => {
                let raw_tip = item_tooltip(item);
                let tip = escape_xml(&raw_tip);
                let tip_attr = escape_xml_attr(&raw_tip);
                let lane_id = item_lane_id(item);
                let tags = item_tags(item);
                let fill = resolve_item_color(tags, color_map, lane_id, lane_color);
                let fill_style = format!("fill:{fill};");
                let data_attrs = if interactive {
                    build_data_attrs(item, lane_id)
                } else {
                    String::new()
                };
                writeln!(
                    s,
                    r#"  <g class="tdsl-item tdsl-item-span" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-span" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="3"><title>{tip}</title></rect><text class="tdsl-item-label" x="{tx}" y="{ty}" dominant-baseline="middle">{label}</text></g>"#,
                    tip = tip,
                    tip_attr = tip_attr,
                    fill_style = fill_style,
                    data_attrs = data_attrs,
                    x = fmt_f(*x),
                    y = fmt_f(*y),
                    w = fmt_f(*width),
                    h = fmt_f(*height),
                    tx = fmt_f(*x + 4.0),
                    ty = fmt_f(*y + height / 2.0),
                    label = escape_xml(item_label(item)),
                )
                .unwrap();
            }
            LaidItem::EventRange {
                item,
                x,
                y,
                width,
                height,
            } => {
                let raw_tip = item_tooltip(item);
                let tip = escape_xml(&raw_tip);
                let tip_attr = escape_xml_attr(&raw_tip);
                let lane_id = item_lane_id(item);
                let tags = item_tags(item);
                let fill = resolve_item_color(tags, color_map, lane_id, lane_color);
                let fill_style = format!("fill:{fill};fill-opacity:0.75;");
                let data_attrs = if interactive {
                    build_data_attrs(item, lane_id)
                } else {
                    String::new()
                };
                writeln!(
                    s,
                    r#"  <g class="tdsl-item tdsl-item-event-range" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-range" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="2"><title>{tip}</title></rect></g>"#,
                    tip = tip,
                    tip_attr = tip_attr,
                    fill_style = fill_style,
                    data_attrs = data_attrs,
                    x = fmt_f(*x),
                    y = fmt_f(*y),
                    w = fmt_f(*width),
                    h = fmt_f(*height),
                )
                .unwrap();
            }
            LaidItem::Event {
                item,
                x,
                y_top,
                y_bottom,
                y_dot,
            } => {
                // An invisible wide hit-rect makes hovering the thin stem / small dot feasible.
                let raw_tip = item_tooltip(item);
                let tip = escape_xml(&raw_tip);
                let tip_attr = escape_xml_attr(&raw_tip);
                let lane_id = item_lane_id(item);
                let tags = item_tags(item);
                let fill = resolve_item_color(tags, color_map, lane_id, lane_color);
                let dot_style = format!("fill:{fill};");
                let hit_x = *x - 8.0;
                let hit_w = 16.0;
                let hit_y = *y_top;
                let hit_h = (y_bottom - y_top).max(20.0);
                let data_attrs = if interactive {
                    build_data_attrs(item, lane_id)
                } else {
                    String::new()
                };
                writeln!(
                    s,
                    r#"  <g class="tdsl-item tdsl-item-event" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-hit" x="{hx}" y="{hy}" width="{hw}" height="{hh}"><title>{tip}</title></rect><line class="tdsl-event-stem" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"><title>{tip}</title></line><circle class="tdsl-event-dot" style="{dot_style}" cx="{x}" cy="{cy}" r="4"><title>{tip}</title></circle></g>"#,
                    tip = tip,
                    tip_attr = tip_attr,
                    dot_style = dot_style,
                    data_attrs = data_attrs,
                    hx = fmt_f(hit_x),
                    hy = fmt_f(hit_y),
                    hw = fmt_f(hit_w),
                    hh = fmt_f(hit_h),
                    x = fmt_f(*x),
                    y1 = fmt_f(*y_top),
                    y2 = fmt_f(*y_bottom),
                    cy = fmt_f(*y_dot),
                )
                .unwrap();
            }
        }
    }
}

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

fn item_tags(item: &Item) -> &[String] {
    match item {
        Item::Span { tags, .. } | Item::Event { tags, .. } | Item::EventRange { tags, .. } => tags,
    }
}

/// Build data-* attributes for interactive mode as a string fragment (leading space included).
fn build_data_attrs(item: &Item, lane_id: &str) -> String {
    let (id, label, type_str, source, source_span) = match item {
        Item::Span {
            id,
            label,
            source,
            source_span,
            ..
        } => (
            id.as_str(),
            label.as_str(),
            "span",
            source.as_deref(),
            source_span.as_ref(),
        ),
        Item::Event {
            id,
            label,
            source,
            source_span,
            ..
        } => (
            id.as_str(),
            label.as_str(),
            "event",
            source.as_deref(),
            source_span.as_ref(),
        ),
        Item::EventRange {
            id,
            label,
            source,
            source_span,
            ..
        } => (
            id.as_str(),
            label.as_str(),
            "event_range",
            source.as_deref(),
            source_span.as_ref(),
        ),
    };
    let mut attrs = format!(
        r#" data-id="{}" data-label="{}" data-type="{}" data-lane="{}""#,
        escape_xml_attr(id),
        escape_xml_attr(label),
        type_str,
        escape_xml_attr(lane_id),
    );
    if let Some(src) = source {
        attrs.push_str(&format!(r#" data-source="{}""#, escape_xml_attr(src)));
    }
    if let Some(ss) = source_span {
        attrs.push_str(&format!(r#" data-line="{}""#, ss.line));
    }
    attrs
}

fn item_label(item: &Item) -> &str {
    match item {
        Item::Span { label, .. } | Item::Event { label, .. } | Item::EventRange { label, .. } => {
            label
        }
    }
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
            end_month,
            end_day,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day),
                format_date(*end, *end_month, *end_day),
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
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format_date(*time, *time_month, *time_day));
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
            end_month,
            end_day,
            ..
        } => {
            lines.push(label.to_string());
            lines.push(format!(
                "{}〜{}",
                format_date(*start, *start_month, *start_day),
                format_date(*end, *end_month, *end_day),
            ));
            push_common(&mut lines, tags, source, origin, id);
        }
    }
    lines.join("\n")
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

fn format_year(year: i64) -> String {
    if year < 0 {
        format!("BC{}", -year)
    } else {
        format!("{year}")
    }
}

fn month_abbr(m: u8) -> &'static str {
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

fn format_date(year: i64, month: Option<u8>, day: Option<u8>) -> String {
    let y = format_year(year);
    match (month, day) {
        (Some(m), Some(d)) => format!("{} {} {}", y, month_abbr(m), d),
        (Some(m), None) => format!("{} {}", y, month_abbr(m)),
        _ => y,
    }
}

/// Escape for SVG/XML text content and attribute values.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape for XML attribute values while preserving newlines as HTML entities.
fn escape_xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\n' => out.push_str("&#10;"),
            '\r' => {}
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Format float with up to 2 decimals, trimming trailing zeros.
fn fmt_f(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::RenderOptions;
    use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

    fn sample_ir() -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "test".into(),
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
            }],
            items: vec![
                Item::Span {
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
                    end_month: None,
                    end_day: None,
                    source_span: None,
                },
                Item::Event {
                    id: "event:han:-209".into(),
                    lane: "han".into(),
                    time: -209,
                    label: "陳勝・呉広の乱".into(),
                    tags: vec![],
                    source: None,
                    origin: None,
                    time_month: None,
                    time_day: None,
                    source_span: None,
                },
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn svg_contains_core_elements() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("tdsl-span"));
        assert!(svg.contains("tdsl-event-dot"));
    }

    #[test]
    fn svg_escapes_xml_in_labels() {
        let mut ir = sample_ir();
        ir.lanes[0].label = "<danger> & \"quoted\"".into();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout);
        assert!(svg.contains("&lt;danger&gt;"));
        assert!(svg.contains("&amp;"));
        assert!(svg.contains("&quot;"));
        assert!(!svg.contains("<danger>"));
    }

    #[test]
    fn svg_includes_tooltip_via_title_element() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout);
        assert!(svg.contains("<title>"));
        assert!(svg.contains("wd:Q7209"));
        assert!(svg.contains(r#"data-tdsl-tooltip="漢&#10;BC206〜220"#));
        assert!(svg.contains(r#"tabindex="0""#));
    }

    #[test]
    fn format_year_prefixes_bc_for_negative() {
        assert_eq!(format_year(-206), "BC206");
        assert_eq!(format_year(0), "0");
        assert_eq!(format_year(220), "220");
    }

    #[test]
    fn format_date_includes_month_abbr() {
        assert_eq!(format_date(1900, Some(2), None), "1900 Feb");
        assert_eq!(format_date(-206, Some(3), Some(15)), "BC206 Mar 15");
        assert_eq!(format_date(2000, None, None), "2000");
    }

    #[test]
    fn tooltip_includes_month_for_precision_event() {
        let ir = TimelineIr {
            meta: Meta {
                title: "test".into(),
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
            }],
            items: vec![Item::Event {
                id: "e1".into(),
                lane: "han".into(),
                time: -206,
                label: "漢建国".into(),
                tags: vec![],
                source: None,
                origin: None,
                time_month: Some(2),
                time_day: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout);
        assert!(
            svg.contains("BC206 Feb"),
            "expected 'BC206 Feb' in tooltip, got:\n{svg}"
        );
    }

    #[test]
    fn color_map_tag_overrides_lane_palette() {
        let ir = sample_ir();
        let color_map: std::collections::HashMap<String, String> =
            [("dynasty".to_string(), "#cc0000".to_string())]
                .into_iter()
                .collect();
        let opts = RenderOptions {
            color_map,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout);
        // The span item has tag "dynasty", so its fill must use the color_map color.
        assert!(
            svg.contains("fill:#cc0000;"),
            "expected fill:#cc0000; in SVG, got:\n{svg}"
        );
    }
}
