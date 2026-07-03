/* tslint:disable */
/* eslint-disable */

/**
 * Rendering options exposed to JavaScript.
 *
 * Create with `new JsRenderOptions()` — all fields default to the same values
 * as `RenderOptions::default()`.  String fields (`orientation`, `grid`, `theme`)
 * accept the lowercase variant names defined below.
 *
 * | Field | Accepted values | Default |
 * |-------|----------------|---------|
 * | `orientation` | `"horizontal"`, `"vertical"` | `"horizontal"` |
 * | `grid` | `"none"`, `"decade"`, `"year"`, `"month"` | `"none"` |
 * | `layout_style` | `"timeline"`, `"group-bands"`, `"gantt"`, `"zigzag"` | `"timeline"` |
 * | `theme` | `"default"`, `"dark"`, `"print"`, `"pastel"` | `"default"` |
 * | `show_table` | `true`, `false` | `false` |
 * | `show_legend` | `true`, `false` | `false` |
 * | `show_event_labels` | `true`, `false` | `false` |
 * | `lane_height` | px per lane; `0` = renderer default (60) | `0` |
 *
 * `lane_height` controls vertical density: the SVG height, each lane band, the
 * bar thickness and intra-lane padding all follow it. Leave it at `0` (the
 * default) to keep the historical appearance.
 */
export class JsRenderOptions {
    free(): void;
    [Symbol.dispose](): void;
    constructor();
    /**
     * Height of each lane in pixels. `0` (default) uses the renderer default (60).
     * Larger values increase vertical density (taller bands and thicker bars).
     */
    lane_height: number;
    /**
     * When true, labels are rendered next to Event/EventRange items.
     */
    show_event_labels: boolean;
    /**
     * When true, render a static legend panel showing lane and tag colors.
     */
    show_legend: boolean;
    /**
     * When true, append an item listing table.
     */
    show_table: boolean;
    grid: string;
    /**
     * High-level visual layout style (#543/#564/#565): `"timeline"` (default),
     * `"group-bands"`, `"gantt"`, or `"zigzag"`. Orthogonal to `orientation`.
     * `"zigzag"` only takes effect when the timeline has at most
     * `ZIGZAG_MAX_LANES` lanes; beyond that it falls back to `"timeline"`
     * positioning at the `tdsl-render` layer — callers that need a
     * user-facing warning (mirroring the CLI's `--layout-style zigzag`
     * notice) should check the timeline's lane count themselves before
     * rendering.
     */
    layout_style: string;
    orientation: string;
    theme: string;
}

/**
 * Check TDSL source and return diagnostics as JSON.
 *
 * Returns a JSON array of diagnostic objects: `[{severity, message, line, col}]`.
 * `severity` is `"error"`, `"warning"`, or `"info"`.
 *
 * `line`/`col` are **1-based** when a source position is available (parse errors via
 * `ParseError::source_location`, validation warnings via the item's `source_span`),
 * matching the IR `SourceSpan` numbering used by `render_svg_from_source`'s `data-line`
 * attributes. Diagnostics that carry no position (lowering errors such as unknown-lane
 * references) report `line: 0, col: 0`; the WebUI treats a `0` line as non-clickable.
 *
 * **Note on `import`/`map` blocks**: Wikidata fetch is not available in the browser.
 * Each `import` or `map` block receives an `"info"` diagnostic pointing to its start
 * line so the user knows to run `tdsl build` for full resolution.
 */
export function check_source(source: string): string;

/**
 * Compile TDSL source to IR (JSON string), without Wikidata resolution.
 * `source_span` fields are populated for each static item (for bidirectional jump).
 * Returns Ok(json_string) or Err(error_message).
 */
export function compile_to_ir(source: string): string;

/**
 * Format TDSL source by re-emitting from the AST.
 *
 * Parses `source` and re-emits a normalized form (2-space indent, single blank line
 * between top-level statements). Comments in the original source are **not preserved**
 * because they are skipped at the PEG layer.
 * Returns Ok(formatted_source) on success, Err(parse_error_message) on parse failure.
 */
export function format_source(source: string): string;

/**
 * Apply `tdsl lint --fix` to TDSL source and return the rewritten source.
 *
 * - When at least one fixable issue is applied, the rewritten source is returned.
 * - When the input has no fixable issues, the original source is returned unchanged
 *   (callers can compare lengths / equality to detect a no-op).
 * - On parse failure, returns `Err(parse_error_message)`.
 *
 * Comments are not preserved because `tdsl-parser` skips them at the PEG layer,
 * matching the behavior of `format_source` and the `tdsl lint --fix` CLI.
 */
export function lint_fix_source(source: string): string;

/**
 * Run lint rules on TDSL source and return issues as JSON.
 *
 * Returns a JSON array of `{code, severity, line, message, fixable}` objects.
 * `severity` is `"error"` or `"warning"` (lint does not emit `"info"`).
 * `line` is **1-based** and matches `check_source`'s line numbering.
 * `fixable` is `true` when `lint_fix_source` can automatically resolve the issue.
 *
 * On parse error, the array contains a single entry with `code: "parse_error"`
 * so callers can still surface the failure through the same path as lint issues.
 */
export function lint_source(source: string): string;

/**
 * Initialize the panic hook for better error messages in the browser console.
 */
export function main(): void;

/**
 * Render standalone HTML from TDSL source (static items only).
 * Returns Ok(html_string) or Err(error_message).
 */
export function render_html_from_source(source: string): string;

/**
 * Render standalone HTML from TDSL source with explicit render options.
 * Returns Ok(html_string) or Err(error_message).
 */
export function render_html_from_source_with_options(source: string, opts: JsRenderOptions): string;

/**
 * Render SVG from TDSL source (static items only).
 * `scale` controls pixels-per-year. Pass `0.0` (or negative) to auto-calculate
 * from the IR's `meta.range` (clamped to `0.5..=50.0`).
 * `source_span` (line numbers) are embedded as `data-line` attributes in the SVG
 * for bidirectional editor↔preview jump.
 * Returns Ok(svg_string) or Err(error_message).
 */
export function render_svg_from_source(source: string, scale: number): string;

/**
 * Render SVG from TDSL source with explicit render options.
 *
 * `scale` controls pixels-per-year. Pass `0.0` (or negative) to auto-calculate.
 * Returns Ok(svg_string) or Err(error_message).
 */
export function render_svg_from_source_with_options(source: string, scale: number, opts: JsRenderOptions): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_get_jsrenderoptions_lane_height: (a: number) => number;
    readonly __wbg_get_jsrenderoptions_show_event_labels: (a: number) => number;
    readonly __wbg_get_jsrenderoptions_show_legend: (a: number) => number;
    readonly __wbg_get_jsrenderoptions_show_table: (a: number) => number;
    readonly __wbg_jsrenderoptions_free: (a: number, b: number) => void;
    readonly __wbg_set_jsrenderoptions_lane_height: (a: number, b: number) => void;
    readonly __wbg_set_jsrenderoptions_show_event_labels: (a: number, b: number) => void;
    readonly __wbg_set_jsrenderoptions_show_legend: (a: number, b: number) => void;
    readonly __wbg_set_jsrenderoptions_show_table: (a: number, b: number) => void;
    readonly check_source: (a: number, b: number, c: number) => void;
    readonly compile_to_ir: (a: number, b: number, c: number) => void;
    readonly format_source: (a: number, b: number, c: number) => void;
    readonly jsrenderoptions_grid: (a: number, b: number) => void;
    readonly jsrenderoptions_layout_style: (a: number, b: number) => void;
    readonly jsrenderoptions_new: () => number;
    readonly jsrenderoptions_orientation: (a: number, b: number) => void;
    readonly jsrenderoptions_set_grid: (a: number, b: number, c: number) => void;
    readonly jsrenderoptions_set_layout_style: (a: number, b: number, c: number) => void;
    readonly jsrenderoptions_set_orientation: (a: number, b: number, c: number) => void;
    readonly jsrenderoptions_set_theme: (a: number, b: number, c: number) => void;
    readonly jsrenderoptions_theme: (a: number, b: number) => void;
    readonly lint_fix_source: (a: number, b: number, c: number) => void;
    readonly lint_source: (a: number, b: number, c: number) => void;
    readonly render_html_from_source: (a: number, b: number, c: number) => void;
    readonly render_html_from_source_with_options: (a: number, b: number, c: number, d: number) => void;
    readonly render_svg_from_source: (a: number, b: number, c: number, d: number) => void;
    readonly render_svg_from_source_with_options: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly main: () => void;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
