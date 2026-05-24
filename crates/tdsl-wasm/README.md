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

// Lint / diagnostics: returns a JSON array [{severity, message, line, col}] (0-indexed).
const diagnostics = JSON.parse(check_source(source));

// Re-emit normalized source (2-space indent). Comments are not preserved.
const formatted = format_source(source);
```

## API

| Function | Signature | Description |
|---|---|---|
| `init(module_or_path?)` | `(…) => Promise<InitOutput>` | Default export. Loads the `.wasm` binary. Must be awaited before any call. |
| `compile_to_ir` | `(source: string) => string` | Compile to JSON IR. Throws on compile error. |
| `render_svg_from_source` | `(source: string, scale: number) => string` | Render SVG. `scale` = pixels-per-year; pass `0` for auto. |
| `render_html_from_source` | `(source: string) => string` | Render a standalone HTML document. |
| `check_source` | `(source: string) => string` | Diagnostics as a JSON array `[{severity, message, line, col}]`. |
| `format_source` | `(source: string) => string` | Re-emit normalized source from the AST. |

Full TypeScript definitions ship with the package (`tdsl_wasm.d.ts`).

## Links

- [Timeline DSL repository](https://github.com/keroway/timeline-dsl) — language reference, CLI, examples
- [DSL Language Reference](https://github.com/keroway/timeline-dsl/blob/main/docs/dsl-spec.en.md)
- [Issues](https://github.com/keroway/timeline-dsl/issues)

## License

MIT
