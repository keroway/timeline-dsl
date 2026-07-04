/* @ts-self-types="./tdsl_wasm.d.ts" */

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
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        JsRenderOptionsFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_jsrenderoptions_free(ptr, 0);
    }
    /**
     * Height of each lane in pixels. `0` (default) uses the renderer default (60).
     * Larger values increase vertical density (taller bands and thicker bars).
     * @returns {number}
     */
    get lane_height() {
        const ret = wasm.__wbg_get_jsrenderoptions_lane_height(this.__wbg_ptr);
        return ret;
    }
    /**
     * When true, labels are rendered next to Event/EventRange items.
     * @returns {boolean}
     */
    get show_event_labels() {
        const ret = wasm.__wbg_get_jsrenderoptions_show_event_labels(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * When true, render a static legend panel showing lane and tag colors.
     * @returns {boolean}
     */
    get show_legend() {
        const ret = wasm.__wbg_get_jsrenderoptions_show_legend(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * When true, append an item listing table.
     * @returns {boolean}
     */
    get show_table() {
        const ret = wasm.__wbg_get_jsrenderoptions_show_table(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * @returns {string}
     */
    get grid() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsrenderoptions_grid(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * High-level visual layout style (#543/#564/#565): `"timeline"` (default),
     * `"group-bands"`, `"gantt"`, or `"zigzag"`. Orthogonal to `orientation`.
     * `"zigzag"` only takes effect when the timeline has at most
     * `ZIGZAG_MAX_LANES` lanes; beyond that it falls back to `"timeline"`
     * positioning at the `tdsl-render` layer — callers that need a
     * user-facing warning (mirroring the CLI's `--layout-style zigzag`
     * notice) should check the timeline's lane count themselves before
     * rendering.
     * @returns {string}
     */
    get layout_style() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsrenderoptions_layout_style(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export(deferred1_0, deferred1_1, 1);
        }
    }
    constructor() {
        const ret = wasm.jsrenderoptions_new();
        this.__wbg_ptr = ret;
        JsRenderOptionsFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * @returns {string}
     */
    get orientation() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsrenderoptions_orientation(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @param {string} val
     */
    set grid(val) {
        const ptr0 = passStringToWasm0(val, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.jsrenderoptions_set_grid(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} val
     */
    set layout_style(val) {
        const ptr0 = passStringToWasm0(val, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.jsrenderoptions_set_layout_style(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} val
     */
    set orientation(val) {
        const ptr0 = passStringToWasm0(val, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.jsrenderoptions_set_orientation(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @param {string} val
     */
    set theme(val) {
        const ptr0 = passStringToWasm0(val, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.jsrenderoptions_set_theme(this.__wbg_ptr, ptr0, len0);
    }
    /**
     * @returns {string}
     */
    get theme() {
        let deferred1_0;
        let deferred1_1;
        try {
            const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
            wasm.jsrenderoptions_theme(retptr, this.__wbg_ptr);
            var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
            var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
            deferred1_0 = r0;
            deferred1_1 = r1;
            return getStringFromWasm0(r0, r1);
        } finally {
            wasm.__wbindgen_add_to_stack_pointer(16);
            wasm.__wbindgen_export(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Height of each lane in pixels. `0` (default) uses the renderer default (60).
     * Larger values increase vertical density (taller bands and thicker bars).
     * @param {number} arg0
     */
    set lane_height(arg0) {
        wasm.__wbg_set_jsrenderoptions_lane_height(this.__wbg_ptr, arg0);
    }
    /**
     * When true, labels are rendered next to Event/EventRange items.
     * @param {boolean} arg0
     */
    set show_event_labels(arg0) {
        wasm.__wbg_set_jsrenderoptions_show_event_labels(this.__wbg_ptr, arg0);
    }
    /**
     * When true, render a static legend panel showing lane and tag colors.
     * @param {boolean} arg0
     */
    set show_legend(arg0) {
        wasm.__wbg_set_jsrenderoptions_show_legend(this.__wbg_ptr, arg0);
    }
    /**
     * When true, append an item listing table.
     * @param {boolean} arg0
     */
    set show_table(arg0) {
        wasm.__wbg_set_jsrenderoptions_show_table(this.__wbg_ptr, arg0);
    }
}
if (Symbol.dispose) JsRenderOptions.prototype[Symbol.dispose] = JsRenderOptions.prototype.free;

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
 * @param {string} source
 * @returns {string}
 */
export function check_source(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.check_source(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred2_0 = r0;
        deferred2_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Compile TDSL source to IR (JSON string), without Wikidata resolution.
 * `source_span` fields are populated for each static item (for bidirectional jump).
 * Returns Ok(json_string) or Err(error_message).
 * @param {string} source
 * @returns {string}
 */
export function compile_to_ir(source) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.compile_to_ir(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Format TDSL source by re-emitting from the AST.
 *
 * Parses `source` and re-emits a normalized form (2-space indent, single blank line
 * between top-level statements). Comments in the original source are **not preserved**
 * because they are skipped at the PEG layer.
 * Returns Ok(formatted_source) on success, Err(parse_error_message) on parse failure.
 * @param {string} source
 * @returns {string}
 */
export function format_source(source) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.format_source(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {string} source
 * @returns {string}
 */
export function lint_fix_source(source) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.lint_fix_source(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred3_0, deferred3_1, 1);
    }
}

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
 * @param {string} source
 * @returns {string}
 */
export function lint_source(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.lint_source(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred2_0 = r0;
        deferred2_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred2_0, deferred2_1, 1);
    }
}

/**
 * Initialize the panic hook for better error messages in the browser console.
 */
export function main() {
    wasm.main();
}

/**
 * Render standalone HTML from TDSL source (static items only).
 * Returns Ok(html_string) or Err(error_message).
 * @param {string} source
 * @returns {string}
 */
export function render_html_from_source(source) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.render_html_from_source(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Render standalone HTML from TDSL source with explicit render options.
 * Returns Ok(html_string) or Err(error_message).
 * @param {string} source
 * @param {JsRenderOptions} opts
 * @returns {string}
 */
export function render_html_from_source_with_options(source, opts) {
    let deferred4_0;
    let deferred4_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(opts, JsRenderOptions);
        var ptr1 = opts.__destroy_into_raw();
        wasm.render_html_from_source_with_options(retptr, ptr0, len0, ptr1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr3 = r0;
        var len3 = r1;
        if (r3) {
            ptr3 = 0; len3 = 0;
            throw takeObject(r2);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred4_0, deferred4_1, 1);
    }
}

/**
 * Render SVG from TDSL source (static items only).
 * `scale` controls pixels-per-year. Pass `0.0` (or negative) to auto-calculate
 * from the IR's `meta.range` (clamped to `0.5..=50.0`).
 * `source_span` (line numbers) are embedded as `data-line` attributes in the SVG
 * for bidirectional editor↔preview jump.
 * Returns Ok(svg_string) or Err(error_message).
 * @param {string} source
 * @param {number} scale
 * @returns {string}
 */
export function render_svg_from_source(source, scale) {
    let deferred3_0;
    let deferred3_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        wasm.render_svg_from_source(retptr, ptr0, len0, scale);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr2 = r0;
        var len2 = r1;
        if (r3) {
            ptr2 = 0; len2 = 0;
            throw takeObject(r2);
        }
        deferred3_0 = ptr2;
        deferred3_1 = len2;
        return getStringFromWasm0(ptr2, len2);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred3_0, deferred3_1, 1);
    }
}

/**
 * Render SVG from TDSL source with explicit render options.
 *
 * `scale` controls pixels-per-year. Pass `0.0` (or negative) to auto-calculate.
 * Returns Ok(svg_string) or Err(error_message).
 * @param {string} source
 * @param {number} scale
 * @param {JsRenderOptions} opts
 * @returns {string}
 */
export function render_svg_from_source_with_options(source, scale, opts) {
    let deferred4_0;
    let deferred4_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
        const len0 = WASM_VECTOR_LEN;
        _assertClass(opts, JsRenderOptions);
        var ptr1 = opts.__destroy_into_raw();
        wasm.render_svg_from_source_with_options(retptr, ptr0, len0, scale, ptr1);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        var r2 = getDataViewMemory0().getInt32(retptr + 4 * 2, true);
        var r3 = getDataViewMemory0().getInt32(retptr + 4 * 3, true);
        var ptr3 = r0;
        var len3 = r1;
        if (r3) {
            ptr3 = 0; len3 = 0;
            throw takeObject(r2);
        }
        deferred4_0 = ptr3;
        deferred4_1 = len3;
        return getStringFromWasm0(ptr3, len3);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export(deferred4_0, deferred4_1, 1);
    }
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_export(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return addHeapObject(ret);
        },
        __wbg_now_d2e0afbad4edbe82: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = getObject(arg1).stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export2, wasm.__wbindgen_export3);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return addHeapObject(ret);
        },
        __wbindgen_object_drop_ref: function(arg0) {
            takeObject(arg0);
        },
    };
    return {
        __proto__: null,
        "./tdsl_wasm_bg.js": import0,
    };
}

const JsRenderOptionsFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_jsrenderoptions_free(ptr, 1));

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function _assertClass(instance, klass) {
    if (!(instance instanceof klass)) {
        throw new Error(`expected instance of ${klass.name}`);
    }
}

function dropObject(idx) {
    if (idx < 1028) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getObject(idx) { return heap[idx]; }

let heap = new Array(1024).fill(undefined);
heap.push(undefined, null, true, false);

let heap_next = heap.length;

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('tdsl_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
