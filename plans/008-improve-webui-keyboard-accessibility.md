<!-- markdownlint-disable MD013 MD060 -->

# Plan 008: Improve WebUI keyboard accessibility for preview and split pane controls

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 3176e9d..HEAD -- apps/webui/src/App.tsx apps/webui/src/components/PreviewPanel.tsx apps/webui/src/hooks/useSvgInteractions.ts apps/webui/src/hooks/useSplitPane.ts apps/webui/src/App.css`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW-MED
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `3176e9d`, 2026-06-28

## Why this matters

The rendered SVG already exposes timeline items as focusable groups with `aria-label`, but the WebUI only handles mouse click/drag interactions. Keyboard users can tab to SVG items without being able to activate the detail panel or editor jump with Enter/Space. The editor/preview split divider is also a mouse-only `div`, so keyboard users cannot resize panes. This plan adds keyboard activation and ARIA semantics without changing the visual design.

## Current state

Relevant files:

- `crates/tdsl-render/src/svg.rs` — emits focusable SVG item groups.
- `apps/webui/src/components/PreviewPanel.tsx` — preview DOM and event handlers.
- `apps/webui/src/hooks/useSvgInteractions.ts` — owns preview item selection and editor jump.
- `apps/webui/src/hooks/useSplitPane.ts` — owns split ratio state and drag behavior.
- `apps/webui/src/App.tsx` — renders the split divider.

Current SVG item emission (`crates/tdsl-render/src/svg.rs:370`) already makes items keyboard-focusable:

```rust
r#"  <g class=\"tdsl-item tdsl-item-span\" role=\"group\" aria-label=\"{aria_label}\" tabindex=\"0\" data-tdsl-tooltip=\"{tip_attr}\"{data_attrs}>..."#,
```

Current preview handlers are mouse-only (`apps/webui/src/components/PreviewPanel.tsx:197-203`):

```tsx
<div
  ref={previewRef}
  className={`preview-pane${cursorGrab ? ' grabbing' : ''}`}
  onMouseDown={onMouseDown}
  onMouseMove={onMouseMove}
  onMouseUp={onMouseUp}
  onMouseLeave={onMouseLeave}
  onDoubleClick={onDoubleClick}
  onClick={onClick}
>
```

Current split divider is a mouse-only `div` (`apps/webui/src/App.tsx:251-256`):

```tsx
<div
  className="split-divider"
  onMouseDown={handleDividerMouseDown}
  title="ドラッグして分割幅を調整"
  style={previewFullscreen ? { display: 'none' } : undefined}
/>
```

Current split hook exposes only mouse down (`apps/webui/src/hooks/useSplitPane.ts:9-13`, `:50-58`):

```ts
export type SplitPaneApi = {
  splitRatio: number
  mainRef: RefObject<HTMLElement | null>
  handleDividerMouseDown: (e: MouseEvent<HTMLDivElement>) => void
}

function handleDividerMouseDown(e: MouseEvent<HTMLDivElement>) { /* drag setup */ }

return { splitRatio, mainRef, handleDividerMouseDown }
```

Repo conventions:

- Keep interaction state in hooks (`useSvgInteractions`, `useSplitPane`) rather than in `App.tsx`.
- Avoid adding dependencies unless needed; current WebUI tests use Vitest without Testing Library.
- Modal focus trapping already exists in `apps/webui/src/hooks/useFocusTrap.ts`; do not rewrite it.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| WebUI lint | `cd apps/webui && npm run lint` | exit 0 |
| WebUI tests | `cd apps/webui && npm test` | exit 0 |
| WebUI build | `cd apps/webui && npm run build` | exit 0 |
| Optional renderer tests | `cargo test -p tdsl-render svg` | exit 0 if SVG attributes are touched |

## Scope

**In scope**:

- `apps/webui/src/components/PreviewPanel.tsx`
- `apps/webui/src/hooks/useSvgInteractions.ts`
- `apps/webui/src/hooks/useSplitPane.ts`
- `apps/webui/src/App.tsx`
- `apps/webui/src/App.css` for focus styles if needed
- Small Vitest helper tests if you extract pure keyboard helpers

**Out of scope**:

- Changing SVG generation unless absolutely necessary; it already emits focusable groups.
- Installing `@testing-library/*`, `axe`, or other new a11y test packages.
- Redesigning the preview controls visually.
- Implementing full screen-reader QA beyond semantic improvements and keyboard support.

## Git workflow

- Suggested branch: `advisor/008-webui-keyboard-a11y`
- Commit message style: `fix(webui): add keyboard access to preview controls`
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Add keyboard activation for SVG items

In `apps/webui/src/hooks/useSvgInteractions.ts`:

1. Extract the shared item-selection logic from `handlePreviewClick` into a helper such as `activatePreviewTarget(target: Element | null)`.
2. Keep drag suppression for mouse clicks only.
3. Add `handlePreviewKeyDown(e)` to the returned API. For `Enter` or `Space`, find the focused/target item with:

```ts
const target = (e.target as Element).closest<HTMLElement>('[data-label]')
```

1. If a target exists, `preventDefault()` and call the same activation helper used by click. The behavior should match mouse click: open detail panel and, if `data-line` exists, jump/focus CodeMirror.
2. Optionally support `Escape` to clear `selectedItem` when the preview pane has focus.

Update `SvgInteractionsApi` type accordingly.

**Verify**: `cd apps/webui && npm run lint` → no type or exhaustive-deps errors.

### Step 2: Wire keyboard handler in PreviewPanel

In `apps/webui/src/components/PreviewPanel.tsx`:

1. Add an `onKeyDown` prop type.
2. Attach it to the same preview pane `div` that has click/mouse handlers.
3. Add `aria-label="年表プレビュー"` to the preview pane if it does not already have an accessible name.
4. For non-modal floating panels (`legend-panel`, `filter-panel`, `detail-panel`), add minimal semantic labels where useful, e.g. `aria-label="凡例"` or `aria-label="選択中アイテムの詳細"`.
5. Add `aria-label` to icon-only close buttons such as detail close if missing.

In `apps/webui/src/App.tsx`, pass `onKeyDown={svg.handlePreviewKeyDown}`.

**Verify**: `cd apps/webui && npm run build` → TypeScript passes.

### Step 3: Make the split divider keyboard-resizable

In `apps/webui/src/hooks/useSplitPane.ts`:

1. Extend `SplitPaneApi` with `handleDividerKeyDown` and a direct setter helper if useful.
2. Implement ArrowLeft/ArrowRight to decrease/increase `splitRatio` by a small step such as `0.02`.
3. Implement Home/End to set `SPLIT_RATIO_MIN` / `SPLIT_RATIO_MAX`.
4. Clamp with existing `SPLIT_RATIO_MIN` and `SPLIT_RATIO_MAX`.
5. Persist updated ratio to `localStorage` using the existing `SPLIT_RATIO_KEY` best-effort pattern.

In `apps/webui/src/App.tsx`, update the divider:

- `role="separator"`
- `aria-orientation="vertical"`
- `aria-valuemin={Math.round(SPLIT_RATIO_MIN * 100)}` or equivalent if constants are exported/imported
- `aria-valuemax={...}`
- `aria-valuenow={Math.round(splitRatio * 100)}`
- `tabIndex={0}`
- `onKeyDown={handleDividerKeyDown}`

If importing constants into `App.tsx` creates awkward coupling, return `splitRatioMin`/`splitRatioMax` from the hook.

**Verify**: `cd apps/webui && npm run lint` → exit 0.

### Step 4: Add visible focus styles if missing

In `apps/webui/src/App.css`, ensure keyboard focus is visible for:

- `.split-divider:focus-visible`
- `.preview-pane :focus-visible` or `.svg-container [tabindex="0"]:focus-visible`
- `.detail-close:focus-visible` if not covered by global button styles

Use existing theme variables and do not introduce a new design system.

**Verify**: `cd apps/webui && npm run build` → CSS and TS build pass.

### Step 5: Add lightweight tests where possible

Without adding new dependencies, prefer extracting a pure clamp/step helper from `useSplitPane` and testing it with Vitest:

- ArrowRight from 0.5 increases by step.
- ArrowLeft decreases by step.
- Home/End clamp to min/max.

If testing DOM keyboard activation is too heavy with current dependencies, leave a manual QA checklist in the PR description instead of installing libraries.

**Verify**: `cd apps/webui && npm test` → exit 0.

## Test plan

- Vitest unit test for split ratio keyboard helper if extracted.
- Manual keyboard QA after build:
  - Tab to a timeline item in the SVG; press Enter and Space; detail panel opens and editor jumps when source line metadata exists.
  - Press Escape in preview; selected item clears if implemented.
  - Tab to split divider; ArrowLeft/ArrowRight resize panes; Home/End clamp to min/max; focus ring is visible.
  - Existing mouse drag/click interactions still work.

## Done criteria

- [ ] SVG preview items can be activated with Enter or Space.
- [ ] Split divider is a keyboard-focusable ARIA separator and responds to arrow keys.
- [ ] Visible focus styles exist for new keyboard targets.
- [ ] `cd apps/webui && npm run lint` exits 0.
- [ ] `cd apps/webui && npm test` exits 0.
- [ ] `cd apps/webui && npm run build` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- SVG items no longer have stable `data-label`/`data-line` attributes to activate.
- The keyboard fix requires changing renderer output in a way that breaks snapshots.
- Current WebUI test tooling cannot run after a minimal helper test and the failure is unrelated to your code.
- You need to add new dependencies to test this properly.

## Maintenance notes

This is not a full a11y certification. It creates a keyboard-accessible baseline for the most obvious mouse-only interactions. Future UI components should expose keyboard behavior at the same time they add mouse behavior; reviewers should reject new icon-only buttons without `aria-label`.
