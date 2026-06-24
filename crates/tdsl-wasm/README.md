# @keroway/tdsl-wasm

WebAssembly bindings for the [Timeline DSL](https://github.com/keroway/timeline-dsl) compiler.
Compile `.tdsl` source to JSON IR and render it to SVG / HTML directly in the browser — no server required.

> **Limitation**: Wikidata imports (`import wikidata` / `query`) are **not** resolved in the browser (no network access). Only static `span`, `event`, and `event_range` items are compiled and rendered. Unresolved imports are silently skipped.

## Install

```bash
npm install @keroway/tdsl-wasm
```

## Usage

This package is built with `wasm-pack --target web`, so you must call the default-exported initializer once before using any function (it loads the `.wasm` binary). Works with bundlers such as Vite, webpack, and Rollup.

```js
import init, {
  compile_to_ir,
  render_svg_from_source,
  render_html_from_source,
  check_source,
  format_source,
} from "@keroway/tdsl-wasm";

// Load the WASM module once at startup.
await init();

const source = `
timeline "Demo" {
  unit year;
  range 1..100;
}
lane "Main" as main { kind dynasty; order 1; }
span main 10..50 "An era" {};
`;

// Compile to JSON IR (throws on error).
const ir = JSON.parse(compile_to_ir(source));

// Render to SVG. Pass 0 for auto scale (pixels-per-year derived from meta.range).
const svg = render_svg_from_source(source, 0);

// Render to a standalone HTML document.
const html = render_html_from_source(source);

// Lint / diagnostics: returns a JSON array [{severity, message, line, col}].
// line/col are 1-based when a source position is available; diagnostics without
// a position (e.g. unknown-lane lowering errors) report line: 0, col: 0.
const diagnostics = JSON.parse(check_source(source));

// Re-emit normalized source (2-space indent). Comments are not preserved.
const formatted = format_source(source);
```

### Render options (`JsRenderOptions`)

For finer control over the output, use the `*_with_options` entry points with a
`JsRenderOptions` instance:

```js
import init, {
  JsRenderOptions,
  render_svg_from_source_with_options,
} from "@keroway/tdsl-wasm";

await init();

const opts = new JsRenderOptions();
opts.orientation = "vertical"; // "horizontal" (default) | "vertical"
opts.grid = "decade"; // "none" (default) | "decade" | "year" | "month"
opts.theme = "dark"; // "default" | "dark" | "print" | "pastel"
opts.show_table = true;
opts.show_event_labels = true;
opts.lane_height = 96; // px per lane; 0 (default) = renderer default (60)

// scale = pixels-per-year; pass 0 for auto.
const svg = render_svg_from_source_with_options(source, 0, opts);
```

`lane_height` controls vertical density. Increasing it makes the SVG taller and
proportionally thickens each lane band, bar, and intra-lane padding — useful for
timelines with few lanes but many overlapping spans. Leave it at `0` to keep the
default appearance.

| Field | Accepted values | Default |
|---|---|---|
| `orientation` | `"horizontal"`, `"vertical"` | `"horizontal"` |
| `grid` | `"none"`, `"decade"`, `"year"`, `"month"` | `"none"` |
| `theme` | `"default"`, `"dark"`, `"print"`, `"pastel"` | `"default"` |
| `show_table` | `true`, `false` | `false` |
| `show_event_labels` | `true`, `false` | `false` |
| `lane_height` | px per lane; `0` = renderer default (60) | `0` |

## API

| Function | Signature | Description |
|---|---|---|
| `init(module_or_path?)` | `(…) => Promise<InitOutput>` | Default export. Loads the `.wasm` binary. Must be awaited before any call. |
| `compile_to_ir` | `(source: string) => string` | Compile to JSON IR. Throws on compile error. |
| `render_svg_from_source` | `(source: string, scale: number) => string` | Render SVG. `scale` = pixels-per-year; pass `0` for auto. |
| `render_svg_from_source_with_options` | `(source: string, scale: number, opts: JsRenderOptions) => string` | Render SVG with explicit options (orientation, grid, theme, table, event labels, `lane_height`). |
| `render_html_from_source` | `(source: string) => string` | Render a standalone HTML document. |
| `render_html_from_source_with_options` | `(source: string, opts: JsRenderOptions) => string` | Render a standalone HTML document with explicit options. |
| `check_source` | `(source: string) => string` | Diagnostics as a JSON array `[{severity, message, line, col}]`. `line`/`col` are 1-based when a position is available, else `0`. |
| `format_source` | `(source: string) => string` | Re-emit normalized source from the AST. |

Full TypeScript definitions ship with the package (`tdsl_wasm.d.ts`).

## Links

- [Timeline DSL repository](https://github.com/keroway/timeline-dsl) — language reference, CLI, examples
- [DSL Language Reference](https://github.com/keroway/timeline-dsl/blob/main/docs/dsl-spec.en.md)
- [Issues](https://github.com/keroway/timeline-dsl/issues)

## License

MIT
