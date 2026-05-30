use std::fmt::Write;

use tdsl_core::ir::Item;

use crate::layout::{LaidItem, LayoutModel, format_year, month_abbr};

/// Render the SVG for a laid-out timeline. Pure string builder, no external deps.
pub fn render_svg(layout: &LayoutModel) -> String {
    let mut s = String::new();
    let w = layout.total_width;
    let h = layout.total_height;

    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="timeline" class="tdsl-root">"#,
        w = fmt_f(w),
        h = fmt_f(h)
    )
    .unwrap();

    let font_family = layout
        .opts
        .font_family
        .as_deref()
        .unwrap_or(r#""Noto Sans JP", "Noto Sans CJK JP", "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif"#);

    // Embed font-family and axis text size for standalone SVG viewers (no CDN dependency).
    // Use .tdsl-root text selector to scope styles and prevent CSS leakage when embedded inline.
    writeln!(
        s,
        r#"  <style>.tdsl-root text {{ font-family: {font_family}; }} .tdsl-axis-text {{ font-size: 11px; }} .tdsl-axis-month-tick {{ stroke: #ccc; stroke-width: 1; }} .tdsl-axis-day-tick {{ stroke: #ddd; stroke-width: 1; }} .tdsl-axis-day-text {{ font-size: 9px; fill: #888; }}</style>"#
    )
    .unwrap();

    render_lane_bands(&mut s, layout);
    render_group_headers(&mut s, layout);
    render_axis(&mut s, layout);
    render_lane_labels(&mut s, layout);
    render_items(&mut s, layout);

    writeln!(s, "</svg>").unwrap();
    s
}

fn render_lane_bands(s: &mut String, layout: &LayoutModel) {
    for band in &layout.lane_bands {
        let class = if band.even {
            "tdsl-lane-band-even"
        } else {
            "tdsl-lane-band-odd"
        };
        writeln!(
            s,
            r#"  <rect class="{class}" x="{x}" y="{y}" width="{w}" height="{h}"/>"#,
            x = fmt_f(band.x),
            y = fmt_f(band.y),
            w = fmt_f(band.width),
            h = fmt_f(band.height),
        )
        .unwrap();
    }
}

fn render_axis(s: &mut String, layout: &LayoutModel) {
    if layout.is_vertical() {
        render_axis_vertical(s, layout);
    } else {
        render_axis_horizontal(s, layout);
    }
}

fn render_axis_horizontal(s: &mut String, layout: &LayoutModel) {
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

fn render_axis_vertical(s: &mut String, layout: &LayoutModel) {
    let left = layout.opts.left_gutter;
    let right = layout.total_width - layout.opts.right_margin;

    // Vertical baseline on the left side.
    let baseline_x = left - 4.0;
    writeln!(
        s,
        r#"  <line class="tdsl-axis-baseline" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
        x = fmt_f(baseline_x),
        y1 = fmt_f(layout.opts.top_margin),
        y2 = fmt_f(layout.total_height - layout.opts.bottom_margin),
    )
    .unwrap();

    for year in layout.ticks() {
        let y = layout.year_to_primary(year);
        // Horizontal grid line across the full chart body.
        writeln!(
            s,
            r#"  <line class="tdsl-axis-tick" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}"/>"#,
            x1 = fmt_f(left),
            y = fmt_f(y),
            x2 = fmt_f(right),
        )
        .unwrap();
        let label = format_year(year);
        writeln!(
            s,
            r#"  <text class="tdsl-axis-text" x="{x}" y="{y}" text-anchor="end" dominant-baseline="middle">{label}</text>"#,
            x = fmt_f(left - 8.0),
            y = fmt_f(y),
            label = escape_xml(&label),
        )
        .unwrap();
    }
}

fn render_group_headers(s: &mut String, layout: &LayoutModel) {
    // 各グループの先頭レーン（最も小さい y 座標 = 最上部）を特定してヘッダーを描画する。
    // グループ名をキーに、先頭レーンの lane_y を集める。
    let mut group_top: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for lane in &layout.lanes_ordered {
        let Some(group) = lane.group.as_deref() else {
            continue;
        };
        let center = layout.lane_y[&lane.id];
        let top = center - layout.opts.lane_height / 2.0;
        let entry = group_top.entry(group).or_insert(top);
        if top < *entry {
            *entry = top;
        }
    }
    if group_top.is_empty() {
        return;
    }

    // グループ先頭位置に区切り線とラベルを描画する。
    for (group_label, top_y) in &group_top {
        let label = escape_xml(group_label);
        if layout.is_vertical() {
            // 垂直レイアウト: グループ内の一番左のレーン列の上にラベルを出す。
            writeln!(
                s,
                r#"  <text class="tdsl-group-label" x="{x}" y="{y}" text-anchor="middle" font-weight="bold" font-size="11">{label}</text>"#,
                x = fmt_f(*top_y + layout.opts.lane_height / 2.0),
                y = fmt_f(layout.opts.top_margin - 20.0),
            )
            .unwrap();
        } else {
            // 水平レイアウト: バンドの上辺に区切り線とラベルを描画する。
            let x2 = layout.total_width - layout.opts.right_margin;
            writeln!(
                s,
                r##"  <line class="tdsl-group-separator" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="#aaa" stroke-width="1"/>  "##,
                x1 = fmt_f(0.0),
                y = fmt_f(*top_y),
                x2 = fmt_f(x2),
            )
            .unwrap();
            writeln!(
                s,
                r#"  <text class="tdsl-group-label" x="{x}" y="{y}" text-anchor="middle" font-weight="bold" font-size="11">{label}</text>"#,
                x = fmt_f(layout.opts.left_gutter / 2.0),
                y = fmt_f(top_y - 3.0),
            )
            .unwrap();
        }
    }
}

fn render_lane_labels(s: &mut String, layout: &LayoutModel) {
    if layout.is_vertical() {
        for lane in &layout.lanes_ordered {
            // In vertical layout, lane_y stores the center X of the lane column.
            let cx = layout.lane_y[&lane.id];
            writeln!(
                s,
                r#"  <text class="tdsl-lane-label" data-lane="{lane_id}" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                lane_id = escape_xml_attr(&lane.id),
                x = fmt_f(cx),
                y = fmt_f(layout.opts.top_margin - 8.0),
                label = escape_xml(&lane.label),
            )
            .unwrap();
        }
    } else {
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
}

fn render_items(s: &mut String, layout: &LayoutModel) {
    for laid in &layout.items {
        match laid {
            LaidItem::Span {
                item,
                x,
                y,
                width,
                height,
                color,
                tooltip,
            } => {
                let tip = escape_xml(tooltip);
                let tip_attr = escape_xml_attr(tooltip);
                let lane_id = item_lane_id(item);
                let fill_style = format!("fill:{color};");
                let data_attrs = if layout.opts.interactive {
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
                color,
                tooltip,
            } => {
                let tip = escape_xml(tooltip);
                let tip_attr = escape_xml_attr(tooltip);
                let lane_id = item_lane_id(item);
                let fill_style = format!("fill:{color};fill-opacity:0.75;");
                let data_attrs = if layout.opts.interactive {
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
                color,
                tooltip,
            } => {
                // An invisible wide hit-rect makes hovering the thin stem / small dot feasible.
                let tip = escape_xml(tooltip);
                let tip_attr = escape_xml_attr(tooltip);
                let lane_id = item_lane_id(item);
                let dot_style = format!("fill:{color};");
                let data_attrs = if layout.opts.interactive {
                    build_data_attrs(item, lane_id)
                } else {
                    String::new()
                };
                if layout.is_vertical() {
                    // Vertical layout: `x` = lane center X, `y_top`/`y_bottom`/`y_dot` = Y coords.
                    // Stem is horizontal (same Y, x varies from y_top to y_bottom — reusing field names).
                    let hit_x = *y_top;
                    let hit_y = *x - 8.0;
                    let hit_w = (y_bottom - y_top).max(20.0);
                    let hit_h = 16.0;
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-hit" x="{hx}" y="{hy}" width="{hw}" height="{hh}"><title>{tip}</title></rect><line class="tdsl-event-stem" x1="{x1}" y1="{cy}" x2="{x2}" y2="{cy}"><title>{tip}</title></line><circle class="tdsl-event-dot" style="{dot_style}" cx="{dot_x}" cy="{cy}" r="4"><title>{tip}</title></circle></g>"#,
                        tip = tip,
                        tip_attr = tip_attr,
                        dot_style = dot_style,
                        data_attrs = data_attrs,
                        hx = fmt_f(hit_x),
                        hy = fmt_f(hit_y),
                        hw = fmt_f(hit_w),
                        hh = fmt_f(hit_h),
                        x1 = fmt_f(*y_top),
                        x2 = fmt_f(*y_bottom),
                        cy = fmt_f(*x),
                        dot_x = fmt_f(*y_dot),
                    )
                    .unwrap();
                } else {
                    let hit_x = *x - 8.0;
                    let hit_w = 16.0;
                    let hit_y = *y_top;
                    let hit_h = (y_bottom - y_top).max(20.0);
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
}

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
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
    use crate::layout::{RenderOptions, format_date, format_year};
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
                group: None,
                source_span: None,
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
                group: None,
                source_span: None,
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
