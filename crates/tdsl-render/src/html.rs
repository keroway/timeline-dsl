/// Wrap a pre-rendered SVG string in a standalone HTML document with embedded CSS.
pub fn wrap_html(
    svg_body: &str,
    title: &str,
    opts: &crate::layout::RenderOptions,
    table_html: Option<&str>,
) -> String {
    use crate::layout::Theme;

    let theme_css = match opts.theme {
        Theme::Default => "",
        Theme::Dark => DARK_THEME_CSS,
        Theme::Print => PRINT_THEME_CSS,
        Theme::Pastel => PASTEL_THEME_CSS,
    };

    let custom_css_block = match &opts.custom_css {
        Some(css) => format!("\n<style>\n{css}\n</style>"),
        None => String::new(),
    };

    let table_block = match table_html {
        Some(t) => format!("\n<div class=\"tdsl-table-wrap\">\n{t}\n</div>"),
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
{theme_css}</style>{custom_css_block}
</head>
<body>
<h1>{title}</h1>
<div class="tdsl-timeline">
{svg}
</div>{table_block}
<div id="tdsl-tooltip" class="tdsl-tooltip" role="tooltip" hidden aria-hidden="true"></div>
<script>
{js}
</script>
</body>
</html>
"#,
        title = escape_html(title),
        css = EMBEDDED_CSS,
        theme_css = theme_css,
        custom_css_block = custom_css_block,
        svg = svg_body,
        table_block = table_block,
        js = EMBEDDED_JS,
    )
}

const EMBEDDED_CSS: &str = r#"body {
  font-family: "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo",
    -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  margin: 24px;
  color: #222;
  background: #fafafa;
}
h1 {
  font-size: 18px;
  margin: 0 0 16px;
  font-weight: 600;
}
.tdsl-timeline {
  background: #fff;
  border: 1px solid #e5e5e5;
  border-radius: 4px;
  padding: 8px;
  overflow-x: auto;
}
.tdsl-timeline svg {
  display: block;
}
.tdsl-lane-band-even { fill: #fff; }
.tdsl-lane-band-odd  { fill: #f5f5f7; }
.tdsl-group-band-even { fill: #eef4fb; }
.tdsl-group-band-odd  { fill: #f6f0fb; }
.tdsl-group-label    { font-size: 11px; fill: #555; font-weight: bold; }
.tdsl-axis-baseline       { stroke: #888; stroke-width: 1; }
.tdsl-axis-tick           { stroke: #e0e0e0; stroke-width: 1; }
.tdsl-axis-month-tick     { stroke: #ccc; stroke-width: 1; }
.tdsl-axis-text           { font-size: 11px; fill: #666; }
.tdsl-lane-label     { font-size: 13px; fill: #333; font-weight: 500; }
.tdsl-span {
  fill: #4682B4;
  fill-opacity: 0.78;
  stroke: #2a4d6e;
  stroke-width: 1;
  cursor: pointer;
  transition: fill-opacity 0.15s;
}
.tdsl-span:hover { fill-opacity: 1; }
.tdsl-event-range {
  fill: #DC143C;
  fill-opacity: 0.75;
  stroke: #8b0c1a;
  stroke-width: 1;
  cursor: pointer;
  transition: fill-opacity 0.15s;
}
.tdsl-event-range:hover { fill-opacity: 1; }
.tdsl-event-dot {
  fill: #333;
  stroke: #fff;
  stroke-width: 1;
  cursor: pointer;
}
.tdsl-event-dot:hover { fill: #1a73e8; }
.tdsl-event-stem     { stroke: #aaa; stroke-width: 1; stroke-dasharray: 2 2; }
/* Invisible but hoverable hit-area so the thin stem + tiny dot are easy to hover for tooltips. */
.tdsl-event-hit      { fill: transparent; cursor: pointer; }
/* #550: open-ended (`now`-terminated) span/event_range bars get a dashed
   right edge to visually signal "still ongoing". Override via --custom-css. */
.tdsl-item-open-ended .tdsl-span,
.tdsl-item-open-ended .tdsl-event-range {
  stroke-dasharray: 4 2;
}
.tdsl-item-label {
  font-size: 11px;
  fill: #fff;
  pointer-events: none;
  font-weight: 500;
}
.tdsl-event-label {
  font-size: 10px;
  fill: #333;
  pointer-events: none;
  user-select: none;
}
.tdsl-item:focus-visible .tdsl-span,
.tdsl-item:focus-visible .tdsl-event-range,
.tdsl-item:focus-visible .tdsl-event-dot {
  stroke: #1a73e8;
  stroke-width: 2;
}
.tdsl-tooltip {
  position: fixed;
  left: 0;
  top: 0;
  z-index: 9999;
  max-width: min(360px, calc(100vw - 16px));
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid #d0d7de;
  background: rgba(255, 255, 255, 0.96);
  color: #111;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
  font-size: 12px;
  line-height: 1.45;
  white-space: pre-line;
  pointer-events: none;
}
.tdsl-table-wrap { margin-top: 2rem; overflow-x: auto; }
.tdsl-table { width: 100%; border-collapse: collapse; font-size: 0.9em; }
.tdsl-table th, .tdsl-table td { border: 1px solid #ccc; padding: 0.4em 0.6em; text-align: left; }
.tdsl-table th { background: #f5f5f5; font-weight: bold; position: sticky; top: 0; }
.tdsl-table tbody tr:nth-child(even) { background: #fafafa; }
"#;

/// Dark theme overrides — deep navy background, light text.
const DARK_THEME_CSS: &str = r#"body {
  background: #1a1a2e;
  color: #e0e0e0;
}
h1 { color: #e0e0e0; }
.tdsl-timeline {
  background: #16213e;
  border-color: #0f3460;
}
.tdsl-lane-band-even { fill: #16213e; }
.tdsl-lane-band-odd  { fill: #0f3460; }
.tdsl-group-band-even { fill: #1f2b4d; }
.tdsl-group-band-odd  { fill: #142a52; }
.tdsl-group-label     { fill: #ccc; }
.tdsl-axis-baseline       { stroke: #555; }
.tdsl-axis-tick           { stroke: #2a4a7f; }
.tdsl-axis-month-tick     { stroke: #1e3a5f; }
.tdsl-axis-text           { fill: #aaa; }
.tdsl-lane-label     { fill: #ccc; }
.tdsl-item-label     { fill: #f0f0f0; }
.tdsl-tooltip {
  background: rgba(22, 33, 62, 0.96);
  border-color: #0f3460;
  color: #e0e0e0;
}
"#;

/// Print theme overrides — monochrome, high contrast.
const PRINT_THEME_CSS: &str = r#"body {
  background: #fff;
  color: #000;
}
.tdsl-timeline {
  background: #fff;
  border-color: #000;
}
.tdsl-lane-band-even { fill: #fff; }
.tdsl-lane-band-odd  { fill: #eee; }
.tdsl-group-band-even { fill: #fff; }
.tdsl-group-band-odd  { fill: #ddd; }
.tdsl-group-label     { fill: #000; }
.tdsl-axis-baseline       { stroke: #000; }
.tdsl-axis-tick           { stroke: #bbb; }
.tdsl-axis-month-tick     { stroke: #999; }
.tdsl-axis-text           { fill: #000; }
.tdsl-lane-label     { fill: #000; }
.tdsl-span {
  fill: #333;
  stroke: #000;
}
.tdsl-event-range {
  fill: #666;
  stroke: #000;
}
.tdsl-event-dot      { fill: #000; }
.tdsl-item-label     { fill: #fff; }
@media print {
  .tdsl-table th { background: #333 !important; color: #fff !important; }
  .tdsl-table th, .tdsl-table td { border-color: #333; }
  thead { display: table-header-group; }
}
"#;

/// Pastel theme overrides — soft, light colors with rounded spans.
const PASTEL_THEME_CSS: &str = r#"body {
  background: #fef9f0;
  color: #444;
}
.tdsl-timeline {
  background: #fffdf7;
  border-color: #e8dcc8;
}
.tdsl-lane-band-even { fill: #fffdf7; }
.tdsl-lane-band-odd  { fill: #fdf3e3; }
.tdsl-group-band-even { fill: #fdf3e3; }
.tdsl-group-band-odd  { fill: #f3e6f5; }
.tdsl-group-label     { fill: #888; }
.tdsl-axis-baseline       { stroke: #ccc; }
.tdsl-axis-tick           { stroke: #e8dcc8; }
.tdsl-axis-month-tick     { stroke: #ddd; }
.tdsl-axis-text           { fill: #888; }
.tdsl-lane-label     { fill: #666; }
.tdsl-span {
  fill: #b5d5f5;
  stroke: #7aabdf;
  rx: 6;
}
.tdsl-event-range {
  fill: #f5c6b5;
  stroke: #df8a7a;
}
.tdsl-event-dot      { fill: #888; stroke: #fff; }
.tdsl-item-label     { fill: #333; }
"#;

const EMBEDDED_JS: &str = r#"(() => {
  const tooltip = document.getElementById("tdsl-tooltip");
  if (!tooltip) return;

  const items = document.querySelectorAll(".tdsl-item[data-tdsl-tooltip]");
  if (!items.length) return;

  const GAP = 12;
  const PAD = 8;

  const hide = () => {
    tooltip.hidden = true;
    tooltip.setAttribute("aria-hidden", "true");
  };

  const show = (text) => {
    if (!text) return;
    tooltip.textContent = text;
    tooltip.hidden = false;
    tooltip.setAttribute("aria-hidden", "false");
  };

  const move = (clientX, clientY) => {
    if (tooltip.hidden) return;
    const rect = tooltip.getBoundingClientRect();
    let x = clientX + GAP;
    let y = clientY + GAP;

    if (x + rect.width > window.innerWidth - PAD) {
      x = Math.max(PAD, clientX - rect.width - GAP);
    }
    if (y + rect.height > window.innerHeight - PAD) {
      y = Math.max(PAD, clientY - rect.height - GAP);
    }

    tooltip.style.left = `${x}px`;
    tooltip.style.top = `${y}px`;
  };

  const showAtElement = (el) => {
    const text = el.getAttribute("data-tdsl-tooltip");
    if (!text) return;
    show(text);
    const box = el.getBoundingClientRect();
    move(box.left + box.width / 2, box.top + box.height / 2);
  };

  for (const el of items) {
    el.addEventListener("pointerenter", (event) => {
      show(el.getAttribute("data-tdsl-tooltip"));
      move(event.clientX, event.clientY);
    });
    el.addEventListener("pointermove", (event) => {
      move(event.clientX, event.clientY);
    });
    el.addEventListener("pointerleave", hide);
    el.addEventListener("focus", () => showAtElement(el));
    el.addEventListener("blur", hide);
  }

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") hide();
  });
  window.addEventListener("scroll", hide, { passive: true });
  window.addEventListener("resize", hide);
})();"#;

fn escape_html(s: &str) -> String {
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

/// Generate an HTML table listing all items from the IR in chronological order.
///
/// Columns: 時期 (time period) / ラベル (label) / レーン (lane) / タグ (tags).
/// Items are sorted by start/time year ascending, then by item type (span > event_range > event),
/// then by label for ties.
///
/// Row-collection logic is shared with the SVG/PNG/PDF table emitter (#536)
/// via [`crate::layout::collect_table_rows`].
pub(crate) fn generate_table_html(
    ir: &tdsl_core::ir::TimelineIr,
    lanes: &[tdsl_core::ir::Lane],
) -> String {
    use crate::layout::{
        TABLE_COL_LABEL, TABLE_COL_LANE, TABLE_COL_TAGS, TABLE_COL_TIME, collect_table_rows,
    };

    let lane_label = |lane_id: &str| -> String {
        lanes
            .iter()
            .find(|l| l.id == lane_id)
            .map(|l| l.label.clone())
            .unwrap_or_else(|| lane_id.to_string())
    };

    let rows = collect_table_rows(ir, lane_label);

    let mut html = String::new();
    html.push_str("<table class=\"tdsl-table\">\n");
    html.push_str("<thead>\n<tr>");
    for col in [
        TABLE_COL_TIME,
        TABLE_COL_LABEL,
        TABLE_COL_LANE,
        TABLE_COL_TAGS,
    ] {
        html.push_str(&format!("<th>{}</th>", escape_html(col)));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");

    for row in &rows {
        html.push_str("<tr>");
        html.push_str(&format!("<td>{}</td>", escape_html(&row.time_str)));
        html.push_str(&format!("<td>{}</td>", escape_html(&row.label)));
        html.push_str(&format!("<td>{}</td>", escape_html(&row.lane_label)));
        html.push_str(&format!("<td>{}</td>", escape_html(&row.tags)));
        html.push_str("</tr>\n");
    }

    html.push_str("</tbody>\n</table>");
    html
}

/// Wrap a pre-rendered SVG string in an interactive standalone HTML document.
///
/// The interactive document adds zoom/pan, search filtering, click detail panel,
/// and lane legend checkboxes — all via inline JS/CSS with no external CDN deps.
pub fn wrap_html_interactive(
    svg_body: &str,
    title: &str,
    opts: &crate::layout::RenderOptions,
    lanes: &[tdsl_core::ir::Lane],
    table_html: Option<&str>,
) -> String {
    use crate::layout::Theme;

    let theme_css = match opts.theme {
        Theme::Default => "",
        Theme::Dark => DARK_THEME_CSS,
        Theme::Print => PRINT_THEME_CSS,
        Theme::Pastel => PASTEL_THEME_CSS,
    };

    let custom_css_block = match &opts.custom_css {
        Some(css) => format!("\n<style>\n{css}\n</style>"),
        None => String::new(),
    };

    // Generate legend HTML from lane data.
    let mut legend_html = String::new();
    for lane in lanes {
        let lane_id_escaped = escape_html(&lane.id);
        let lane_label_escaped = escape_html(&lane.label);
        legend_html.push_str(&format!(
            r#"<label class="tdsl-legend-item"><input type="checkbox" checked data-lane-toggle="{lane_id_escaped}"> {lane_label_escaped}</label>"#,
        ));
    }

    let table_block = match table_html {
        Some(t) => format!("\n<div class=\"tdsl-table-wrap\">\n{t}\n</div>"),
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
{theme_css}{interactive_css}</style>{custom_css_block}
</head>
<body>
<div id="tdsl-app">
  <header id="tdsl-header">
    <h1>{title}</h1>
    <input id="tdsl-search" type="search" placeholder="ラベル検索..." autocomplete="off">
  </header>
  <div id="tdsl-main">
    <div id="tdsl-legend">
      {legend_html}
    </div>
    <div id="tdsl-canvas">
      {svg}
    </div>
    <div id="tdsl-detail" hidden>
      <button id="tdsl-detail-close" aria-label="閉じる">&times;</button>
      <div id="tdsl-detail-content"></div>
    </div>
  </div>
</div>{table_block}
<div id="tdsl-tooltip" class="tdsl-tooltip" role="tooltip" hidden aria-hidden="true"></div>
<script>
{js}
</script>
</body>
</html>
"#,
        title = escape_html(title),
        css = EMBEDDED_CSS,
        theme_css = theme_css,
        interactive_css = INTERACTIVE_CSS,
        custom_css_block = custom_css_block,
        legend_html = legend_html,
        svg = svg_body,
        table_block = table_block,
        js = INTERACTIVE_JS,
    )
}

const INTERACTIVE_CSS: &str = r#"
/* Interactive mode layout */
#tdsl-app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  margin: 0;
  overflow: hidden;
}
#tdsl-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 16px;
  background: #fff;
  border-bottom: 1px solid #e5e5e5;
  flex-shrink: 0;
}
#tdsl-header h1 {
  margin: 0;
  font-size: 16px;
}
#tdsl-search {
  padding: 5px 10px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 13px;
  width: 200px;
  outline-offset: 2px;
}
#tdsl-main {
  display: flex;
  flex: 1;
  overflow: hidden;
}
#tdsl-legend {
  width: 160px;
  flex-shrink: 0;
  padding: 12px 8px;
  border-right: 1px solid #e5e5e5;
  overflow-y: auto;
  background: #fafafa;
  font-size: 12px;
}
.tdsl-legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
  cursor: pointer;
  user-select: none;
}
#tdsl-canvas {
  flex: 1;
  overflow: auto;
  position: relative;
  cursor: grab;
}
#tdsl-canvas.dragging {
  cursor: grabbing;
}
#tdsl-canvas svg {
  display: block;
}
#tdsl-detail {
  width: 240px;
  flex-shrink: 0;
  padding: 12px;
  border-left: 1px solid #e5e5e5;
  background: #fafafa;
  font-size: 13px;
  overflow-y: auto;
  position: relative;
}
#tdsl-detail[hidden] {
  display: none;
}
#tdsl-detail-close {
  position: absolute;
  top: 8px;
  right: 8px;
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: #666;
  line-height: 1;
  padding: 2px 6px;
}
#tdsl-detail-close:hover {
  color: #111;
}
#tdsl-detail-content {
  margin-top: 4px;
}
#tdsl-detail-content dl {
  margin: 0;
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 4px 8px;
}
#tdsl-detail-content dt {
  font-weight: 600;
  color: #555;
  white-space: nowrap;
}
#tdsl-detail-content dd {
  margin: 0;
  word-break: break-all;
}
/* Search highlight / dim */
.tdsl-item.tdsl-search-dim {
  opacity: 0.15;
}
.tdsl-item.tdsl-search-match .tdsl-span,
.tdsl-item.tdsl-search-match .tdsl-event-range,
.tdsl-item.tdsl-search-match .tdsl-event-dot {
  stroke: #f5c000;
  stroke-width: 2.5;
}
/* Lane hide */
.tdsl-item.tdsl-lane-hidden {
  display: none;
}
"#;

const INTERACTIVE_JS: &str = r#"(() => {
  // ── Tooltip (reuse existing logic) ──────────────────────────────────────
  const tooltip = document.getElementById("tdsl-tooltip");
  const items = document.querySelectorAll(".tdsl-item[data-tdsl-tooltip]");
  if (tooltip && items.length) {
    const GAP = 12, PAD = 8;
    const hide = () => { tooltip.hidden = true; tooltip.setAttribute("aria-hidden","true"); };
    const show = (text) => {
      if (!text) return;
      tooltip.textContent = text;
      tooltip.hidden = false;
      tooltip.setAttribute("aria-hidden","false");
    };
    const move = (cx, cy) => {
      if (tooltip.hidden) return;
      const r = tooltip.getBoundingClientRect();
      let x = cx + GAP, y = cy + GAP;
      if (x + r.width  > window.innerWidth  - PAD) x = Math.max(PAD, cx - r.width  - GAP);
      if (y + r.height > window.innerHeight - PAD) y = Math.max(PAD, cy - r.height - GAP);
      tooltip.style.left = x + "px";
      tooltip.style.top  = y + "px";
    };
    const showAt = (el) => {
      const text = el.getAttribute("data-tdsl-tooltip");
      if (!text) return;
      show(text);
      const box = el.getBoundingClientRect();
      move(box.left + box.width / 2, box.top + box.height / 2);
    };
    for (const el of items) {
      el.addEventListener("pointerenter", (e) => { show(el.getAttribute("data-tdsl-tooltip")); move(e.clientX, e.clientY); });
      el.addEventListener("pointermove",  (e) => move(e.clientX, e.clientY));
      el.addEventListener("pointerleave", hide);
      el.addEventListener("focus", () => showAt(el));
      el.addEventListener("blur",  hide);
    }
    document.addEventListener("keydown", (e) => { if (e.key === "Escape") hide(); });
    window.addEventListener("scroll", hide, { passive: true });
    window.addEventListener("resize", hide);
  }

  // ── Zoom (wheel) + Pan (drag) ────────────────────────────────────────────
  const canvas = document.getElementById("tdsl-canvas");
  if (canvas) {
    const svg = canvas.querySelector("svg");
    let svgBaseWidth = svg ? parseFloat(svg.getAttribute("width") || "0") : 0;
    let zoomLevel = 1.0;

    // Zoom: scale SVG width on wheel; canvas overflow:auto provides scrollbars.
    if (svg && svgBaseWidth > 0) {
      canvas.addEventListener("wheel", (e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.12 : 0.893;
        zoomLevel = Math.min(10, Math.max(0.25, zoomLevel * factor));
        // Preserve the hovered x-position after zoom.
        const canvasRect = canvas.getBoundingClientRect();
        const mouseX = e.clientX - canvasRect.left + canvas.scrollLeft;
        const ratio = mouseX / (svgBaseWidth * zoomLevel / factor);
        svg.style.width = (svgBaseWidth * zoomLevel) + "px";
        canvas.scrollLeft = ratio * svgBaseWidth * zoomLevel - (e.clientX - canvasRect.left);
      }, { passive: false });
    }

    // Pan: drag to scroll horizontally.
    let dragging = false;
    let startX = 0, startScrollLeft = 0;
    canvas.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      dragging = true;
      startX = e.clientX;
      startScrollLeft = canvas.scrollLeft;
      canvas.classList.add("dragging");
    });
    document.addEventListener("mousemove", (e) => {
      if (!dragging) return;
      const dx = e.clientX - startX;
      canvas.scrollLeft = startScrollLeft - dx;
    });
    document.addEventListener("mouseup", () => {
      if (dragging) {
        dragging = false;
        canvas.classList.remove("dragging");
      }
    });
  }

  // ── Search filter ────────────────────────────────────────────────────────
  const searchInput = document.getElementById("tdsl-search");
  const allItems = Array.from(document.querySelectorAll(".tdsl-item[data-label]"));
  if (searchInput && allItems.length) {
    searchInput.addEventListener("input", () => {
      const q = searchInput.value.trim().toLowerCase();
      for (const el of allItems) {
        const label = (el.getAttribute("data-label") || "").toLowerCase();
        if (!q) {
          el.classList.remove("tdsl-search-dim", "tdsl-search-match");
        } else if (label.includes(q)) {
          el.classList.remove("tdsl-search-dim");
          el.classList.add("tdsl-search-match");
        } else {
          el.classList.remove("tdsl-search-match");
          el.classList.add("tdsl-search-dim");
        }
      }
    });
  }

  // ── Click detail panel ───────────────────────────────────────────────────
  const detail = document.getElementById("tdsl-detail");
  const detailContent = document.getElementById("tdsl-detail-content");
  const detailClose = document.getElementById("tdsl-detail-close");
  if (detail && detailContent && allItems.length) {
    const showDetail = (el) => {
      const label  = el.getAttribute("data-label")  || "";
      const type_  = el.getAttribute("data-type")   || "";
      const lane   = el.getAttribute("data-lane")   || "";
      const source = el.getAttribute("data-source") || "";
      let html = "<dl>";
      if (label)  html += "<dt>ラベル</dt><dd>" + escapeHtml(label) + "</dd>";
      if (type_)  html += "<dt>種別</dt><dd>"   + escapeHtml(type_) + "</dd>";
      if (lane)   html += "<dt>レーン</dt><dd>"  + escapeHtml(lane)  + "</dd>";
      if (source) {
        const wd = source.match(/^wd:(Q\d+)$/);
        if (wd) {
          html += "<dt>出典</dt><dd><a href='https://www.wikidata.org/wiki/" + wd[1] + "' target='_blank' rel='noopener'>" + escapeHtml(source) + "</a></dd>";
        } else {
          html += "<dt>出典</dt><dd>" + escapeHtml(source) + "</dd>";
        }
      }
      html += "</dl>";
      detailContent.innerHTML = html;
      detail.hidden = false;
    };
    for (const el of allItems) {
      el.addEventListener("click", () => showDetail(el));
    }
    if (detailClose) {
      detailClose.addEventListener("click", () => { detail.hidden = true; });
    }
    document.addEventListener("keydown", (e) => { if (e.key === "Escape") detail.hidden = true; });
  }

  // ── Legend toggles ───────────────────────────────────────────────────────
  const laneToggles = document.querySelectorAll("[data-lane-toggle]");
  if (laneToggles.length) {
    const laneItemMap = {};
    for (const el of allItems) {
      const l = el.getAttribute("data-lane") || "";
      if (!laneItemMap[l]) laneItemMap[l] = [];
      laneItemMap[l].push(el);
    }
    for (const cb of laneToggles) {
      cb.addEventListener("change", () => {
        const laneId = cb.getAttribute("data-lane-toggle");
        const visible = cb.checked;
        for (const el of (laneItemMap[laneId] || [])) {
          el.classList.toggle("tdsl-lane-hidden", !visible);
        }
      });
    }
  }

  // ── Utility ─────────────────────────────────────────────────────────────
  function escapeHtml(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }
})();"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::RenderOptions;
    use crate::layout::Theme;

    #[test]
    fn html_wraps_with_doctype_and_svg() {
        let opts = RenderOptions::default();
        let html = wrap_html("<svg></svg>", "test title", &opts, None);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>test title</title>"));
        assert!(html.contains("<style>"));
        assert!(html.contains("<svg></svg>"));
        assert!(html.contains(r#"id="tdsl-tooltip""#));
        assert!(html.contains(r#"data-tdsl-tooltip"#));
    }

    #[test]
    fn standalone_html_has_no_external_font_dependency() {
        let opts = RenderOptions::default();
        let html = wrap_html("<svg></svg>", "test title", &opts, None);
        assert!(!html.contains("fonts.googleapis.com"));
        assert!(!html.contains("fonts.gstatic.com"));
        assert!(!html.contains("https://"));
        assert!(html.contains("Hiragino Sans"));
    }

    #[test]
    fn interactive_html_has_no_external_font_dependency() {
        let opts = RenderOptions::default();
        let html = wrap_html_interactive("<svg></svg>", "test title", &opts, &[], None);
        assert!(!html.contains("fonts.googleapis.com"));
        assert!(!html.contains("fonts.gstatic.com"));
        assert!(!html.contains("fonts.googleapis.com/css2"));
    }

    #[test]
    fn html_escapes_title() {
        let opts = RenderOptions::default();
        let html = wrap_html("<svg></svg>", "A & B <danger>", &opts, None);
        assert!(html.contains("A &amp; B &lt;danger&gt;"));
        assert!(!html.contains("<danger>"));
    }

    #[test]
    fn dark_theme_applies_dark_background() {
        let opts = RenderOptions {
            theme: Theme::Dark,
            ..Default::default()
        };
        let html = wrap_html("<svg></svg>", "test", &opts, None);
        assert!(html.contains("1a1a2e"), "dark theme should include #1a1a2e");
    }

    #[test]
    fn custom_css_is_injected() {
        let opts = RenderOptions {
            custom_css: Some(".tdsl-span { fill: hotpink; }".into()),
            ..Default::default()
        };
        let html = wrap_html("<svg></svg>", "test", &opts, None);
        assert!(html.contains("hotpink"), "custom CSS should be in output");
    }

    #[test]
    fn print_theme_applies_monochrome_background() {
        let opts = RenderOptions {
            theme: Theme::Print,
            ..Default::default()
        };
        let html = wrap_html("<svg></svg>", "test", &opts, None);
        // Print theme uses white background and black text
        assert!(
            html.contains("#fff") || html.contains("#ffffff"),
            "print theme should include white background"
        );
        assert!(
            html.contains("#000") || html.contains("#000000"),
            "print theme should include black foreground"
        );
    }

    #[test]
    fn pastel_theme_applies_soft_colors() {
        let opts = RenderOptions {
            theme: Theme::Pastel,
            ..Default::default()
        };
        let html = wrap_html("<svg></svg>", "test", &opts, None);
        // Pastel theme uses #fef9f0 as background
        assert!(
            html.contains("fef9f0"),
            "pastel theme should include #fef9f0 background"
        );
        // Pastel theme uses rounded spans (rx: 6)
        assert!(
            html.contains("b5d5f5"),
            "pastel theme should include pastel blue span color"
        );
    }

    #[test]
    fn table_html_is_inserted_when_some() {
        let opts = RenderOptions::default();
        let table = "<table class=\"tdsl-table\"><thead><tr><th>時期</th></tr></thead><tbody></tbody></table>";
        let html = wrap_html("<svg></svg>", "test", &opts, Some(table));
        assert!(
            html.contains("<div class=\"tdsl-table-wrap\">"),
            "wrap_html with table_html=Some should include the table-wrap div element"
        );
        assert!(
            html.contains("<table class=\"tdsl-table\">"),
            "wrap_html with table_html=Some should include the table element"
        );
    }

    #[test]
    fn table_html_absent_when_none() {
        let opts = RenderOptions::default();
        let html = wrap_html("<svg></svg>", "test", &opts, None);
        assert!(
            !html.contains("<div class=\"tdsl-table-wrap\">"),
            "wrap_html with table_html=None must not include the table-wrap div element"
        );
    }

    #[test]
    fn generate_table_html_basic() {
        use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

        let ir = TimelineIr {
            meta: Meta {
                title: "テスト".into(),
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
                    label: "漢王朝".into(),
                    tags: vec!["dynasty".into()],
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
                    end_open: false,
                    source_span: None,
                },
                Item::Event {
                    id: "event:1".into(),
                    lane: "han".into(),
                    time: 0,
                    label: "紀元".into(),
                    tags: vec![],
                    source: None,
                    origin: None,
                    time_month: None,
                    time_day: None,
                    time_hour: None,
                    time_minute: None,
                    source_span: None,
                },
            ],
            imports: vec![],
            sources: vec![],
        };

        let table = generate_table_html(&ir, &ir.lanes);
        assert!(
            table.contains("<table class=\"tdsl-table\">"),
            "table must have tdsl-table class"
        );
        assert!(table.contains("<thead>"), "table must have thead");
        assert!(table.contains("<tbody>"), "table must have tbody");
        assert!(table.contains("漢王朝"), "table must contain span label");
        assert!(table.contains("紀元"), "table must contain event label");
        assert!(table.contains("漢"), "table must contain lane label");
        assert!(table.contains("dynasty"), "table must contain tags");
    }

    #[test]
    fn generate_table_html_escapes_special_chars() {
        use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

        let ir = TimelineIr {
            meta: Meta {
                title: "T".into(),
                unit: "year".into(),
                range: (0, 100),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "l".into(),
                label: "Lane <A> & \"B\"".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![Item::Event {
                id: "e1".into(),
                lane: "l".into(),
                time: 50,
                label: "<script>alert('xss')</script>".into(),
                tags: vec!["<bad>".into()],
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

        let table = generate_table_html(&ir, &ir.lanes);
        assert!(
            !table.contains("<script>"),
            "table must not contain raw <script> tag"
        );
        assert!(
            table.contains("&lt;script&gt;"),
            "label special chars must be escaped"
        );
        assert!(
            !table.contains("<bad>"),
            "tag special chars must be escaped"
        );
        assert!(
            table.contains("Lane &lt;A&gt; &amp; &quot;B&quot;"),
            "lane label special chars must be escaped"
        );
    }

    #[test]
    fn generate_table_html_sort_order() {
        use tdsl_core::ir::{Item, Lane, Meta, TimelineIr};

        let ir = TimelineIr {
            meta: Meta {
                title: "T".into(),
                unit: "year".into(),
                range: (0, 500),
                calendar: "proleptic_gregorian".into(),
                color_map: std::collections::HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "l".into(),
                label: "L".into(),
                kind: "k".into(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Event {
                    id: "e3".into(),
                    lane: "l".into(),
                    time: 300,
                    label: "C_event".into(),
                    tags: vec![],
                    source: None,
                    origin: None,
                    time_month: None,
                    time_day: None,
                    time_hour: None,
                    time_minute: None,
                    source_span: None,
                },
                Item::Span {
                    id: "s1".into(),
                    lane: "l".into(),
                    start: 100,
                    end: 200,
                    label: "A_span".into(),
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
                    end_open: false,
                    source_span: None,
                },
                Item::EventRange {
                    id: "er2".into(),
                    lane: "l".into(),
                    start: 200,
                    end: 250,
                    label: "B_event_range".into(),
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
                    end_open: false,
                    source_span: None,
                },
            ],
            imports: vec![],
            sources: vec![],
        };

        let table = generate_table_html(&ir, &ir.lanes);
        let pos_a = table.find("A_span").expect("A_span must be in table");
        let pos_b = table
            .find("B_event_range")
            .expect("B_event_range must be in table");
        let pos_c = table.find("C_event").expect("C_event must be in table");
        assert!(
            pos_a < pos_b,
            "A_span (year 100) must appear before B_event_range (year 200)"
        );
        assert!(
            pos_b < pos_c,
            "B_event_range (year 200) must appear before C_event (year 300)"
        );
    }
}
