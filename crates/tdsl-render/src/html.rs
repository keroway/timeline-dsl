/// Wrap a pre-rendered SVG string in a standalone HTML document with embedded CSS.
pub fn wrap_html(svg_body: &str, title: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
</style>
</head>
<body>
<h1>{title}</h1>
<div class="tdsl-timeline">
{svg}
</div>
<div id="tdsl-tooltip" class="tdsl-tooltip" role="tooltip" hidden aria-hidden="true"></div>
<script>
{js}
</script>
</body>
</html>
"#,
        title = escape_html(title),
        css = EMBEDDED_CSS,
        svg = svg_body,
        js = EMBEDDED_JS,
    )
}

const EMBEDDED_CSS: &str = r#"body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Hiragino Sans",
    "Yu Gothic UI", sans-serif;
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
.tdsl-axis-baseline  { stroke: #888; stroke-width: 1; }
.tdsl-axis-tick      { stroke: #e0e0e0; stroke-width: 1; }
.tdsl-axis-text      { font-size: 11px; fill: #666; }
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
.tdsl-item-label {
  font-size: 11px;
  fill: #fff;
  pointer-events: none;
  font-weight: 500;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_wraps_with_doctype_and_svg() {
        let html = wrap_html("<svg></svg>", "test title");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<title>test title</title>"));
        assert!(html.contains("<style>"));
        assert!(html.contains("<svg></svg>"));
        assert!(html.contains(r#"id="tdsl-tooltip""#));
        assert!(html.contains(r#"data-tdsl-tooltip"#));
    }

    #[test]
    fn html_escapes_title() {
        let html = wrap_html("<svg></svg>", "A & B <danger>");
        assert!(html.contains("A &amp; B &lt;danger&gt;"));
        assert!(!html.contains("<danger>"));
    }
}
