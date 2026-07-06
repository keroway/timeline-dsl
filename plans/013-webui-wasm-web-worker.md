<!-- markdownlint-disable MD013 MD060 -->

# Plan 013: Move WebUI WASM compile/render off the main thread into a Web Worker

> **Executor instructions**: Follow this plan step by step. Run every verification command. If anything in "STOP conditions" occurs, stop and report. When done, update the status row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 1c22dac..HEAD -- apps/webui/src/wasmLoader.ts apps/webui/src/hooks/useWasm.ts apps/webui/src/hooks/useCompiler.ts`

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MEDIUM
- **Depends on**: none (but touches the same files as several WebUI plans — sequence after 011/012 to avoid churn)
- **Category**: performance / architecture
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #591

## Why this matters

Every debounced keystroke currently runs `check_source` + `lint_source` +
`render_svg_from_source` **synchronously on the main thread** via the WASM
module loaded in `wasmLoader.ts`. For small timelines this is fine, but for
large timelines (many lanes/events, `import`/`query` expansions when pasted as
static data, or heavy `color_map`) the compile+render blocks the UI: the editor
janks, scrolling stutters, and the debounce (`DEBOUNCE_MS`) only masks it.

`docs/webui-design.md` already records "Full Web Worker migration for WebUI WASM
rendering" as a future direction. This plan executes that: run the WASM facade in
a dedicated Worker so the main thread stays responsive, with the render result
posted back asynchronously.

## Current state

- `apps/webui/src/wasmLoader.ts` — lazy-imports `@keroway/tdsl-wasm`, exposes synchronous wrappers (`compileToIr`, `renderSvg`, `renderSvgWithOptions`, `renderHtmlWithOptions`, `checkSource`, `lintSource`, `formatSource`, `lintFixSource`). All are `mod().xxx(...)` synchronous calls.
- `apps/webui/src/hooks/useWasm.ts` — initializes WASM on mount, exposes `{ wasmReady, wasmError }`.
- `apps/webui/src/hooks/useCompiler.ts` — debounced `compileAndCheck` calls the sync wrappers directly and `setState`s results.
- `apps/webui/src/hooks/useExport.ts` — calls sync wrappers on demand (user-initiated, not per-keystroke).
- Build: Vite 8 + `vite-plugin-wasm`; `base: './'`; PWA precaches `**/*.wasm`.

## Scope

**In scope**:

- Add a Worker entry (e.g. `apps/webui/src/worker/tdsl.worker.ts`) that imports and initializes `@keroway/tdsl-wasm` and handles typed request/response messages.
- A main-thread client (e.g. `apps/webui/src/worker/client.ts`) exposing **async** versions of the facade (`compileToIrAsync`, `renderSvgWithOptionsAsync`, `checkSourceAsync`, `lintSourceAsync`, `renderHtmlWithOptionsAsync`, `formatSourceAsync`, `lintFixSourceAsync`).
- Rework `useWasm` and `useCompiler` to use the async client.
- Rework `useExport` on-demand calls to await the client.
- Cancellation: supersede stale in-flight compile requests (request id / latest-wins) so fast typing does not queue a backlog.

**Out of scope**:

- Changing the WASM crate (`crates/tdsl-wasm`) API.
- Rendering algorithm changes.
- Adding a bundler other than Vite.
- Multiple workers / worker pool (single worker is sufficient; note as future work).

## Git workflow

- Suggested branch: `advisor/013-webui-wasm-worker`
- Commit message style: `perf(webui): run WASM compile/render in a Web Worker`
- Do not push or open a PR unless instructed.

## Design

- Use Vite's native worker support: `new Worker(new URL('./tdsl.worker.ts', import.meta.url), { type: 'module' })`. Confirm `vite-plugin-wasm` works inside module workers with the current Vite 8 config; if the WASM import inside the worker needs the plugin, it is already enabled globally.
- Message protocol: `{ id, op, args }` → `{ id, ok, result } | { id, ok: false, error }`. Keep a `Map<id, {resolve, reject}>` on the client.
- **Latest-wins for preview**: `useCompiler` tags each debounced request; when a newer request is issued, drop the resolution of superseded ones (ignore their results) so only the freshest render reaches state. This replaces the current purely-synchronous flow where staleness could not happen.
- `wasmReady` becomes "worker initialized" (worker posts a `ready`/`error` message after `init`).
- Preserve current UX: `isStalePreview`, diagnostics ordering (check diagnostics first, then lint, dedupe `parse_error`), and the render-error-as-diagnostic behavior in `useCompiler`.
- PWA: the worker chunk and `.wasm` must remain precached; verify the Workbox `globPatterns` still catch the worker output (it is JS, already covered by `**/*.js`).

## Steps

### Step 1: Create the worker + typed client

Add `tdsl.worker.ts` (init + op dispatch) and `client.ts` (async facade +
pending-request map + init handshake). Keep `wasmLoader.ts` temporarily for
`useExport` until Step 4, or migrate everything at once — your call, but keep
each commit green.

**Verify**: `cd apps/webui && npm run build` → worker chunk emitted, exit 0.

### Step 2: Rework `useWasm`

`useWasm` spins up the worker (or consumes a singleton client), awaits the
`ready` handshake, exposes `{ wasmReady, wasmError }` unchanged to callers.

**Verify**: `cd apps/webui && npm run lint` → exit 0.

### Step 3: Rework `useCompiler` with latest-wins cancellation

Convert `compileAndCheck` to async against the client; ignore results from
superseded requests. Preserve diagnostics merge/dedupe and `isStalePreview`
semantics exactly.

**Verify**: `cd apps/webui && npm test` → existing WebUI tests exit 0.

### Step 4: Rework `useExport` on-demand calls

`downloadJsonIr`, `downloadHtml`, `exportPdf` etc. await the client. These are
user-initiated so latency is acceptable; show a Toast if a call is slow only if
trivial (optional).

**Verify**: `cd apps/webui && npm run build` → exit 0.

### Step 5: Delete/retire `wasmLoader.ts` sync wrappers

Once all callers use the async client, remove the now-unused synchronous
wrappers (or keep the file as a thin re-export if many tests import types from
it — keep the `Diagnostic`/`LintIssue`/`RenderOptions` types available).

**Verify**: `rg -n "from '.*wasmLoader'" apps/webui/src` → only type imports remain (or none).

### Step 6: Manual performance validation

Load a large timeline (duplicate a big example many times). Confirm typing stays
smooth (no long main-thread blocking in DevTools Performance), preview updates
after debounce, diagnostics still appear, and export still works. Confirm the
PWA still works offline (worker + wasm precached).

## Test plan

- Keep existing Vitest suites green (they largely test pure helpers; if any test imported sync wrappers, adapt to async or mock the client).
- Add a client-level test for latest-wins: issue two requests, resolve them out of order, assert only the newest result is delivered.
- Manual: DevTools Performance trace before/after on a large document showing reduced main-thread long tasks; offline reload still renders.

## Done criteria

- [ ] WASM `init` + all compile/render/lint/format calls run in a Worker.
- [ ] Preview uses latest-wins so fast typing cannot backlog or show a stale-newer race.
- [ ] `wasmReady` / `wasmError` / `isStalePreview` / diagnostics behavior preserved.
- [ ] PWA offline still renders (worker + `.wasm` precached).
- [ ] `cd apps/webui && npm run lint && npm test && npm run build` all exit 0.
- [ ] `plans/README.md` status row updated; `docs/webui-design.md` future-direction note updated to "implemented".

## STOP conditions

Stop and report if:

- `vite-plugin-wasm` cannot instantiate `@keroway/tdsl-wasm` inside a module worker under the current Vite 8 setup (capture the exact error; may need a worker-specific plugin/config).
- The async conversion forces a redesign of how `useSvgInteractions` consumes `svgContent` (should be unaffected — it consumes the resulting string).
- Offline/PWA precaching no longer covers the worker or wasm chunk.

## Maintenance notes

Single worker is enough today. If future timelines are large enough to make even
a single worker's render latency noticeable, a worker pool or incremental render
is the next step — record as future work, do not build preemptively.
