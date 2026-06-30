<!-- markdownlint-disable MD013 MD060 -->

# Plan 005: Propagate DSL color_map through WASM and WebUI rendering

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 3176e9d..HEAD -- crates/tdsl-wasm/src/lib.rs apps/webui/src/wasmLoader.ts apps/webui/src/hooks/useCompiler.ts apps/webui/src/App.tsx`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `3176e9d`, 2026-06-28

## Why this matters

`color_map` is a documented DSL feature and the CLI honors it, but browser rendering currently drops it at the WASM boundary. That means WebUI preview, WebUI SVG export, and WebUI HTML/PDF export can disagree with `tdsl render` for the same source. This plan makes the IR's `meta.color_map` flow into `tdsl-render::RenderOptions` in the WASM facade, matching the CLI behavior without changing the DSL.

## Current state

Relevant files:

- `crates/tdsl-cli/src/commands/render.rs` — exemplar: CLI copies DSL `ir.meta.color_map` into render options.
- `crates/tdsl-wasm/src/lib.rs` — WASM facade; currently builds `RenderOptions` from JavaScript options only.
- `apps/webui/src/wasmLoader.ts` / `apps/webui/src/hooks/useCompiler.ts` — WebUI calls the WASM facade for preview.

Current CLI exemplar (`crates/tdsl-cli/src/commands/render.rs:134-137`, `:152`):

```rust
let mut color_map = ir.meta.color_map.clone();
if let Some(raw) = color_map_raw {
    for (tag, color) in parse_color_map(raw)? {
        color_map.insert(tag, color);
    }
}

let opts = tdsl_render::RenderOptions {
```

Current WASM option conversion (`crates/tdsl-wasm/src/lib.rs:792-800`):

```rust
RenderOptions {
    scale,
    lane_height,
    orientation,
    grid,
    theme,
    show_table: opts.show_table,
    show_event_labels: opts.show_event_labels,
    ..defaults
}
```

Current renderer lookup (`crates/tdsl-render/src/layout.rs:307` and `:714-715`):

```rust
let color = resolve_item_color(item_tags, &opts.color_map, lane_id, &lane_colors);

if let Some(color) = color_map.get(tag.as_str()) {
    return color.clone();
}
```

Repo conventions to follow:

- Keep IR authoritative; renderer inputs should be `TimelineIr` + `RenderOptions`, not parser AST.
- WASM facade stays thin and calls parser/core/render crates only.
- Tests for WASM facade live in `crates/tdsl-wasm/src/lib.rs` under `#[cfg(test)]`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Rust unit tests | `cargo test -p tdsl-wasm` | exit 0; new color_map regression test passes |
| Renderer tests | `cargo test -p tdsl-render` | exit 0 |
| WASM build check | `wasm-pack build crates/tdsl-wasm --target web` | exit 0 and generated exports still exist |
| WebUI build | `cd apps/webui && npm run build` | exit 0 |
| Full gate | `cargo test --workspace --all-targets` | exit 0 |

## Scope

**In scope**:

- `crates/tdsl-wasm/src/lib.rs`
- `apps/webui/src/wasmLoader.ts` only if TypeScript typings need adjustment after the Rust export shape changes
- `apps/webui/src/hooks/useCompiler.ts` only if the call site needs adjustment
- Tests in `crates/tdsl-wasm/src/lib.rs`

**Out of scope**:

- Changing DSL syntax or parser behavior.
- Adding CLI flags; CLI already handles `color_map`.
- Sanitizing color values; do that in Plan 006.
- Reworking WebUI settings/export behavior; do that in Plan 007.

## Git workflow

- Suggested branch: `advisor/005-wasm-color-map`
- Commit message style follows Conventional Commits, e.g. `fix(wasm): honor DSL color_map in browser rendering`.
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Thread IR color_map into WASM render options

In `crates/tdsl-wasm/src/lib.rs`, avoid losing `ir.meta.color_map` after lowering. A safe shape is:

1. Keep `js_opts_to_render_options(&JsRenderOptions, scale)` for JavaScript-controlled options.
2. Add a helper such as:

```rust
fn render_options_for_ir(ir: &tdsl_core::ir::TimelineIr, opts: &JsRenderOptions, scale: f64) -> RenderOptions {
    let mut render_opts = js_opts_to_render_options(opts, scale);
    render_opts.color_map = ir.meta.color_map.clone();
    render_opts
}
```

1. Use it in `render_svg_from_source_with_options` and `render_html_from_source_with_options`.
2. Also update legacy `render_svg_from_source` and `render_html_from_source` if they build `RenderOptions::default()` directly, so old WebUI callers and external package users get the same fix.

Do not put parser-specific logic in the renderer; the color map is already in IR.

**Verify**: `cargo test -p tdsl-wasm` → existing tests still pass.

### Step 2: Add native regression tests in tdsl-wasm

Add tests near existing `render_svg_with_options_produces_svg_output` in `crates/tdsl-wasm/src/lib.rs`:

- Build a TDSL source with `color_map { dynasty: "#3366cc"; }` and an item tagged `dynasty`.
- Parse/lower using the same pure Rust functions the WASM export uses.
- Render through the same helper that now merges IR color map.
- Assert the SVG contains `#3366cc` and no longer uses only the lane palette color for that tagged item.
- Add a companion test for HTML render if the helper is shared; checking that the embedded SVG contains `#3366cc` is enough.

Use existing native tests in the same file as the structural pattern; do not require a browser runtime for these tests.

**Verify**: `cargo test -p tdsl-wasm color_map` → only the new filtered tests run and pass.

### Step 3: Confirm WebUI does not need API changes

Inspect `apps/webui/src/wasmLoader.ts`. If the Rust wasm-bindgen signatures are unchanged, no TypeScript change is needed. If you add any JS-visible option field, update the `RenderOptions` interface and wrapper assignments consistently.

**Verify**: `cd apps/webui && npm run build` → TypeScript and Vite build pass.

### Step 4: Run integration gates

Run the Rust and WASM gates listed below.

**Verify**:

- `cargo test -p tdsl-render` → exit 0.
- `wasm-pack build crates/tdsl-wasm --target web` → exit 0.
- `cargo test --workspace --all-targets` → exit 0.

## Test plan

- New `tdsl-wasm` tests proving `ir.meta.color_map` reaches SVG and HTML rendering through the WASM facade helpers.
- Existing renderer tests remain unchanged; `tdsl-render` already has a tag override test for direct `RenderOptions.color_map` usage.

## Done criteria

- [ ] `cargo test -p tdsl-wasm` exits 0 and includes a color_map regression test.
- [ ] `cargo test -p tdsl-render` exits 0.
- [ ] `wasm-pack build crates/tdsl-wasm --target web` exits 0.
- [ ] `cd apps/webui && npm run build` exits 0.
- [ ] No files outside the in-scope list are modified, except generated wasm package files if `wasm-pack build` changes them and the repo convention requires committing them.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- `crates/tdsl-wasm/src/lib.rs` no longer has `js_opts_to_render_options` or the excerpts above do not match.
- Fixing the issue appears to require parser/lowering changes.
- The wasm-bindgen public API must become incompatible with existing WebUI calls.
- A verification command fails twice after a reasonable fix attempt.

## Maintenance notes

When adding future render options to the CLI, check whether the WASM facade should also inherit data from IR. Reviewers should specifically check both legacy and `with_options` WASM exports; fixing only one path will leave a subtle parity bug.
