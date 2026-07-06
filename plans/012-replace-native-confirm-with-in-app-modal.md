<!-- markdownlint-disable MD013 MD060 -->

# Plan 012: Replace native `window.confirm` dialogs with an in-app, accessible, translatable modal

> **Executor instructions**: Follow this plan step by step. Run every verification command. If anything in "STOP conditions" occurs, stop and report. When done, update the status row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 1c22dac..HEAD -- apps/webui/src/hooks/useExport.ts apps/webui/src/App.tsx`

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: pairs with Plan 011 (i18n). If 011 lands first, reuse its keys; if not, add keys as part of this plan.
- **Category**: UX / accessibility / i18n consistency
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #590

## Why this matters

The WebUI uses the browser-native `window.confirm(...)` in two flows:

- `hooks/useExport.ts` → `downloadJsonIr`: warns that WASM does not resolve
  `import` / `map`, so the JSON IR omits imported items, and asks whether to
  save the static-only IR.
- `App.tsx` → `handleLintFix`: confirms applying auto-fixes (with a stronger
  warning when the source contains comments, since lint-fix may drop them).

Native `window.confirm` has three problems consistent with the app's own
quality bar (the rest of the app uses styled Toasts, a focus-trapped
`SettingsModal`/`HistoryModal`, and typed i18n):

1. **Not translatable** — the text is hardcoded Japanese, bypassing i18n (Plan 011).
2. **Not styled / inconsistent** — a system dialog breaks the app's visual language and dark theme.
3. **Accessibility** — the app already ships `useFocusTrap` and modal patterns; native confirm is inconsistent with the a11y work done in Plan 008.

## Current state

- `hooks/useExport.ts`, `downloadJsonIr`: `const proceed = window.confirm('import / map ブロックは …')`.
- `App.tsx`, `handleLintFix`: `if (!window.confirm(warning)) return`, where `warning` is `appLintFixCommentConfirm` / `appLintFixConfirm` (these two are already i18n keys, but delivered through native confirm).
- Existing modal infrastructure to reuse: `components/SettingsModal.tsx`, `components/HistoryModal.tsx`, `hooks/useFocusTrap.ts`, `hooks/useOutsideClick.ts`.

## Scope

**In scope**:

- Add a reusable `ConfirmModal` (or `useConfirm` hook returning a promise) under `apps/webui/src/components/` + `apps/webui/src/hooks/`.
- Wire `downloadJsonIr` and `handleLintFix` to it.
- i18n keys for the confirm bodies/titles/buttons (both `ja` and `en`).

**Out of scope**:

- Replacing Toasts (they are non-blocking and appropriate as-is).
- Any other flow that does not currently use `window.confirm`.
- Changing the *logic* of when confirmation is required.

## Git workflow

- Suggested branch: `advisor/012-in-app-confirm-modal`
- Commit message style: `feat(webui): replace native confirm with in-app modal`
- Do not push or open a PR unless instructed.

## Design

Prefer a promise-based `useConfirm()` so call sites stay linear:

```ts
const confirm = useConfirm() // returns (opts) => Promise<boolean>
// ...
if (!(await confirm({ title, body, confirmLabel, cancelLabel, tone: 'warn' }))) return
```

Implementation notes:

- Render a single `<ConfirmModal>` near the app root, controlled by state held in the `useConfirm` provider/hook (a small context, or lift state into `App.tsx`).
- Reuse `useFocusTrap` and `useOutsideClick`, mirror `SettingsModal` markup/roles (`role="dialog"`, `aria-modal="true"`, labelled title, initial focus on the safe/cancel action).
- `Esc` cancels; `Enter` confirms only when focus is on the confirm button (do not auto-confirm on Enter globally).
- Keep it dependency-free (no new packages).

Because `useExport` is a hook that returns handlers, thread the `confirm`
function into `useExport(...)` (new argument) rather than calling a global.
`handleLintFix` lives in `App.tsx`, which can call `confirm` directly.

## Steps

### Step 1: Build `useConfirm` + `ConfirmModal`

Add the hook and component. Add i18n keys: title/body/confirm/cancel for the
JSON-IR-incomplete case and (reuse existing `appLintFixConfirm` /
`appLintFixCommentConfirm` bodies) plus generic `confirmProceed` / `confirmCancel`
button labels. Add all keys to both `ja` and `en`.

**Verify**: `cd apps/webui && npm test` → i18n key-parity test green, exit 0.

### Step 2: Wire `downloadJsonIr`

Replace `window.confirm(...)` with `await confirm({...})`. `downloadJsonIr`
becomes `async`; update the `ExportApi` type and the Toolbar/menu call site to
handle the promise (fire-and-forget with error handling is fine — it already
shows toasts).

**Verify**: `cd apps/webui && npm run build` → exit 0 (type change propagates).

### Step 3: Wire `handleLintFix`

Make `handleLintFix` `async` and `await confirm(...)`. Ensure the editor
dispatch still happens only on confirmation. Keep the comment-present stronger
warning by choosing the body key based on `hadComment`.

**Verify**: `cd apps/webui && npm run lint` → exit 0.

### Step 4: Final gates + manual a11y check

**Verify**:

- `cd apps/webui && npm run lint && npm test && npm run build` → all exit 0.
- Manual: trigger both flows; confirm focus is trapped, `Esc` cancels, the dialog is themed, and (with locale=en) the text is English.

## Test plan

- Unit-test `useConfirm` resolution: confirm resolves `true`, cancel/Esc resolves `false`. Mock nothing external.
- i18n parity test covers the new keys.
- Manual keyboard test: Tab cycles within the dialog only; focus returns to the triggering control on close.

## Done criteria

- [ ] No `window.confirm` remains in `apps/webui/src` (`rg -n 'window\.confirm' apps/webui/src` → no matches).
- [ ] Both flows use the in-app modal with focus trap + Esc-to-cancel.
- [ ] Confirm text is translatable (keys in both locales).
- [ ] `cd apps/webui && npm run lint && npm test && npm run build` all exit 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- Making `downloadJsonIr` / `handleLintFix` async forces broad signature churn across many unrelated call sites.
- A promise-based confirm cannot integrate cleanly with the existing render tree without a new state-management dependency (report; a simpler controlled-modal-in-App approach is acceptable).

## Maintenance notes

After this lands, add a lint guard or a note in `apps/webui/README.md` that
`window.confirm` / `window.alert` are disallowed — use `useConfirm` / Toast.
