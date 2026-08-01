use std::fmt::Write;

use tdsl_core::ir::Item;

use crate::layout::{
    EVENT_LABEL_STACK_STEP, GridStyle, LANE_PALETTE, LEGEND_ROW_HEIGHT, LaidItem, LayoutModel,
    LayoutStyle, TABLE_COL_LABEL, TABLE_COL_LANE, TABLE_COL_TAGS, TABLE_COL_TIME, TABLE_ROW_HEIGHT,
    estimate_text_width_px, format_year, label_available_width_px, laid_item_label, month_abbr,
};

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
    // #701: define --tdsl-lane-N custom properties scoped to the <svg> element itself via
    // :where(.tdsl-root), not :root — when the SVG is inserted as inline DOM (e.g. via
    // adoptNode rather than <img>), a :root selector inside its <style> matches the *host*
    // document's root element and leaks these custom properties into the embedding page.
    // :where() keeps the selector's specificity at zero so a host page's own
    // `.tdsl-root { --tdsl-lane-N: ... }` override (e.g. timeline-dsl-lp's semantic-token
    // bridge, DESIGN.md) still wins deterministically regardless of stylesheet source order —
    // a plain `.tdsl-root` selector here would tie on specificity and could lose to an
    // earlier-loaded host stylesheet purely by DOM position.
    let mut root_css = String::from(":where(.tdsl-root) {");
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

    render_group_bands(&mut s, layout)?;
    render_lane_bands(&mut s, layout)?;
    render_group_headers(&mut s, layout)?;
    render_grid_lines(&mut s, layout)?;
    render_axis(&mut s, layout)?;
    render_lane_labels(&mut s, layout)?;
    render_items(&mut s, layout)?;
    if layout.opts.show_legend {
        render_static_legend(&mut s, layout)?;
    }
    if layout.opts.show_table {
        render_table(&mut s, layout)?;
    }

    writeln!(s, "</svg>")?;
    Ok(s)
}

/// Render a static legend panel (#544) below the timeline body for SVG/PNG/PDF output.
fn render_static_legend(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    let left = 8.0;
    let content_width = (layout.total_width - left * 2.0).max(0.0);
    let top = layout.legend_top_y;
    let height = layout.legend_row_count as f64 * LEGEND_ROW_HEIGHT;

    writeln!(
        s,
        r##"  <g class="tdsl-static-legend" role="group" aria-label="legend">"##,
    )?;
    writeln!(
        s,
        r##"    <rect class="tdsl-static-legend-bg" x="{x}" y="{y}" width="{w}" height="{h}" fill="#fff" stroke="#d0d7de"/>"##,
        x = fmt_f(left),
        y = fmt_f(top),
        w = fmt_f(content_width),
        h = fmt_f(height),
    )?;
    writeln!(
        s,
        r#"    <text class="tdsl-static-legend-title" x="{x}" y="{y}" dominant-baseline="middle" font-weight="bold" font-size="12">凡例</text>"#,
        x = fmt_f(left + 8.0),
        y = fmt_f(top + LEGEND_ROW_HEIGHT / 2.0),
    )?;

    let mut row = 1usize;
    for lane in &layout.lanes_ordered {
        let y = top + (row as f64 + 0.5) * LEGEND_ROW_HEIGHT;
        let color = layout
            .lane_colors
            .get(&lane.id)
            .map(String::as_str)
            .unwrap_or("#4682B4");
        writeln!(
            s,
            r#"    <rect class="tdsl-static-legend-swatch" x="{x}" y="{y}" width="12" height="12" rx="2" fill="{fill}"/>"#,
            x = fmt_f(left + 8.0),
            y = fmt_f(y - 6.0),
            fill = escape_xml_attr(color),
        )?;
        writeln!(
            s,
            r#"    <text class="tdsl-static-legend-item" x="{x}" y="{y}" dominant-baseline="middle" font-size="11">レーン: {label}</text>"#,
            x = fmt_f(left + 28.0),
            y = fmt_f(y),
            label = escape_xml(&lane.label),
        )?;
        row += 1;
    }

    let mut tag_colors: Vec<_> = layout.opts.color_map.iter().collect();
    tag_colors.sort_by(|a, b| a.0.cmp(b.0));
    for (tag, color) in tag_colors {
        let y = top + (row as f64 + 0.5) * LEGEND_ROW_HEIGHT;
        writeln!(
            s,
            r#"    <rect class="tdsl-static-legend-swatch" x="{x}" y="{y}" width="12" height="12" rx="2" fill="{fill}"/>"#,
            x = fmt_f(left + 8.0),
            y = fmt_f(y - 6.0),
            fill = escape_xml_attr(color),
        )?;
        writeln!(
            s,
            r#"    <text class="tdsl-static-legend-item" x="{x}" y="{y}" dominant-baseline="middle" font-size="11">タグ: {label}</text>"#,
            x = fmt_f(left + 28.0),
            y = fmt_f(y),
            label = escape_xml(tag),
        )?;
        row += 1;
    }

    writeln!(s, "  </g>")?;
    Ok(())
}

/// Render the "all items" table (#536) as SVG `<rect>`/`<text>` elements below the
/// timeline body. Used for SVG/PNG/PDF output when `RenderOptions.show_table` is
/// true (HTML output instead uses a native `<table>` element; see `html.rs`).
///
/// Columns: 時期 (time period) / ラベル (label) / レーン (lane) / タグ (tags), matching
/// the HTML table exactly. `LayoutModel::compute` has already reserved enough
/// vertical space (`total_height`) for the header row plus one row per item.
fn render_table(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    let left = 8.0;
    let content_width = (layout.total_width - left * 2.0).max(0.0);
    // Proportional column widths: time / label / lane / tags.
    let col_widths = [
        content_width * 0.20,
        content_width * 0.40,
        content_width * 0.15,
        content_width * 0.25,
    ];
    let col_x = [
        left,
        left + col_widths[0],
        left + col_widths[0] + col_widths[1],
        left + col_widths[0] + col_widths[1] + col_widths[2],
    ];

    writeln!(
        s,
        r#"  <g class="tdsl-table" role="table" aria-label="item list">"#,
    )?;

    // Header row background + labels.
    let header_y = layout.table_top_y;
    writeln!(
        s,
        r##"    <rect class="tdsl-table-header-bg" x="{x}" y="{y}" width="{w}" height="{h}" fill="#e8e8e8"></rect>"##,
        x = fmt_f(left),
        y = fmt_f(header_y),
        w = fmt_f(content_width),
        h = fmt_f(TABLE_ROW_HEIGHT),
    )?;
    for (i, col) in [
        TABLE_COL_TIME,
        TABLE_COL_LABEL,
        TABLE_COL_LANE,
        TABLE_COL_TAGS,
    ]
    .iter()
    .enumerate()
    {
        writeln!(
            s,
            r#"    <text class="tdsl-table-header" x="{x}" y="{y}" dominant-baseline="middle" font-weight="bold" font-size="11">{label}</text>"#,
            x = fmt_f(col_x[i] + 4.0),
            y = fmt_f(header_y + TABLE_ROW_HEIGHT / 2.0),
            label = escape_xml(col),
        )?;
    }

    for (row_idx, row) in layout.table_rows.iter().enumerate() {
        let row_y = header_y + (row_idx as f64 + 1.0) * TABLE_ROW_HEIGHT;
        if row_idx % 2 == 1 {
            writeln!(
                s,
                r##"    <rect class="tdsl-table-row-alt" x="{x}" y="{y}" width="{w}" height="{h}" fill="#f5f5f5"></rect>"##,
                x = fmt_f(left),
                y = fmt_f(row_y),
                w = fmt_f(content_width),
                h = fmt_f(TABLE_ROW_HEIGHT),
            )?;
        }
        let cells = [&row.time_str, &row.label, &row.lane_label, &row.tags];
        for (i, cell) in cells.iter().enumerate() {
            let available = (col_widths[i] - 8.0).max(0.0);
            let text = truncate_with_ellipsis(cell, 11.0, available);
            writeln!(
                s,
                r#"    <text class="tdsl-table-cell" x="{x}" y="{y}" dominant-baseline="middle" font-size="11">{label}</text>"#,
                x = fmt_f(col_x[i] + 4.0),
                y = fmt_f(row_y + TABLE_ROW_HEIGHT / 2.0),
                label = escape_xml(&text),
            )?;
        }
    }

    writeln!(s, "  </g>")?;
    Ok(())
}

/// Render one table-only SVG page for paginated PDF output.
///
/// The dimensions are expressed in PDF points so [`crate::pdf`] can place this
/// SVG into the printable area without scaling. `rows` must contain only whole
/// table rows; callers determine pagination boundaries before invoking this
/// function.
///
/// `page_number`/`total_pages` (1-based, ADR-0004 D4) render a `"i / N"` footer
/// centred at the bottom of the printable area on every table page, so the
/// reader can tell their position even after the pages are printed/reordered.
pub(crate) fn render_table_page_svg(
    rows: &[crate::layout::TableRow],
    width: f32,
    height: f32,
    page_number: usize,
    total_pages: usize,
) -> Result<String, std::fmt::Error> {
    let mut s = String::new();
    let width = f64::from(width);
    let height = f64::from(height);
    let left = 8.0;
    let content_width = (width - left * 2.0).max(0.0);
    let col_widths = [
        content_width * 0.20,
        content_width * 0.40,
        content_width * 0.15,
        content_width * 0.25,
    ];
    let col_x = [
        left,
        left + col_widths[0],
        left + col_widths[0] + col_widths[1],
        left + col_widths[0] + col_widths[1] + col_widths[2],
    ];

    writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" width="{width}" height="{height}">"#,
        width = fmt_f(width),
        height = fmt_f(height),
    )?;
    writeln!(
        s,
        r#"  <style>text {{ font-family: "Noto Sans JP", "Noto Sans CJK JP", "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", sans-serif; }}</style>"#,
    )?;
    writeln!(
        s,
        r#"  <g class="tdsl-table" role="table" aria-label="item list">"#
    )?;
    writeln!(
        s,
        r##"    <rect class="tdsl-table-header-bg" x="{x}" y="0" width="{w}" height="{h}" fill="#e8e8e8"></rect>"##,
        x = fmt_f(left),
        w = fmt_f(content_width),
        h = fmt_f(TABLE_ROW_HEIGHT),
    )?;
    for (i, col) in [
        TABLE_COL_TIME,
        TABLE_COL_LABEL,
        TABLE_COL_LANE,
        TABLE_COL_TAGS,
    ]
    .iter()
    .enumerate()
    {
        writeln!(
            s,
            r#"    <text class="tdsl-table-header" x="{x}" y="{y}" dominant-baseline="middle" font-weight="bold" font-size="11">{label}</text>"#,
            x = fmt_f(col_x[i] + 4.0),
            y = fmt_f(TABLE_ROW_HEIGHT / 2.0),
            label = escape_xml(col),
        )?;
    }
    for (row_idx, row) in rows.iter().enumerate() {
        let row_y = (row_idx as f64 + 1.0) * TABLE_ROW_HEIGHT;
        if row_idx % 2 == 1 {
            writeln!(
                s,
                r##"    <rect class="tdsl-table-row-alt" x="{x}" y="{y}" width="{w}" height="{h}" fill="#f5f5f5"></rect>"##,
                x = fmt_f(left),
                y = fmt_f(row_y),
                w = fmt_f(content_width),
                h = fmt_f(TABLE_ROW_HEIGHT),
            )?;
        }
        let cells = [&row.time_str, &row.label, &row.lane_label, &row.tags];
        for (i, cell) in cells.iter().enumerate() {
            let text = truncate_with_ellipsis(cell, 11.0, (col_widths[i] - 8.0).max(0.0));
            writeln!(
                s,
                r#"    <text class="tdsl-table-cell" x="{x}" y="{y}" dominant-baseline="middle" font-size="11">{label}</text>"#,
                x = fmt_f(col_x[i] + 4.0),
                y = fmt_f(row_y + TABLE_ROW_HEIGHT / 2.0),
                label = escape_xml(&text),
            )?;
        }
    }
    writeln!(s, "  </g>")?;

    // ADR-0004 D4: repeat a "i / N" page-number footer on every table page,
    // centred near the bottom of the printable area.
    writeln!(
        s,
        r#"  <text class="tdsl-table-page-footer" x="{x}" y="{y}" text-anchor="middle" font-size="9">{label}</text>"#,
        x = fmt_f(width / 2.0),
        y = fmt_f(height - 6.0),
        label = escape_xml(&format!("{page_number} / {total_pages}")),
    )?;

    writeln!(s, "</svg>")?;
    Ok(s)
}

/// Render background bands (#543) spanning contiguous lane groups/eras.
/// Purely decorative (`role="presentation"`); empty (no-op) unless
/// `RenderOptions.layout_style == LayoutStyle::GroupBands`.
fn render_group_bands(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    for band in &layout.group_bands {
        let class = if band.even {
            "tdsl-group-band-even"
        } else {
            "tdsl-group-band-odd"
        };
        writeln!(
            s,
            r#"  <rect class="{class}" role="presentation" aria-hidden="true" data-group="{label}" x="{x}" y="{y}" width="{w}" height="{h}"/>"#,
            class = class,
            label = escape_xml_attr(&band.label),
            x = fmt_f(band.x),
            y = fmt_f(band.y),
            w = fmt_f(band.width),
            h = fmt_f(band.height),
        )?;
    }
    Ok(())
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
/// the intervals dictated by `layout.effective_grid_style()`. When that
/// resolves to `GridStyle::None` this function writes nothing, guaranteeing
/// that existing SVG output is unchanged.
///
/// #564: `LayoutStyle::Gantt` uses a heavier `tdsl-grid-gantt` CSS class
/// (darker/thicker stroke than the standard `tdsl-grid-line`) instead of the
/// default styling, whether the effective grid came from an explicit `--grid`
/// choice or the Gantt-forced month grid.
fn render_grid_lines(s: &mut String, layout: &LayoutModel) -> std::fmt::Result {
    if layout.effective_grid_style() == GridStyle::None {
        return Ok(());
    }

    let positions = layout.grid_positions();
    if positions.is_empty() {
        return Ok(());
    }

    let is_gantt = layout.opts.layout_style == LayoutStyle::Gantt;
    let (class, stroke, stroke_width, stroke_opacity) = if is_gantt {
        ("tdsl-grid-gantt", "#888", "1.5", "0.6")
    } else {
        ("tdsl-grid-line", "#ccc", "1", "0.4")
    };

    if layout.is_vertical() {
        // Vertical layout: time axis is Y; grid lines are horizontal.
        let x1 = layout.opts.left_gutter;
        let x2 = layout.total_width - layout.opts.right_margin;
        for frac in &positions {
            let y = layout.opts.top_margin + (frac - layout.year_min as f64) * layout.opts.scale;
            writeln!(
                s,
                r##"  <line class="{class}" role="presentation" x1="{x1}" y1="{y}" x2="{x2}" y2="{y}" stroke="{stroke}" stroke-width="{stroke_width}" stroke-opacity="{stroke_opacity}"/>"##,
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
                r##"  <line class="{class}" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}" stroke="{stroke}" stroke-width="{stroke_width}" stroke-opacity="{stroke_opacity}"/>"##,
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

    // Hour minor ticks (unit=hour only, #556).
    let pixels_per_hour = layout.opts.scale / (365.25 * 24.0);
    let single_day = crate::layout::is_single_day_range(&layout.ir.meta);
    for (year, month, day, hour) in layout.hour_ticks() {
        let x = layout.hour_frac_to_x(year, month, day, hour);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-hour-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 2.0),
            y2 = fmt_f(baseline_y),
        )?;
        if pixels_per_hour >= 4.0 {
            let label = crate::layout::format_hour_tick_label(month, day, hour, single_day);
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-hour-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
                label = escape_xml(&label),
            )?;
        }
    }

    // Minute minor ticks (unit=minute only, #556).
    let pixels_per_minute = layout.opts.scale / (365.25 * 24.0 * 60.0);
    for (year, month, day, hour, minute) in layout.minute_ticks() {
        let x = layout.minute_frac_to_x(year, month, day, hour, minute);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-minute-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 2.0),
            y2 = fmt_f(baseline_y),
        )?;
        if pixels_per_minute >= 4.0 {
            let label =
                crate::layout::format_minute_tick_label(month, day, hour, minute, single_day);
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-minute-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
                label = escape_xml(&label),
            )?;
        }
    }

    // Second minor ticks (unit=second only, #614, ADR 0003).
    let pixels_per_second = layout.opts.scale / (365.25 * 24.0 * 60.0 * 60.0);
    for (year, month, day, hour, minute, second) in layout.second_ticks() {
        let x = layout.second_frac_to_x(year, month, day, hour, minute, second);
        writeln!(
            s,
            r#"  <line class="tdsl-axis-second-tick" role="presentation" x1="{x}" y1="{y1}" x2="{x}" y2="{y2}"/>"#,
            x = fmt_f(x),
            y1 = fmt_f(baseline_y - 2.0),
            y2 = fmt_f(baseline_y),
        )?;
        if pixels_per_second >= 4.0 {
            let label = crate::layout::format_second_tick_label(
                month, day, hour, minute, second, single_day,
            );
            writeln!(
                s,
                r#"  <text class="tdsl-axis-text tdsl-axis-second-text" x="{x}" y="{y}" text-anchor="middle">{label}</text>"#,
                x = fmt_f(x),
                y = fmt_f(baseline_y - 5.0),
                label = escape_xml(&label),
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
                period_label_stack_level,
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
                // #550: hook class for ongoing (open-ended) spans so custom CSS
                // can style them (e.g. dashed border).
                let open_class = if item_end_open(item) {
                    " tdsl-item-open-ended"
                } else {
                    ""
                };
                let period_label_fragment = if layout.opts.layout_style == LayoutStyle::Gantt {
                    render_gantt_period_label_fragment(
                        item,
                        layout,
                        *x,
                        *y,
                        *width,
                        *height,
                        *period_label_stack_level,
                    )
                } else {
                    String::new()
                };
                writeln!(
                    s,
                    r#"  <g class="tdsl-item tdsl-item-span{open_class}" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-span" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="3"><title>{tip}</title></rect>{label_fragment}{period_label_fragment}</g>"#,
                    aria_label = aria_label,
                    tip = tip,
                    tip_attr = tip_attr,
                    fill_style = fill_style,
                    data_attrs = data_attrs,
                    open_class = open_class,
                    x = fmt_f(*x),
                    y = fmt_f(*y),
                    w = fmt_f(*width),
                    h = fmt_f(*height),
                    label_fragment = render_bar_label_fragment(
                        laid,
                        layout,
                        *x + 4.0,
                        *y + height / 2.0,
                        false,
                        "tdsl-item-label"
                    ),
                    period_label_fragment = period_label_fragment,
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
                period_label_stack_level,
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
                // #550: hook class for ongoing (open-ended) event_range items.
                let open_class = if item_end_open(item) {
                    " tdsl-item-open-ended"
                } else {
                    ""
                };
                let is_gantt = layout.opts.layout_style == LayoutStyle::Gantt;
                let period_label_fragment = if is_gantt {
                    render_gantt_period_label_fragment(
                        item,
                        layout,
                        *x,
                        *y,
                        *width,
                        *height,
                        *period_label_stack_level,
                    )
                } else {
                    String::new()
                };
                if layout.opts.show_event_labels {
                    let label_fragment = if layout.is_vertical() {
                        render_bar_label_fragment(
                            laid,
                            layout,
                            *x + *width / 2.0,
                            *y + *height + 12.0,
                            true,
                            "tdsl-event-label",
                        )
                    } else {
                        render_bar_label_fragment(
                            laid,
                            layout,
                            *x + 4.0,
                            *y + *height / 2.0,
                            false,
                            "tdsl-event-label",
                        )
                    };
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event-range{open_class}" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-range" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="2"><title>{tip}</title></rect>{label_fragment}{period_label_fragment}</g>"#,
                        aria_label = aria_label,
                        tip = tip,
                        tip_attr = tip_attr,
                        fill_style = fill_style,
                        data_attrs = data_attrs,
                        open_class = open_class,
                        x = fmt_f(*x),
                        y = fmt_f(*y),
                        w = fmt_f(*width),
                        h = fmt_f(*height),
                        label_fragment = label_fragment,
                        period_label_fragment = period_label_fragment,
                    )?;
                } else {
                    writeln!(
                        s,
                        r#"  <g class="tdsl-item tdsl-item-event-range{open_class}" role="group" aria-label="{aria_label}" tabindex="0" data-tdsl-tooltip="{tip_attr}"{data_attrs}><rect class="tdsl-event-range" style="{fill_style}" x="{x}" y="{y}" width="{w}" height="{h}" rx="2"><title>{tip}</title></rect>{period_label_fragment}</g>"#,
                        aria_label = aria_label,
                        tip = tip,
                        tip_attr = tip_attr,
                        fill_style = fill_style,
                        data_attrs = data_attrs,
                        open_class = open_class,
                        x = fmt_f(*x),
                        y = fmt_f(*y),
                        w = fmt_f(*width),
                        h = fmt_f(*height),
                        period_label_fragment = period_label_fragment,
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
                label_stack_level,
            } => {
                let label_stack_offset = *label_stack_level as f64 * EVENT_LABEL_STACK_STEP;
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
                        // Vertical: label to the right of the dot (dot_x + 6, same Y as dot
                        // center, offset upward per #537 when it collides with a neighbour).
                        let label_x = *y_dot + 6.0;
                        let label_y = *x - label_stack_offset;
                        if label_stack_offset > 0.0 {
                            writeln!(
                                s,
                                r##"    <line class="tdsl-label-leader" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#999" stroke-width="1" stroke-dasharray="2 2"></line>"##,
                                x1 = fmt_f(*y_dot),
                                y1 = fmt_f(*x),
                                x2 = fmt_f(*y_dot),
                                y2 = fmt_f(label_y),
                            )?;
                        }
                        write!(
                            s,
                            "{}",
                            render_bar_label_fragment(
                                laid,
                                layout,
                                label_x,
                                label_y,
                                false,
                                "tdsl-event-label",
                            )
                        )?;
                        writeln!(s)?;
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
                        // Horizontal: label above the dot, centered horizontally, offset
                        // further up per #537 when it collides with a neighbouring label.
                        let label_y = *y_top - 4.0 - label_stack_offset;
                        if label_stack_offset > 0.0 {
                            writeln!(
                                s,
                                r##"    <line class="tdsl-label-leader" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#999" stroke-width="1" stroke-dasharray="2 2"></line>"##,
                                x1 = fmt_f(*x),
                                y1 = fmt_f(*y_top),
                                x2 = fmt_f(*x),
                                y2 = fmt_f(label_y),
                            )?;
                        }
                        write!(
                            s,
                            "{}",
                            render_bar_label_fragment(
                                laid,
                                layout,
                                *x,
                                label_y,
                                true,
                                "tdsl-event-label",
                            )
                        )?;
                        writeln!(s)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Font-size fractions tried in order before falling back to truncation/external placement (#535).
const LABEL_SHRINK_STEPS: &[f64] = &[1.0, 0.85, 0.70];
/// Minimum useful font size (px); below this, shrinking stops helping legibility.
const LABEL_MIN_FONT_PX: f64 = 7.0;
/// External-label leader-line length (px) when a bar is too narrow to hold any text.
const LABEL_LEADER_LEN: f64 = 14.0;

fn base_font_size_for_class(class: &str) -> f64 {
    if class == "tdsl-item-label" {
        11.0
    } else {
        10.0
    }
}

/// Truncate `text` to fit within `available_px` at `font_size_px`, appending an
/// ellipsis ("…") when truncation occurs. Returns the original text unchanged
/// if it already fits.
fn truncate_with_ellipsis(text: &str, font_size_px: f64, available_px: f64) -> String {
    if estimate_text_width_px(text, font_size_px) <= available_px {
        return text.to_string();
    }
    let ellipsis_width = estimate_text_width_px("…", font_size_px);
    let mut out = String::new();
    for ch in text.chars() {
        let candidate_width = estimate_text_width_px(&out, font_size_px)
            + estimate_text_width_px(&ch.to_string(), font_size_px);
        if candidate_width + ellipsis_width > available_px {
            break;
        }
        out.push(ch);
    }
    if out.is_empty() {
        "…".to_string()
    } else {
        format!("{out}…")
    }
}

/// Render a `<text>` fragment for a bar/event label, applying the #535 overflow
/// strategy: shrink font-size in steps, then truncate with an ellipsis, and
/// finally (when even a truncated single char + ellipsis would not fit) move
/// the label outside the bar and connect it with a thin leader line. The full,
/// un-truncated text always remains available via the item's `<title>` tooltip
/// (and, per #536, the data table).
///
/// `anchor_x`/`anchor_y` is the normal (non-overflowing) label position.
/// `anchor_below` selects `text-anchor="middle"` with no `dominant-baseline` (label
/// centered below/above a point, matching the pre-existing EventRange-vertical and
/// Event-horizontal placements); when `false`, `text-anchor` is left at its default
/// (`start`) with `dominant-baseline="middle"` (matching Span/EventRange-horizontal
/// and Event-vertical placements).
fn render_bar_label_fragment(
    laid: &LaidItem<'_>,
    layout: &LayoutModel,
    anchor_x: f64,
    anchor_y: f64,
    anchor_below: bool,
    class: &str,
) -> String {
    let text = laid_item_label(laid);
    if text.is_empty() {
        return String::new();
    }
    let base_font = base_font_size_for_class(class);
    let available =
        label_available_width_px(laid, &layout.opts, layout.total_width, layout.total_height);

    let (x, y, text_anchor, baseline_attr) = if anchor_below {
        (anchor_x, anchor_y, r#" text-anchor="middle""#, "")
    } else {
        (anchor_x, anchor_y, "", r#" dominant-baseline="middle""#)
    };

    // 1. Try shrinking the font-size in fixed steps.
    for &fraction in LABEL_SHRINK_STEPS {
        let size = base_font * fraction;
        if size < LABEL_MIN_FONT_PX {
            break;
        }
        if estimate_text_width_px(text, size) <= available {
            let size_attr = if fraction < 1.0 {
                format!(r#" style="font-size:{}px""#, fmt_f(size))
            } else {
                String::new()
            };
            return format!(
                r#"<text class="{class}" x="{x}" y="{y}"{baseline_attr}{text_anchor}{size_attr}>{label}</text>"#,
                class = class,
                x = fmt_f(x),
                y = fmt_f(y),
                baseline_attr = baseline_attr,
                text_anchor = text_anchor,
                size_attr = size_attr,
                label = escape_xml(text),
            );
        }
    }

    // 2. Truncate with an ellipsis at the smallest shrink step.
    let min_size =
        (base_font * LABEL_SHRINK_STEPS[LABEL_SHRINK_STEPS.len() - 1]).max(LABEL_MIN_FONT_PX);
    let truncated = truncate_with_ellipsis(text, min_size, available);
    if estimate_text_width_px(&truncated, min_size) <= available.max(0.0) || available >= min_size {
        return format!(
            r#"<text class="{class}" x="{x}" y="{y}"{baseline_attr}{text_anchor} style="font-size:{size}px">{label}</text>"#,
            class = class,
            x = fmt_f(x),
            y = fmt_f(y),
            baseline_attr = baseline_attr,
            text_anchor = text_anchor,
            size = fmt_f(min_size),
            label = escape_xml(&truncated),
        );
    }

    // 3. Bar is too narrow even for a truncated label: place the label outside
    // the bar, offset by a fixed leader length, and connect it with a thin line.
    let (leader_x2, leader_y2, label_x, label_y, label_anchor) = if anchor_below {
        (
            anchor_x,
            anchor_y + LABEL_LEADER_LEN,
            anchor_x,
            anchor_y + LABEL_LEADER_LEN + min_size,
            r#" text-anchor="middle""#,
        )
    } else {
        (
            anchor_x + LABEL_LEADER_LEN,
            anchor_y,
            anchor_x + LABEL_LEADER_LEN + 2.0,
            anchor_y,
            r#" dominant-baseline="middle""#,
        )
    };
    format!(
        r##"<line class="tdsl-label-leader" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#999" stroke-width="1" stroke-dasharray="2 2"></line><text class="{class} tdsl-item-label-external" x="{lx}" y="{ly}"{label_anchor} style="font-size:{size}px">{label}</text>"##,
        x1 = fmt_f(anchor_x),
        y1 = fmt_f(anchor_y),
        x2 = fmt_f(leader_x2),
        y2 = fmt_f(leader_y2),
        class = class,
        lx = fmt_f(label_x),
        ly = fmt_f(label_y),
        label_anchor = label_anchor,
        size = fmt_f(min_size),
        label = escape_xml(&truncated),
    )
}

/// Render the always-on Gantt period label (#564: "<start>〜<end>") for a
/// Span/EventRange bar, placed just above the bar in horizontal orientation or
/// just to the right of the bar in vertical orientation. `period_label_stack_level`
/// (from `assign_period_label_stack_levels`) offsets colliding labels within the
/// same lane sub-row further away from the bar, each with a thin leader line
/// connecting it back, matching the #537 Event-label collision pattern.
fn render_gantt_period_label_fragment(
    item: &Item,
    layout: &LayoutModel,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    period_label_stack_level: u8,
) -> String {
    let text = crate::layout::gantt_period_label(item);
    if text.is_empty() {
        return String::new();
    }
    let stack_offset = period_label_stack_level as f64 * EVENT_LABEL_STACK_STEP;
    let class = "tdsl-gantt-period-label";
    if layout.is_vertical() {
        // Vertical: label to the right of the bar, vertically centered, pushed
        // further right per stack level.
        let label_x = x + width + 6.0 + stack_offset;
        let label_y = y + height / 2.0;
        if stack_offset > 0.0 {
            format!(
                r##"<line class="tdsl-label-leader" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#999" stroke-width="1" stroke-dasharray="2 2"></line><text class="{class}" x="{lx}" y="{ly}" dominant-baseline="middle">{label}</text>"##,
                x1 = fmt_f(x + width),
                y1 = fmt_f(label_y),
                x2 = fmt_f(label_x),
                y2 = fmt_f(label_y),
                class = class,
                lx = fmt_f(label_x),
                ly = fmt_f(label_y),
                label = escape_xml(&text),
            )
        } else {
            format!(
                r#"<text class="{class}" x="{lx}" y="{ly}" dominant-baseline="middle">{label}</text>"#,
                class = class,
                lx = fmt_f(label_x),
                ly = fmt_f(label_y),
                label = escape_xml(&text),
            )
        }
    } else {
        // Horizontal: label centered above the bar, pushed further up per stack level.
        let label_x = x + width / 2.0;
        let label_y = y - 4.0 - stack_offset;
        if stack_offset > 0.0 {
            format!(
                r##"<line class="tdsl-label-leader" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="#999" stroke-width="1" stroke-dasharray="2 2"></line><text class="{class}" x="{lx}" y="{ly}" text-anchor="middle">{label}</text>"##,
                x1 = fmt_f(label_x),
                y1 = fmt_f(y),
                x2 = fmt_f(label_x),
                y2 = fmt_f(label_y),
                class = class,
                lx = fmt_f(label_x),
                ly = fmt_f(label_y),
                label = escape_xml(&text),
            )
        } else {
            format!(
                r#"<text class="{class}" x="{lx}" y="{ly}" text-anchor="middle">{label}</text>"#,
                class = class,
                lx = fmt_f(label_x),
                ly = fmt_f(label_y),
                label = escape_xml(&text),
            )
        }
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

fn item_lane_id(item: &Item) -> &str {
    match item {
        Item::Span { lane, .. } | Item::Event { lane, .. } | Item::EventRange { lane, .. } => lane,
    }
}

fn item_tags(item: &Item) -> &[String] {
    match item {
        Item::Span { tags, .. } => tags,
        Item::Event { tags, .. } => tags,
        Item::EventRange { tags, .. } => tags,
    }
}

/// #550: whether the item is open-ended (`end` was `now`). `Event` has no
/// end, so it is always `false`. Used to add a `tdsl-item-open-ended` CSS
/// hook class so users can style ongoing periods (e.g. dashed border).
fn item_end_open(item: &Item) -> bool {
    match item {
        Item::Span { end_open, .. } | Item::EventRange { end_open, .. } => *end_open,
        Item::Event { .. } => false,
    }
}

/// Build the ARIA label string for a timeline item.
///
/// Format: `"<type>: <tooltip_on_one_line>, Lane: <lane_label>"`
/// Newlines in the tooltip are replaced with `, ` for a compact single-line value.
///
/// #701: fixed to English (was hardcoded Japanese) to match the surrounding
/// tooling ecosystem's UI language convention (e.g. `obsidian-tdsl`, #82).
fn item_aria_label(item: &Item, tooltip: &str, lane_label: &str) -> String {
    let type_str = match item {
        Item::Span { .. } => "Span",
        Item::Event { .. } => "Event",
        Item::EventRange { .. } => "Event range",
    };
    let info = tooltip.replace('\n', ", ");
    format!("{type_str}: {info}, Lane: {lane_label}")
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
///
/// Only compiled when a raster/vector backend is enabled (`png` or `pdf`);
/// SVG/HTML output keeps the `var()` references for CSS theming.
#[cfg(any(feature = "png", feature = "pdf"))]
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
#[cfg(any(feature = "png", feature = "pdf"))]
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
                Item::Event {
                    id: "event:han:-209".into(),
                    lane: "han".into(),
                    time: -209,
                    label: "陳勝・呉広の乱".into(),
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
    fn svg_renders_hour_ticks_for_unit_hour_timeline() {
        // #556: unit hour produces hour-level axis ticks/labels within a
        // single-day range.
        let ir = TimelineIr {
            meta: tdsl_core::ir::Meta {
                title: "Apollo 11".into(),
                unit: "hour".into(),
                range: (1969, 1969),
                range_start_month: Some(7),
                range_start_day: Some(20),
                range_start_hour: Some(0),
                range_start_minute: Some(0),
                range_end_month: Some(7),
                range_end_day: Some(20),
                range_end_hour: Some(23),
                range_end_minute: Some(59),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "mission".into(),
                label: "Mission".into(),
                kind: "event".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Event {
                id: "landing".into(),
                lane: "mission".into(),
                time: 1969,
                label: "Landing".into(),
                tags: vec![],
                source: None,
                origin: None,
                note: None,
                link: None,
                color: None,
                time_month: Some(7),
                time_day: Some(20),
                time_hour: Some(20),
                time_minute: Some(17),
                time_second: None,
                time_offset_minutes: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 6.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-axis-hour-tick"),
            "expected hour tick lines in SVG: {svg}"
        );
        assert!(
            svg.contains(">14:00<") || svg.contains(">00:00<"),
            "expected single-day HH:00 hour label: {svg}"
        );
    }

    #[test]
    fn svg_renders_second_ticks_for_unit_second_timeline() {
        // #614 (ADR 0003): unit second produces second-level axis ticks/labels
        // within a single-day range.
        let ir = TimelineIr {
            meta: tdsl_core::ir::Meta {
                title: "Countdown".into(),
                unit: "second".into(),
                range: (1969, 1969),
                range_start_month: Some(7),
                range_start_day: Some(20),
                range_start_hour: Some(20),
                range_start_minute: Some(17),
                range_start_second: Some(0),
                range_end_month: Some(7),
                range_end_day: Some(20),
                range_end_hour: Some(20),
                range_end_minute: Some(18),
                range_end_second: Some(0),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "mission".into(),
                label: "Mission".into(),
                kind: "event".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Event {
                id: "landing".into(),
                lane: "mission".into(),
                time: 1969,
                label: "Landing".into(),
                tags: vec![],
                source: None,
                origin: None,
                note: None,
                link: None,
                color: None,
                time_month: Some(7),
                time_day: Some(20),
                time_hour: Some(20),
                time_minute: Some(17),
                time_second: Some(40),
                time_offset_minutes: None,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 60.0 * 60.0 * 6.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-axis-second-tick"),
            "expected second tick lines in SVG: {svg}"
        );
        assert!(
            svg.contains(">20:17:00<") || svg.contains(">20:18:00<"),
            "expected single-day HH:MM:SS second label: {svg}"
        );
    }

    #[test]
    fn svg_second_ticks_empty_for_non_second_unit() {
        // unit=hour must not emit second ticks even if range_*_second happens
        // to be populated.
        let ir = TimelineIr {
            meta: tdsl_core::ir::Meta {
                title: "Apollo 11".into(),
                unit: "hour".into(),
                range: (1969, 1969),
                range_start_month: Some(7),
                range_start_day: Some(20),
                range_start_hour: Some(0),
                range_start_minute: Some(0),
                range_start_second: Some(0),
                range_end_month: Some(7),
                range_end_day: Some(20),
                range_end_hour: Some(23),
                range_end_minute: Some(59),
                range_end_second: Some(59),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };
        let opts = RenderOptions {
            scale: 365.25 * 24.0 * 60.0 * 60.0 * 6.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            !svg.contains("tdsl-axis-second-tick"),
            "unit=hour must not render second ticks: {svg}"
        );
    }

    #[test]
    fn svg_marks_open_ended_span_with_hook_class() {
        // #550: an open-ended span gets the `tdsl-item-open-ended` CSS hook
        // class; a closed span does not.
        let mut ir = sample_ir();
        if let Item::Span { end_open, .. } = &mut ir.items[0] {
            *end_open = true;
        }
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-span tdsl-item-open-ended""#),
            "open-ended span must carry the tdsl-item-open-ended hook class: {svg}"
        );
        assert!(
            svg.contains("進行中"),
            "open-ended span tooltip must say 進行中 instead of a placeholder end year"
        );
    }

    #[test]
    fn svg_closed_span_has_no_open_ended_class() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(!svg.contains("tdsl-item-open-ended"));
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
        // aria-label に "Span" が含まれる（#701: 英語に統一）
        assert!(
            svg.contains("Span:"),
            "span aria-label must contain type prefix 'Span:'"
        );
        // aria-label に期間情報が含まれる (BC206 と 220 の両方)
        assert!(
            svg.contains("BC206"),
            "span aria-label must contain start year"
        );
        // aria-label に "Lane:" が含まれる
        assert!(
            svg.contains("Lane:"),
            "aria-label must contain lane label reference"
        );
        // Event item: <g> に role=group と aria-label が含まれる
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-event" role="group" aria-label=""#),
            "event item must have role=group and aria-label"
        );
        // event aria-label に "Event" が含まれる
        assert!(
            svg.contains("Event:"),
            "event aria-label must contain type prefix 'Event:'"
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();

        // event_range の <g> に role=group と aria-label が含まれる
        assert!(
            svg.contains(r#"class="tdsl-item tdsl-item-event-range" role="group" aria-label=""#),
            "event_range item must have role=group and aria-label"
        );
        // aria-label に "Event range" が含まれる（#701: 英語に統一）
        assert!(
            svg.contains("Event range:"),
            "event_range aria-label must contain type prefix 'Event range:'"
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
        assert_eq!(format_date(1900, Some(2), None, None, None), "1900 Feb");
        assert_eq!(
            format_date(-206, Some(3), Some(15), None, None),
            "BC206 Mar 15"
        );
        assert_eq!(format_date(2000, None, None, None, None), "2000");
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
                note: None,
                link: None,
                color: None,
                time_month: Some(2),
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
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("BC206 Feb"),
            "expected 'BC206 Feb' in tooltip, got:\n{svg}"
        );
    }

    fn render_sample_with_color_map(color: &str) -> String {
        let ir = sample_ir();
        let color_map: std::collections::HashMap<String, String> =
            [("dynasty".to_string(), color.to_string())]
                .into_iter()
                .collect();
        let opts = RenderOptions {
            color_map,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        render_svg(&layout).unwrap()
    }

    #[test]
    fn color_map_tag_overrides_lane_palette() {
        let svg = render_sample_with_color_map("#cc0000");
        // The span item has tag "dynasty", so its fill must use the color_map color.
        assert!(
            svg.contains("fill:#cc0000;"),
            "expected fill:#cc0000; in SVG, got:\n{svg}"
        );
    }

    #[test]
    fn color_map_accepts_named_color_keyword() {
        let svg = render_sample_with_color_map("rebeccapurple");
        assert!(
            svg.contains("fill:rebeccapurple;"),
            "expected fill:rebeccapurple; in SVG, got:\n{svg}"
        );
    }

    #[test]
    fn color_map_invalid_declaration_falls_back_to_lane_palette() {
        let invalid = "#cc0000;stroke:red";
        let svg = render_sample_with_color_map(invalid);
        assert!(
            !svg.contains(invalid),
            "invalid color must not appear in SVG:\n{svg}"
        );
        assert!(
            svg.contains("fill:var(--tdsl-lane-0, #4682B4);"),
            "expected lane palette fallback in SVG, got:\n{svg}"
        );
    }

    #[test]
    fn color_map_invalid_function_falls_back_to_lane_palette() {
        let invalid = "url('x')";
        let svg = render_sample_with_color_map(invalid);
        assert!(
            !svg.contains(invalid),
            "invalid color must not appear in SVG:\n{svg}"
        );
        assert!(
            svg.contains("fill:var(--tdsl-lane-0, #4682B4);"),
            "expected lane palette fallback in SVG, got:\n{svg}"
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

    // ─── #535 ラベルはみ出し対策テスト ─────────────────────────────────────

    fn overflow_ir(label: &str, start: i64, end: i64) -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "test".into(),
                unit: "year".into(),
                range: (0, 1000),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::EventRange {
                id: "r1".into(),
                lane: "x".into(),
                start,
                end,
                label: label.into(),
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
        }
    }

    #[test]
    fn short_label_rendering_is_unchanged_by_overflow_logic() {
        // A short label that clearly fits must not get a font-size override or
        // be truncated/relocated (no regression for the common case).
        let ir = overflow_ir("OK", 0, 500);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 2.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(svg.contains(">OK</text>"));
        assert!(!svg.contains("tdsl-label-leader"));
        assert!(!svg.contains("font-size:") || !svg.contains("…"));
    }

    #[test]
    fn long_label_on_moderately_narrow_bar_shrinks_font_size() {
        let ir = overflow_ir("Moderately Long Label", 0, 30);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 3.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("font-size:"),
            "a long label on a narrow bar should trigger a font-size shrink, got:\n{svg}"
        );
    }

    #[test]
    fn long_label_on_very_narrow_bar_is_truncated_with_ellipsis() {
        let ir = overflow_ir("This Label Is Far Too Long To Fit", 0, 3);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 3.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains('…'),
            "a label that doesn't fit even at the smallest font size must be truncated with an ellipsis, got:\n{svg}"
        );
        // Full text must still be recoverable via the tooltip.
        assert!(svg.contains("This Label Is Far Too Long To Fit"));
    }

    #[test]
    fn extremely_narrow_bar_places_label_externally_with_leader_line() {
        let ir = overflow_ir("漢字だらけの非常に長いラベルテキスト", 0, 1);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 1.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-label-leader"),
            "an extremely narrow bar must relocate the label outside with a leader line, got:\n{svg}"
        );
        assert!(svg.contains("tdsl-item-label-external"));
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

    // ─── #537 イベントラベル袘噂回避（スタキング）テスト ────────────────────────────

    fn two_close_events_ir(labels: [&str; 2], times: [i64; 2]) -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "test".into(),
                unit: "year".into(),
                range: (0, 100),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "x".into(),
                label: "X".into(),
                kind: "custom".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: times
                .iter()
                .zip(labels.iter())
                .enumerate()
                .map(|(i, (&time, &label))| Item::Event {
                    id: format!("e{i}"),
                    lane: "x".into(),
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
                })
                .collect(),
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn colliding_horizontal_event_labels_are_stacked() {
        // Two events very close together (year 50 vs 51) at a large scale will
        // have overlapping estimated label widths; the second must be stacked
        // (offset upward) rather than overlapping the first.
        let ir = two_close_events_ir(["Alpha Event", "Beta Event"], [50, 51]);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 10.0,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let levels: Vec<u8> = layout
            .items
            .iter()
            .map(|i| match i {
                LaidItem::Event {
                    label_stack_level, ..
                } => *label_stack_level,
                _ => 0,
            })
            .collect();
        assert!(
            levels.iter().any(|&l| l > 0),
            "colliding event labels must be assigned different stack levels, got {levels:?}"
        );
        let svg = render_svg(&layout).unwrap();
        assert!(
            svg.contains("tdsl-label-leader"),
            "a stacked label should be connected to its dot with a leader line, got:\n{svg}"
        );
    }

    #[test]
    fn colliding_vertical_event_labels_are_stacked() {
        let ir = two_close_events_ir(["Alpha Event", "Beta Event"], [50, 51]);
        let opts = RenderOptions {
            show_event_labels: true,
            scale: 10.0,
            orientation: Orientation::Vertical,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let levels: Vec<u8> = layout
            .items
            .iter()
            .map(|i| match i {
                LaidItem::Event {
                    label_stack_level, ..
                } => *label_stack_level,
                _ => 0,
            })
            .collect();
        assert!(
            levels.iter().any(|&l| l > 0),
            "colliding event labels must be assigned different stack levels (vertical), got {levels:?}"
        );
    }

    #[test]
    fn non_colliding_event_labels_are_unaffected_by_stacking() {
        // Two events far apart never collide: both stay at stack level 0, and the
        // rendered output must not gain any leader lines (no regression).
        let ir = two_close_events_ir(["Alpha", "Beta"], [10, 90]);
        let opts = RenderOptions {
            show_event_labels: true,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        for i in &layout.items {
            if let LaidItem::Event {
                label_stack_level, ..
            } = i
            {
                assert_eq!(*label_stack_level, 0);
            }
        }
        let svg = render_svg(&layout).unwrap();
        assert!(!svg.contains("tdsl-label-leader"));
    }

    #[cfg(any(feature = "png", feature = "pdf"))]
    #[test]
    fn resolve_lane_vars_in_styles_replaces_within_style_attr() {
        let input = r#"<rect style="fill:var(--tdsl-lane-0, #4682B4);" x="0"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert_eq!(got, r#"<rect style="fill:#4682B4;" x="0"/>"#);
    }

    #[cfg(any(feature = "png", feature = "pdf"))]
    #[test]
    fn resolve_lane_vars_in_styles_leaves_text_content_untouched() {
        // User label that contains the variable syntax must not be modified.
        let input = r#"<title>var(--tdsl-lane-0, #color)</title><rect style="fill:var(--tdsl-lane-0, #4682B4);"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert!(got.contains("<title>var(--tdsl-lane-0, #color)</title>"));
        assert!(got.contains(r#"style="fill:#4682B4;""#));
    }

    #[cfg(any(feature = "png", feature = "pdf"))]
    #[test]
    fn resolve_lane_vars_in_styles_leaves_root_css_block_untouched() {
        let input = ":root { --tdsl-lane-0: #4682B4; }";
        assert_eq!(resolve_lane_vars_in_styles(input), input);
    }

    #[cfg(any(feature = "png", feature = "pdf"))]
    #[test]
    fn resolve_lane_vars_in_styles_handles_multiple_attrs() {
        let input = r#"<rect style="fill:var(--tdsl-lane-0, #4682B4);fill-opacity:0.75;"/><circle style="fill:var(--tdsl-lane-1, #E67E22);"/>"#;
        let got = resolve_lane_vars_in_styles(input);
        assert!(got.contains(r#"style="fill:#4682B4;fill-opacity:0.75;""#));
        assert!(got.contains(r#"style="fill:#E67E22;""#));
    }

    // ─── #536 show_table (SVG/PNG/PDF) テスト ──────────────────────────────────────────────

    #[test]
    fn show_table_false_no_table_elements_in_svg() {
        let ir = sample_ir();
        let layout = LayoutModel::compute(&ir, RenderOptions::default());
        let svg = render_svg(&layout).unwrap();
        assert!(
            !svg.contains(r#"class="tdsl-table""#),
            "show_table=false (default) must not emit a table, got:\n{svg}"
        );
    }

    #[test]
    fn show_table_true_emits_table_with_all_items() {
        let ir = sample_ir();
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let layout = LayoutModel::compute(&ir, opts);
        let svg = render_svg(&layout).unwrap();
        assert!(svg.contains(r#"class="tdsl-table""#));
        assert!(svg.contains("tdsl-table-header"));
        // Every item's label must appear somewhere in the table cells.
        for item in &ir.items {
            let label = match item {
                Item::Span { label, .. }
                | Item::Event { label, .. }
                | Item::EventRange { label, .. } => label,
            };
            assert!(
                svg.contains(label.as_str()),
                "table must contain label {label:?}, got:\n{svg}"
            );
        }
    }

    #[test]
    fn show_table_true_expands_total_height_to_fit_table() {
        let ir = sample_ir();
        let layout_without = LayoutModel::compute(&ir, RenderOptions::default());
        let opts = RenderOptions {
            show_table: true,
            ..RenderOptions::default()
        };
        let layout_with = LayoutModel::compute(&ir, opts);
        assert!(
            layout_with.total_height > layout_without.total_height,
            "show_table=true must reserve extra vertical space for the table"
        );
    }

    #[test]
    fn show_table_false_leaves_svg_output_otherwise_identical() {
        // Regression guard: toggling show_table off must not affect the rest of
        // the SVG output at all (identical bytes).
        let ir = sample_ir();
        let svg_default = render_svg(&LayoutModel::compute(&ir, RenderOptions::default())).unwrap();
        let svg_explicit_false = render_svg(&LayoutModel::compute(
            &ir,
            RenderOptions {
                show_table: false,
                ..RenderOptions::default()
            },
        ))
        .unwrap();
        assert_eq!(svg_default, svg_explicit_false);
    }

    // ─── render_table_page_svg: header repetition + page-number footer (#619 / ADR-0004 D4) ───

    fn sample_table_rows(count: usize) -> Vec<crate::layout::TableRow> {
        (0..count)
            .map(|i| crate::layout::TableRow {
                sort_year: i as i64,
                sort_type: 0,
                time_str: format!("{i}"),
                label: format!("Item {i}"),
                lane_label: "漢".to_string(),
                tags: String::new(),
            })
            .collect()
    }

    #[test]
    fn render_table_page_svg_repeats_column_headers_on_every_page() {
        // ADR-0004 D4: the header row (column names) must be present on every
        // table page, not just the first.
        let rows = sample_table_rows(3);
        let svg = render_table_page_svg(&rows, 500.0, 700.0, 1, 3).unwrap();
        for col in [
            crate::layout::TABLE_COL_TIME,
            crate::layout::TABLE_COL_LABEL,
            crate::layout::TABLE_COL_LANE,
            crate::layout::TABLE_COL_TAGS,
        ] {
            assert!(
                svg.contains(col),
                "table page SVG must contain the '{col}' column header, got: {svg}"
            );
        }
    }

    #[test]
    fn render_table_page_svg_includes_page_number_footer() {
        let rows = sample_table_rows(2);
        let svg = render_table_page_svg(&rows, 500.0, 700.0, 2, 5).unwrap();
        assert!(
            svg.contains("2 / 5"),
            "table page SVG must contain a '2 / 5' page-number footer, got: {svg}"
        );
        assert!(
            svg.contains("tdsl-table-page-footer"),
            "page-number footer must use the tdsl-table-page-footer CSS hook class"
        );
    }

    #[test]
    fn render_table_page_svg_page_number_differs_across_pages() {
        let rows = sample_table_rows(2);
        let page1 = render_table_page_svg(&rows, 500.0, 700.0, 1, 3).unwrap();
        let page2 = render_table_page_svg(&rows, 500.0, 700.0, 2, 3).unwrap();
        let page3 = render_table_page_svg(&rows, 500.0, 700.0, 3, 3).unwrap();
        assert!(page1.contains("1 / 3"));
        assert!(page2.contains("2 / 3"));
        assert!(page3.contains("3 / 3"));
    }

    #[test]
    fn render_table_page_svg_renders_cjk_lane_label_and_header() {
        // CJK lane label "漢" (from sample_table_rows) plus the CJK column
        // headers (時期/ラベル/レーン/タグ) must both appear verbatim, escaped
        // for XML but not otherwise mangled/dropped.
        let rows = sample_table_rows(1);
        let svg = render_table_page_svg(&rows, 500.0, 700.0, 1, 1).unwrap();
        assert!(svg.contains("漢"), "CJK lane label must appear, got: {svg}");
        assert!(svg.contains("時期"));
        assert!(svg.contains("ラベル"));
        assert!(svg.contains("レーン"));
        assert!(svg.contains("タグ"));
    }

    #[test]
    fn render_table_page_svg_empty_rows_still_has_header_and_footer() {
        // The first (and only) page of an empty table must still show the
        // repeated header and a "1 / 1" footer rather than an empty document.
        let svg = render_table_page_svg(&[], 500.0, 700.0, 1, 1).unwrap();
        assert!(svg.contains(crate::layout::TABLE_COL_TIME));
        assert!(svg.contains("1 / 1"));
    }
}
