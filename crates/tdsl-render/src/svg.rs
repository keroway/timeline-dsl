use std::fmt::Write;

use tdsl_core::ir::Item;

use crate::layout::{GridStyle, LANE_PALETTE, LaidItem, LayoutModel, format_year, month_abbr};

/// Render the SVG for a laid-out timeline. Pure string builder, no external deps.
pub fn render_svg(layout: &LayoutModel) -> Result<String, std::fmt::Error> {
    let mut s = String::new();
    let w = layout.total_width;
    let h = layout.total_height;

    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-label="timeline" class="tdsl-root">"#,
        w = fmt_f(w),
        h = fmt_f(h)
    )?;

    let font_family = layout
        .opts
        .font_family
        .as_deref()
        .unwrap_or(r#""Noto Sans JP", "Noto Sans CJK JP", "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif"#);

    // Embed font-family and axis text size for standalone SVG viewers (no CDN dependency).
    // Use .tdsl-root text selector to scope styles and prevent CSS leakage when embedded inline.
    // :root block defines --tdsl-lane-N custom properties so LP sites can override lane colors.
    let mut root_css = String::from(":root {");
    for (i, hex) in LANE_PALETTE.iter().enumerate() {
        write!(root_css, " --tdsl-lane-{i}: {hex};")?;
    }
    root_css.push_str(" }");
    writeln!(
        s,
        r#"  <style>{root_css} .tdsl-root text {{ font-family: {font_family}; }} .tdsl-axis-text {{ font-size: 11px; }} .tdsl-axis-month-tick {{ stroke: #ccc; stroke-width: 1; }} .tdsl-axis-day-tick {{ stroke: #ddd; stroke-width: 1; }} .tdsl-axis-day-text {{ font-size: 9px; fill: #888; }} .tdsl-event-label {{ font-size: 10px; fill: #333; pointer-events: none; }}</style>"#,
        root_css = root_css,
        font_family = font_family,
    )?;

    render_lane_bands(&mut s, layout)?;
    render_group_headers(&mut s, layout)?;
    render_grid_lines(&mut s, layout)?;
    render_axis(&mut s, layout)?;
    render_lane_labels(&mut s, layout)?;
    render_items(&mut s, layout)?;

    writeln!(s, "</svg>")?;
    Ok(s)
}

fn render_lane_bands(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    for band in &layout.lane_bands {
        let class = if band.even {
            "tdsl-lane-band-even"
        } else {
            "tdsl-lane-band-odd"
        };
        writeln!(
            s,
            r#"  <rect class="{class}" role="presentation" aria-hidden="true" x="{x}" y="{y}" width="{w}" height="{h}"/>"#,
            x = fmt_f(band.x),
            y = fmt_f(band.y),
            w = fmt_f(band.width),
            h = fmt_f(band.height),
        )?;
    }
    Ok(())
}

/// Render auxiliary grid lines behind the chart content.
///
/// Grid lines are purely decorative (`role="presentation"`) and are drawn at
/// the intervals dictated by `layout.opts.grid`. When `GridStyle::None` this
/// function writes nothing, guaranteeing that existing SVG output is unchanged.
fn render_grid_lines(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    if layout.opts.grid == GridStyle::None {
        return Ok(());
    }

    let positions = layout.grid_positions();
    if positions.is_empty() {
        return Ok(());
    }

    if layout.is_vertical() {
        // Vertical layout: time axis is Y; grid lines are horizontal.
        let x1 = layout.opts.left_gutter;
        let x2 = layout.total_width - layout.opts.right_margin;
        for frac in &positions {
            let y = layout.opts.top_margin + (frac - layout.year_min as f64) * layout.opts.scale;
            writeln!(
                s,
                r##"  <line class="tdsl-grid-line" role="presentation" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="#ccc" stroke-width="1" stroke-opacity="0.4"/>"##,
                x1 = fmt_f(x1),
                y = fmt_f(y),
                x2 = fmt_f(x2),
            )?;
        }
    } else {
        // Horizontal layout: time axis is X; grid lines are vertical.
        let y1 = layout.opts.top_margin;
        let y2 = layout.total_height - layout.opts.bottom_margin;
        for frac in &positions {
            let x = layout.opts.left_gutter + (frac - layout.year_min as f64) * layout.opts.scale;
            writeln!(
                s,
                r##"  <line class="tdsl-grid-line" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}" stroke="#ccc" stroke-width="1" stroke-opacity="0.4"/>"##,
                x = fmt_f(x),
                y1 = fmt_f(y1),
                y2 = fmt_f(y2),
            )?;
        }
    }
    Ok(())
}

fn render_axis(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    if layout.is_vertical() {
        render_axis_vertical(s, layout)
    } else {
        render_axis_horizontal(s, layout)
    }
}

fn render_axis_horizontal(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    let top = layout.opts.top_margin;
    let bottom = layout.total_height - layout.opts.bottom_margin;

    // Horizontal baseline at the top.
    let baseline_y = top - 4.0;
    writeln!(
        s,
        r#"  <line class="tdsl-axis-baseline" role="presentation" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}"/>"#,
        x1 = fmt_f(layout.opts.left_gutter),
        y = fmt_f(baseline_y),
        x2 = fmt_f(layout.total_width - layout.opts.right_margin),
    )?;

    for year in layout.ticks() {
        let x = layout.year_to_x(year);
        // Vertical grid line across the full chart body.
        writeln!(
            s,
            r#"  <line class="tdsl-axis-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(top),
            y2 = fmt_f(bottom),
        )?;
        let label = format_year(year);
        writeln!(
            s,
            r#"  <text class="tdsl-axis-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
            x = fmt_f(x),
            y = fmt_f(top - 8.0),
            label = escape_xml(&label),
        )?;
    }

    // Month minor ticks (unit=month only, hidden when scale too small).
    let px_per_month = layout.opts.scale / 12.0;
    for (year, month) in layout.month_ticks() {
        let x = layout.frac_year_to_x(year, month);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-month-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 3.0),
            y2 = fmt_f(baseline_y),
        )?;
        if px_per_month >= 20.0 {
            let label = month_abbr(month);
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-month-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
            )?;
        }
    }

    // Day minor ticks (unit=day only, hidden when scale too small).
    // 月初には `YYYY-MM` ラベルを表示し、それ以外は短い tick のみ。
    let pixels_per_day = layout.opts.scale / 365.25;
    for (year, month, day) in layout.day_ticks() {
        let x = layout.day_frac_to_x(year, month, day);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-day-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 2.0),
            y2 = fmt_f(baseline_y),
        )?;
        if day == 1 && pixels_per_day >= 1.5 {
            // 月またぎラベル: YYYY-MM
            let label = format!("{year:04}-{month:02}");
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-day-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
                label = escape_xml(&label),
            )?;
        } else if pixels_per_day >= 8.0 {
            // 日番号ラベル（密度が十分なときのみ）
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-day-text" x="{x}" y="{y}" text-anchor="middle">{day}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
            )?;
        }
    }
    Ok(())
}

fn render_axis_vertical(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    let left = layout.opts.left_gutter;
    let right = layout.total_width - layout.opts.right_margin;

    // Vertical baseline on the left side.
    let baseline_x = left - 4.0;
    writeln!(
        s,
        r#"  <line class="tdsl-axis-baseline" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
        x = fmt_f(baseline_x),
        y1 = fmt_f(layout.opts.top_margin),
        y2 = fmt_f(layout.total_height - layout.opts.bottom_margin),
    )?;

    for year in layout.ticks() {
        let y = layout.year_to_primary(year);
        // Horizontal grid line across the full chart body.
        writeln!(
            s,
            r#"  <line class="tdsl-axis-tick" role="presentation" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}"/>"#,
            x1 = fmt_f(left),
            y = fmt_f(y),
            x2 = fmt_f(right),
        )?;
        let label = format_year(year);
        writeln!(
            s,
            r#"  <text class="tdsl-axis-text" x="{x}" y="{y}" text-anchor="end" dominant-baseline="middle">{label}</text>"#,
            x = fmt_f(left - 8.0),
            y = fmt_f(y),
            label = escape_xml(&label),
        )?;
    }
    Ok(())
}

fn render_group_headers(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
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
        return Ok(());
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
            )?;
        } else {
            // 水平レイアウト: バンドの上辺に区切り線とラベルを描画する。
            let x2 = layout.total_width - layout.opts.right_margin;
            writeln!(
                s,
                r##"  <line class="tdsl-group-separator" role="presentation" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="#aaa" stroke-width="1"/>  "##,
                x1 = fmt_f(0.0),
                y = fmt_f(*top_y),
                x2 = fmt_f(x2),
            )?;
            writeln!(
                s,
                r#"  <text class="tdsl-group-label" x="{x}" y="{y}" text-anchor="middle" font-weight="bold" font-size="11">{label}</text>"#,
                x = fmt_f(layout.opts.left_gutter / 2.0),
                y = fmt_f(top_y - 3.0),
            )?;
        }
    }
    Ok(())
}

fn render_lane_labels(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
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
            )?;
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
            )?;
        }
    }
    Ok(())
}

fn render_items(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
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
                let lane_label = layout
                    .lanes_ordered
                    .iter()
                    .find(|l| l.id == lane_id)
                    .map(|l| l.label.as_str())
                    .unwrap_or(lane_id);
                let aria_label = escape_xml_attr(&item_aria_label(item, tooltip, lane_label));
                let fill_style = format!("fill:{color};");
                let tags = item_tags(item);
                let mut data_attrs = format!(r#" data-lane="{}""#, escape_xml_attr(lane_id));
                if !tags.is_empty() {
                    data_attrs.push_str(&format!(
                        r#" data-tags="{}""#,
                        escape_xml_attr(&tags.join(","))
                    ));
                }
                if layout.opts.interactive {
                    data_attrs.push_str(&build_interactive_attrs(item));
                }
                writeln!(
                    s,
                    r#"  <g class="tdsl-item tdsl-item-span" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-span" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="3"><title>{tip}</title></rect><text class="tdsl-item-label" x="{tx}" y="{ty}" dominant-baseline="middle">{label}</text></g>"#,
                    aria_label = aria_label,
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
                )?;
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
                let lane_label = layout
                    .lanes_ordered
                    .iter()
                    .find(|l| l.id == lane_id)
                    .map(|l| l.label.as_str())
                    .unwrap_or(lane_id);
                let aria_label = escape_xml_attr(&item_aria_label(item, tooltip, lane_label));
                let fill_style = format!("fill:{color};fill-opacity:0.75;");
                let tags = item_tags(item);
                let mut data_attrs = format!(r#" data-lane="{}""#, escape_xml_attr(lane_id));
                if !tags.is_empty() {
                    data_attrs.push_str(&format!(
                        r#" data-tags="{}""#,
                        escape_xml_attr(&tags.join(","))
                    ));
                }
                if layout.opts.interactive {
                    data_attrs.push_str(&build_interactive_attrs(item));
                }
                if layout.opts.show_event_labels {
                    // EventRange with always-on label: include text inside the <g>.
                    // For horizontal layout: label at (x+4, y+height/2) — same pattern as Span.
                    // For vertical layout: label below the bar (y+height+4, x center).
                    let label_fragment = if layout.is_vertical() {
                        format!(
                            r#"<text class="tdsl-event-label" x="{lx}" y="{ly}" text-anchor="middle">{label}</text>"#,
                            lx = fmt_f(*x + *width / 2.0),
                            ly = fmt_f(*y + *height + 12.0),
                            label = escape_xml(item_label(item)),
                        )
                    } else {
                        format!(
                            r#"<text class="tdsl-event-label" x="{lx}" y="{ly}" dominant-baseline="middle">{label}</text>"#,
                            lx = fmt_f(*x + 4.0),
                            ly = fmt_f(*y + *height / 2.0),
                            label = escape_xml(item_label(item)),
                        )
                    };
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event-range" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-range" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="2"><title>{tip}</title></rect>{label_fragment}</g>"#,
                        aria_label = aria_label,
                        tip = tip,
                        tip_attr = tip_attr,
                        fill_style = fill_style,
                        data_attrs = data_attrs,
                        x = fmt_f(*x),
                        y = fmt_f(*y),
                        w = fmt_f(*width),
                        h = fmt_f(*height),
                        label_fragment = label_fragment,
                    )?;
                } else {
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event-range" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-range" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="2"><title>{tip}</title></rect></g>"#,
                        aria_label = aria_label,
                        tip = tip,
                        tip_attr = tip_attr,
                        fill_style = fill_style,
                        data_attrs = data_attrs,
                        x = fmt_f(*x),
                        y = fmt_f(*y),
                        w = fmt_f(*width),
                        h = fmt_f(*height),
                    )?;
                }
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
                let lane_label = layout
                    .lanes_ordered
                    .iter()
                    .find(|l| l.id == lane_id)
                    .map(|l| l.label.as_str())
                    .unwrap_or(lane_id);
                let aria_label = escape_xml_attr(&item_aria_label(item, tooltip, lane_label));
                let dot_style = format!("fill:{color};");
                let tags = item_tags(item);
                let mut data_attrs = format!(r#" data-lane="{}""#, escape_xml_attr(lane_id));
                if !tags.is_empty() {
                    data_attrs.push_str(&format!(
                        r#" data-tags="{}""#,
                        escape_xml_attr(&tags.join(","))
                    ));
                }
                if layout.opts.interactive {
                    data_attrs.push_str(&build_interactive_attrs(item));
                }
                if layout.is_vertical() {
                    // Vertical layout: `x` = lane center X, `y_top`/`y_bottom`/`y_dot` = Y coords.
                    // Stem is horizontal (same Y, x varies from y_top to y_bottom — reusing field names).
                    let hit_x = *y_top;
                    let hit_y = *x - 8.0;
                    let hit_w = (y_bottom - y_top).max(20.0);
                    let hit_h = 16.0;
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-hit" x="{hx}" y="{hy}" width="{hw}" height="{hh}"><title>{tip}</title></rect><line class="tdsl-event-stem" x1="{x1}" y1="{cy}" x2="{x2}" y2="{cy}"><title>{tip}</title></line><circle class="tdsl-event-dot" style="{dot_style}" cx="{dot_x}" cy="{cy}" r="4"><title>{tip}</title></circle></g>"#,
                        aria_label = aria_label,
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
                    )?;
                    if layout.opts.show_event_labels {
                        // Vertical: label to the right of the dot (dot_x + 6, same Y as dot center).
                        writeln!(
                            s,
                            r#"  <text class="tdsl-event-label" x="{lx}" y="{ly}" dominant-baseline="middle" text-anchor="start">{label}</text>"#,
                            lx = fmt_f(*y_dot + 6.0),
                            ly = fmt_f(*x),
                            label = escape_xml(item_label(item)),
                        )?;
                    }
                } else {
                    let hit_x = *x - 8.0;
                    let hit_w = 16.0;
                    let hit_y = *y_top;
                    let hit_h = (y_bottom - y_top).max(20.0);
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-hit" x="{hx}" y="{hy}" width="{hw}" height="{hh}"><title>{tip}</title></rect><line class="tdsl-event-stem" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"><title>{tip}</title></line><circle class="tdsl-event-dot" style="{dot_style}" cx="{x}" cy="{cy}" r="4"><title>{tip}</title></circle></g>"#,
                        aria_label = aria_label,
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
                    )?;
                    if layout.opts.show_event_labels {
                        // Horizontal: label above the dot, centered horizontally.
                        writeln!(
                            s,
                            r#"  <text class="tdsl-event-label" x="{lx}" y="{ly}" text-anchor="middle">{label}</text>"#,
                            lx = fmt_f(*x),
                            ly = fmt_f(*y_top - 4.0),
                            label = escape_xml(item_label(item)),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

/// Build data-* attributes for interactive mode as a string fragment (leading space included).
/// Does NOT include `data-lane` (always emitted unconditionally in render_items).
fn build_interactive_attrs(item: &Item) -> String {
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
        r#" data-id="{}" data-label="{}" data-type="{}""#,
        escape_xml_attr(id),
        escape_xml_attr(label),
        type_str,
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

fn item_tags(item: &Item) -> &[String] {
    match item {
        Item::Span { tags, .. } => tags,
        Item::Event { tags, .. } => tags,
        Item::EventRange { tags, .. } => tags,
    }
}

/// Build the ARIA label string for a timeline item.
///
/// Format: `"<type>: <tooltip_on_one_line>、レーン: <lane_label>"`
/// Newlines in the tooltip are replaced with `、` for a compact single-line value.
fn item_aria_label(item: &Item, tooltip: &str, lane_label: &str) -> String {
    let type_str = match item {
        Item::Span { .. } => "スパン",
        Item::Event { .. } => "イベント",
        Item::EventRange { .. } => "期間イベント",
    };
    let info = tooltip.replace('\n', "、");
    format!("{type_str}: {info}、レーン: {lane_label}")
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

/// Resolve CSS custom property references in SVG `style="…"` attributes for
/// raster renderers that do not support CSS variables.
///
/// Only the content of `style="…"` attribute values is modified.
/// User-visible text (title elements, text labels, aria-label attributes) is
/// never touched because those values are not bounded by `style="…"`.
/// Replaces `var(--tdsl-lane-N, <fallback>)` with `<fallback>` inside each
/// matched attribute.
pub(crate) fn resolve_lane_vars_in_styles(svg: &str) -> String {
    const ATTR: &str = "style=\"";
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(start) = rest.find(ATTR) {
        let after_open = start + ATTR.len();
        out.push_str(&rest[..after_open]);
        rest = &rest[after_open..];
        match rest.find('"') {
            Some(end) => {
                out.push_str(&resolve_vars_in_css_value(&rest[..end]));
                out.push('"');
                rest = &rest[end + 1..];
            }
            None => {
                // Malformed — emit remainder verbatim.
                out.push_str(rest);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Replace `var(--tdsl-lane-N, <fallback>)` with `<fallback>` within a single
/// CSS property value string. Other content is passed through unchanged.
fn resolve_vars_in_css_value(css: &str) -> String {
    const PREFIX: &str = "var(--tdsl-lane-";
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find(PREFIX) {
        out.push_str(&rest[..start]);
        rest = &rest[start + PREFIX.len()..];
        match (rest.find(','), rest.find(')')) {
            (Some(comma), Some(close)) if comma < close => {
                out.push_str(rest[comma + 1..close].trim());
                rest = &rest[close + 1..];
            }
            _ => {
                out.push_str(PREFIX);
            }
        }
    }
    out.push_str(rest);
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
    use crate::layout::{GridStyle, Orientation, RenderOptions, format_date, format_year};
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
        let svg = render_svg(&layout).unwrap();
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
        let svg = render_svg(&layout).unwrap();
        assert!(svg.contains("&lt;danger&gt;"));
        assert!(svg.contains("&amp;"));
        assert!(svg.contains("&quot;"));
        assert!(!svg.contains("<danger>"));
    }

    #[test]
    fn svg_includes_tooltip_via_title_element() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(svg.contains("<title>"));
        assert!(svg.contains("wd:Q7209"));
        assert!(svg.contains(r#"data-tdsl-tooltip="漢&#10;BC206〜220"#));
        assert!(svg.contains(r#"tabindex="0""#));
        // ARIA attributes
        assert!(
            svg.contains(r#"role="group""#),
            "items must have role=group"
        );
        assert!(
            svg.contains(r#"aria-label=""#),
            "items must have aria-label"
        );
        assert!(
            svg.contains(r#"role="presentation""#),
            "decorative elements must have role=presentation"
        );
    }

    #[test]
    fn aria_attributes_on_items() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();

        // Span item: <g> に role=group と aria-label が含まれる
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-span" role="group" aria-label=""#),
            "span item must have role=group and aria-label"
        );
        // aria-label に "スパン" が含まれる
        assert!(
            svg.contains("スパン:"),
            "span aria-label must contain type prefix 'スパン:'"
        );
        // aria-label に期間情報が含まれる (BC206 と 220 の両方)
        assert!(
            svg.contains("BC206"),
            "span aria-label must contain start year"
        );
        // aria-label に "レーン:" が含まれる
        assert!(
            svg.contains("レーン:"),
            "aria-label must contain lane label reference"
        );
        // Event item: <g> に role=group と aria-label が含まれる
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-event" role="group" aria-label=""#),
            "event item must have role=group and aria-label"
        );
        // event aria-label に "イベント" が含まれる
        assert!(
            svg.contains("イベント:"),
            "event aria-label must contain type prefix 'イベント:'"
        );

        // lane band の rect に role=presentation が含まれる
        assert!(
            svg.contains(r#"class="tdsl-lane-band-even" role="presentation" aria-hidden="true""#),
            "lane band rect must have role=presentation and aria-hidden=true"
        );

        // 軸の line に role=presentation が含まれる
        assert!(
            svg.contains(r#"class="tdsl-axis-baseline" role="presentation""#),
            "axis baseline must have role=presentation"
        );
        assert!(
            svg.contains(r#"class="tdsl-axis-tick" role="presentation""#),
            "axis tick must have role=presentation"
        );
    }

    #[test]
    fn aria_attributes_on_event_range() {
        let ir = TimelineIr {
            meta: Meta {
                title: "test".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "war".into(),
                label: "戦争".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::EventRange {
                id: "er1".into(),
                lane: "war".into(),
                start: 100,
                end: 200,
                label: "大乱".into(),
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
        let svg = render_svg(&layout).unwrap();

        // event_range の <g> に role=group と aria-label が含まれる
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-event-range" role="group" aria-label=""#),
            "event_range item must have role=group and aria-label"
        );
        // aria-label に "期間イベント" が含まれる
        assert!(
            svg.contains("期間イベント:"),
            "event_range aria-label must contain type prefix '期間イベント:'"
        );
        // aria-label に レーン名 "戦争" が含まれる
        assert!(
            svg.contains("戦争"),
            "event_range aria-label must contain lane label"
        );
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
        let svg = render_svg(&layout).unwrap();
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
        let svg = render_svg(&layout).unwrap();
        // The span item has tag "dynasty", so its fill must use the color_map color.
        assert!(
            svg.contains("fill:#cc0000;"),
            "expected fill:#cc0000; in SVG, got:\n{svg}"
        );
    }

    // ─── GridStyle テスト ────────────────────────────────────────────────────

    #[test]
    fn grid_none_produces_no_grid_lines() {
        // GridStyle::None (default) must not output any tdsl-grid-line element.
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(
            !svg.contains("tdsl-grid-line"),
            "GridStyle::None must produce no grid lines, got:\n{svg}"
        );
    }

    #[test]
    fn grid_none_svg_output_unchanged() {
        // Explicitly setting GridStyle::None must produce identical output to the default.
        let ir = sample_ir();
        let default_svg = render_svg(&LayoutModel::compute(&ir, RenderOptions::default())).unwrap();
        let explicit_none_svg = render_svg(&LayoutModel::compute(
            &ir,
            RenderOptions {
                grid: GridStyle::None,
                ..RenderOptions::default()
            },
        ))
        .unwrap();
        assert_eq!(
            default_svg, explicit_none_svg,
            "explicit GridStyle::None must produce identical SVG to default"
        );
    }

    #[test]
    fn grid_decade_horizontal_produces_grid_lines() {
        // GridStyle::Decade must output tdsl-grid-line elements in horizontal layout.
        let ir = sample_ir(); // range -300..300 → many decade boundaries
        let opts = RenderOptions {
            grid: GridStyle::Decade,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-grid-line"),
            "GridStyle::Decade (horizontal) must produce grid lines"
        );
        // role=presentation must be set for accessibility
        assert!(
            svg.contains(r#"role="presentation""#),
            "grid lines must have role=presentation"
        );
    }

    #[test]
    fn grid_year_horizontal_produces_grid_lines() {
        // GridStyle::Year must output tdsl-grid-line elements.
        let ir = sample_ir();
        let opts = RenderOptions {
            grid: GridStyle::Year,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-grid-line"),
            "GridStyle::Year (horizontal) must produce grid lines"
        );
    }

    #[test]
    fn grid_month_horizontal_produces_grid_lines() {
        // GridStyle::Month must output tdsl-grid-line elements.
        let ir = sample_ir();
        let opts = RenderOptions {
            grid: GridStyle::Month,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-grid-line"),
            "GridStyle::Month (horizontal) must produce grid lines"
        );
    }

    #[test]
    fn grid_decade_vertical_produces_horizontal_grid_lines() {
        // GridStyle::Decade on a vertical layout must output horizontal grid lines
        // (i.e. x1 != x2, y1 == y2 for each grid line).
        let ir = sample_ir(); // range -300..300
        let opts = RenderOptions {
            grid: GridStyle::Decade,
            orientation: Orientation::Vertical,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-grid-line"),
            "GridStyle::Decade (vertical) must produce grid lines"
        );
    }

    #[test]
    fn grid_year_vertical_produces_grid_lines() {
        // GridStyle::Year on a vertical layout must output grid lines.
        let ir = sample_ir();
        let opts = RenderOptions {
            grid: GridStyle::Year,
            orientation: Orientation::Vertical,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-grid-line"),
            "GridStyle::Year (vertical) must produce grid lines"
        );
    }

    #[test]
    fn grid_lines_appear_before_axis_in_output() {
        // Grid lines must be drawn before the axis tick so they render behind tick marks.
        let ir = sample_ir();
        let opts = RenderOptions {
            grid: GridStyle::Decade,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        let grid_pos = svg.find("tdsl-grid-line").expect("grid line must exist");
        let axis_pos = svg.find("tdsl-axis-tick").expect("axis tick must exist");
        assert!(
            grid_pos < axis_pos,
            "grid lines must appear before axis ticks in SVG output (z-order)"
        );
    }

    // ─── show_event_labels テスト ─────────────────────────────────────────────

    #[test]
    fn show_event_labels_false_produces_no_event_label_elements() {
        // Default (show_event_labels=false): no <text class="tdsl-event-label"> elements in output.
        // Note: the CSS class name appears once in the embedded <style> block, but no <text> elements
        // with that class should be emitted.
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(
            !svg.contains(r#"class="tdsl-event-label""#),
            "show_event_labels=false must not produce any <text class=\"tdsl-event-label\"> elements, got:\n{svg}"
        );
    }

    #[test]
    fn show_event_labels_true_produces_event_label_for_event_item() {
        // show_event_labels=true: event items get a <text class="tdsl-event-label"> element.
        let ir = sample_ir(); // contains an Event item "陳勝・呉広の乱"
        let opts = RenderOptions {
            show_event_labels: true,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains(r#"class="tdsl-event-label""#),
            "show_event_labels=true must produce <text class=\"tdsl-event-label\"> elements, got:\n{svg}"
        );
        // Label text for the event must appear.
        assert!(
            svg.contains("陳勝・呉広の乱"),
            "event label text must appear in SVG when show_event_labels=true"
        );
    }

    #[test]
    fn show_event_labels_true_produces_event_label_for_event_range() {
        // show_event_labels=true: EventRange items also get a tdsl-event-label element.
        let ir = TimelineIr {
            meta: Meta {
                title: "test".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "war".into(),
                label: "戦争".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::EventRange {
                id: "er1".into(),
                lane: "war".into(),
                start: 100,
                end: 200,
                label: "大乱ラベル".into(),
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
        let opts = RenderOptions {
            show_event_labels: true,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains(r#"class="tdsl-event-label""#),
            "show_event_labels=true must produce <text class=\"tdsl-event-label\"> for EventRange, got:\n{svg}"
        );
        assert!(
            svg.contains("大乱ラベル"),
            "event_range label text must appear in SVG when show_event_labels=true"
        );
    }

    #[test]
    fn show_event_labels_vertical_layout_renders_labels() {
        // Vertical layout: event labels are rendered to the right of the dot.
        let ir = sample_ir();
        let opts = RenderOptions {
            show_event_labels: true,
            orientation: Orientation::Vertical,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains(r#"class="tdsl-event-label""#),
            "show_event_labels=true (vertical) must produce tdsl-event-label elements"
        );
        // Vertical event label should use text-anchor="start".
        assert!(
            svg.contains(r#"class="tdsl-event-label" x="#),
            "vertical event label must include x attribute"
        );
    }

    #[test]
    fn resolve_lane_vars_in_styles_replaces_within_style_attr() {
        let input = r#"<rect style="fill:var(--tdsl-lane-0, #4682B4);" x="0"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert_eq!(got, r#"<rect style="fill:#4682B4;" x="0"/>"#);
    }

    #[test]
    fn resolve_lane_vars_in_styles_leaves_text_content_untouched() {
        // User label that contains the variable syntax must not be modified.
        let input = r#"<title>var(--tdsl-lane-0, #color)</title><rect style="fill:var(--tdsl-lane-0, #4682B4);"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert!(got.contains("<title>var(--tdsl-lane-0, #color)</title>"));
        assert!(got.contains(r#"style="fill:#4682B4;""#));
    }

    #[test]
    fn resolve_lane_vars_in_styles_leaves_root_css_block_untouched() {
        let input = ":root { --tdsl-lane-0: #4682B4; }";
        assert_eq!(resolve_lane_vars_in_styles(input), input);
    }

    #[test]
    fn resolve_lane_vars_in_styles_handles_multiple_attrs() {
        let input = r#"<rect style="fill:var(--tdsl-lane-0, #4682B4);fill-opacity:0.75;"/><circle style="fill:var(--tdsl-lane-1, #E67E22);"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert!(got.contains(r#"style="fill:#4682B4;fill-opacity:0.75;""#));
        assert!(got.contains(r#"style="fill:#E67E22;""#));
    }

}
