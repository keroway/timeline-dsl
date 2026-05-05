/* tslint:disable */
/* eslint-disable */

/**
 * Check TDSL source and return diagnostics as JSON.
 * Returns a JSON array of diagnostic objects with `severity`, `message`, `line`, `col`.
 */
export function check_source(source: string): string;

/**
 * Compile TDSL source to IR (JSON string), without Wikidata resolution.
 * Returns Ok(json_string) or Err(error_message).
 */
export function compile_to_ir(source: string): string;

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
 * Render SVG from TDSL source (static items only).
 * Returns Ok(svg_string) or Err(error_message).
 */
export function render_svg_from_source(source: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly check_source: (a: number, b: number, c: number) => void;
    readonly compile_to_ir: (a: number, b: number, c: number) => void;
    readonly render_html_from_source: (a: number, b: number, c: number) => void;
    readonly render_svg_from_source: (a: number, b: number, c: number) => void;
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
