<!-- markdownlint-disable MD013 MD060 -->

# Plan 007: Preserve WebUI preview render options in HTML and PDF export

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 3176e9d..HEAD -- apps/webui/src/App.tsx apps/webui/src/hooks/useExport.ts apps/webui/src/wasmLoader.ts apps/webui/src/lib/settings.ts apps/webui/src/hooks/useCompiler.ts`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none; benefits from Plan 005 if color_map parity is also desired
- **Category**: bug
- **Planned at**: commit `3176e9d`, 2026-06-28

## Why this matters

The WebUI lets users change preview orientation, grid density, SVG theme, and scale. The preview uses those settings, but HTML export and browser print-to-PDF currently call the default HTML renderer, so saved output can differ from what the user is looking at. This plan makes export use the same render options as preview for a predictable WYSIWYG workflow.

## Current state

Relevant files:

- `apps/webui/src/App.tsx` — owns settings and creates `renderOpts` for preview.
- `apps/webui/src/hooks/useCompiler.ts` — uses `renderSvgWithOptions` for preview.
- `apps/webui/src/hooks/useExport.ts` — exports `.tdsl`, JSON IR, SVG, HTML, PNG, and PDF.
- `apps/webui/src/wasmLoader.ts` — already exposes `renderHtmlWithOptions`.

Current preview option flow (`apps/webui/src/App.tsx:53-62`):

```ts
const renderOpts = useMemo(
  () => ({ orientation: settings.svgOrientation, grid: settings.svgGrid, theme: settings.svgTheme }),
  [settings.svgOrientation, settings.svgGrid, settings.svgTheme]
)
const { svgContent, diagnostics, diagnosticsRef, isStalePreview } = useCompiler(source, wasmReady, settings.scale, renderOpts)
const svg = useSvgInteractions(svgContent, editorViewRef)
const { splitRatio, mainRef, handleDividerMouseDown } = useSplitPane()
const exportApi = useExport(source, svgContent, settings.pngWhiteBg, showToast)
```

Current export hook signature (`apps/webui/src/hooks/useExport.ts:20-25`):

```ts
export function useExport(
  source: string,
  svgContent: string,
  pngWhiteBg: boolean,
  showToast: (message: string, variant?: ToastVariant) => void,
): ExportApi {
```

Current HTML/PDF export uses defaults (`apps/webui/src/hooks/useExport.ts:97-105`, `:114-124`):

```ts
function downloadHtml() {
  if (!svgContent) return
  try {
    const html = renderHtml(source)
    triggerDownload(new Blob([html], { type: 'text/html' }), 'timeline.html')
  } catch {
    // keep silent — errors are already shown in diagnostics
  }
}

function exportPdf() {
  if (!svgContent) return
  let html: string
  try {
    html = renderHtml(source)
```

WASM loader already has an options-aware function (`apps/webui/src/wasmLoader.ts:82-90`):

```ts
export function renderHtmlWithOptions(source: string, opts: RenderOptions = {}): string {
  const jsOpts = new (mod().JsRenderOptions)()
  if (opts.orientation !== undefined) jsOpts.orientation = opts.orientation
  if (opts.grid !== undefined) jsOpts.grid = opts.grid
  if (opts.theme !== undefined) jsOpts.theme = opts.theme
```

Repo conventions:

- WebUI settings are centralized in `apps/webui/src/lib/settings.ts` and exposed through `useSettings`.
- Keep export logic inside `useExport`; keep `App.tsx` as orchestration only.
- WebUI verification is `npm run lint`, `npm test`, and `npm run build` from `apps/webui`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| WebUI tests | `cd apps/webui && npm test` | exit 0 |
| WebUI lint | `cd apps/webui && npm run lint` | exit 0 |
| WebUI build | `cd apps/webui && npm run build` | exit 0 |
| Optional WASM tests if Rust touched | `cargo test -p tdsl-wasm` | exit 0 |

## Scope

**In scope**:

- `apps/webui/src/App.tsx`
- `apps/webui/src/hooks/useExport.ts`
- `apps/webui/src/wasmLoader.ts` only if imports/types need adjustment
- Tests under `apps/webui/src/` if you add pure helper coverage

**Out of scope**:

- Changing Rust renderer semantics.
- Adding new render settings UI.
- PNG export changes; PNG already converts the current `svgContent` shown in preview.
- JSON IR export; it is data, not rendered output.

## Git workflow

- Suggested branch: `advisor/007-webui-export-options`
- Commit message style: `fix(webui): preserve preview options in HTML export`
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Pass render options into useExport

In `apps/webui/src/hooks/useExport.ts`:

1. Import `renderHtmlWithOptions` instead of `renderHtml`, and import the `RenderOptions` type.
2. Add a `renderOpts: RenderOptions` parameter to `useExport`.
3. Use `renderHtmlWithOptions(source, renderOpts)` in `downloadHtml` and `exportPdf`.

In `apps/webui/src/App.tsx`, update the call to:

```ts
const exportApi = useExport(source, svgContent, settings.pngWhiteBg, renderOpts, showToast)
```

Keep the hook argument order clear; if you prefer to avoid positional mistakes, convert `useExport` to accept an object parameter. If you do that, update all call sites in the same commit.

**Verify**: `cd apps/webui && npm run lint` → exit 0, no hook dependency or unused import errors.

### Step 2: Include the settings that export can actually honor

Today `renderOpts` contains orientation, grid, and theme. Confirm these are the only settings supported by `RenderOptions` in `apps/webui/src/wasmLoader.ts` for HTML rendering.

If export should also honor future `showTable`, `showEventLabels`, or `laneHeight`, extend `renderOpts` in `App.tsx` only if those settings already exist in `Settings`. Do not add new settings in this plan.

**Verify**: `cd apps/webui && npm run build` → TypeScript confirms the option shape.

### Step 3: Add a lightweight regression test if practical

If `useExport` can be tested without loading real WASM, add a small pure helper such as:

```ts
export function renderHtmlForExport(source: string, renderOpts: RenderOptions): string {
  return renderHtmlWithOptions(source, renderOpts)
}
```

Then mock `renderHtmlWithOptions` in a Vitest test and assert `downloadHtml`/PDF path passes the exact `renderOpts`. If mocking the hook is too invasive with the current test setup, skip this step and rely on TypeScript/build plus a manual checklist in the PR description.

Do not install new test libraries in this plan.

**Verify**: `cd apps/webui && npm test` → exit 0.

### Step 4: Run final WebUI gates

**Verify**:

- `cd apps/webui && npm run lint` → exit 0.
- `cd apps/webui && npm test` → exit 0.
- `cd apps/webui && npm run build` → exit 0.

## Test plan

- Prefer a Vitest test around a small pure export helper if it can be added without new dependencies.
- Manual verification after build: set SVG preview to vertical + grid + dark theme, export HTML, open the saved HTML, and confirm orientation/grid/theme match the preview.
- PNG export does not need changes because it uses current `svgContent`.

## Done criteria

- [ ] `downloadHtml` uses `renderHtmlWithOptions(source, renderOpts)`.
- [ ] `exportPdf` prints HTML generated with the same `renderOpts`.
- [ ] `cd apps/webui && npm run lint` exits 0.
- [ ] `cd apps/webui && npm test` exits 0.
- [ ] `cd apps/webui && npm run build` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- `renderHtmlWithOptions` is missing or its generated HTML ignores orientation/grid/theme.
- The fix requires adding new user settings or changing Rust renderer APIs.
- TypeScript changes force a broad rewrite of `useExport` unrelated to render options.
- WebUI tests require installing new packages.

## Maintenance notes

Whenever a new preview render setting is added, update both preview and export option plumbing in the same PR. Reviewers should compare `renderOpts` construction in `App.tsx` against the fields consumed in `wasmLoader.ts`.
