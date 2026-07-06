<!-- markdownlint-disable MD013 MD060 -->

# Plan 011: Complete the WebUI i18n migration (hardcoded Japanese strings remain)

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving on. If anything in "STOP conditions" occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 1c22dac..HEAD -- apps/webui/src`
> If in-scope files changed since this plan was written, re-run the audit command in "Current state" before proceeding.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: LOW
- **Depends on**: none (builds on the Plan 009 typed i18n foundation, already merged)
- **Category**: bug / incomplete feature
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #589

## Why this matters

Plan 009 (`feat(webui): add typed i18n foundation`, #531) introduced a typed
`createTranslator` / `t()` system with `ja` and `en` dictionaries and a
locale setting. **The migration was only partially completed.** `App.tsx`
routes some strings through `t()`, but the bulk of user-facing strings in
components, hooks, and editor modules are **still hardcoded Japanese**. An
English-locale user therefore sees a mix of English and Japanese — toasts,
export messages, confirm dialogs, tooltips, settings labels, and editor
completion detail are all still Japanese.

This is a genuine "half-finished" area: the infrastructure exists but is not
applied. Finishing it makes the `en` locale actually usable and prevents the
two dictionaries from drifting further as new strings are added.

## Current state

Audit command (run to reproduce and to re-scope after drift):

```bash
cd apps/webui/src
rg -n '[ぁ-んァ-ヶ一-龠]' --glob '*.ts' --glob '*.tsx' \
  -g '!*.test.*' -g '!lib/i18n.ts' -g '!examples.ts' -g '!lib/initialSource.ts' \
  -g '!gallery-meta.ts' -g '!lang-tdsl/hover.ts' \
  | rg -v '^\s*//|^\s*\*'
```

At planning time this reports hardcoded Japanese (user-facing, excluding
comments) concentrated in — in rough count order:

- `hooks/useExport.ts` — toast messages for every export/copy action, and the
  `window.confirm` text in `downloadJsonIr` and the PDF hint toast.
- `components/Toolbar.tsx` — button labels / titles / menu items.
- `editor/completions.ts` — completion `detail`/`info` strings.
- `editor/shortcuts.ts` — shortcut descriptions.
- `components/PreviewPanel.tsx` — control labels / aria text.
- `components/SettingsModal.tsx` — setting labels / help text.
- `editor/extensions.ts` — a few inline strings.
- `hooks/useHistorySnapshots.ts`, `history.ts` — snapshot labels
  (e.g. `テンプレートロード前`, `ファイルオープン前`).
- `App.tsx` — `ファイルを開けませんでした: ${msg}` and snapshot labels.
- `components/StatusBar.tsx`, `MobileTabBar.tsx`, `HistoryModal.tsx`,
  `Toast.tsx`, `DiagnosticsPanel.tsx` — a small number each.

The i18n API (already present in `lib/i18n.ts`):

- `type Locale = 'ja' | 'en'`, `SUPPORTED_LOCALES`, `DEFAULT_LOCALE = 'ja'`.
- `createTranslator(locale)` → `Translator` with `t(key)` and `t.fmt(key, params)`.
- `dictionaries: Record<Locale, Dictionary> = { ja, en }`.
- A test (`lib/i18n.test.ts`) enforces that **every key exists in every locale**.

How `App.tsx` already consumes it (pattern to follow): `const t = createTranslator(settings.locale)` then `t('key')` / `t.fmt('key', { msg })`.

## Scope

**In scope** (migrate hardcoded JA → `t()` keys, add matching `ja`/`en` entries):

- `apps/webui/src/hooks/useExport.ts`
- `apps/webui/src/hooks/useHistorySnapshots.ts`
- `apps/webui/src/history.ts` (snapshot label constants)
- `apps/webui/src/components/Toolbar.tsx`
- `apps/webui/src/components/PreviewPanel.tsx`
- `apps/webui/src/components/SettingsModal.tsx`
- `apps/webui/src/components/StatusBar.tsx`
- `apps/webui/src/components/MobileTabBar.tsx`
- `apps/webui/src/components/HistoryModal.tsx`
- `apps/webui/src/components/Toast.tsx`
- `apps/webui/src/components/DiagnosticsPanel.tsx`
- `apps/webui/src/editor/completions.ts`
- `apps/webui/src/editor/shortcuts.ts`
- `apps/webui/src/editor/extensions.ts`
- `apps/webui/src/App.tsx` (remaining hardcoded strings)
- `apps/webui/src/lib/i18n.ts` (new keys in both `ja` and `en`)

**Out of scope**:

- DSL example content (`examples.ts`, `initialSource.ts`, `gallery-meta.ts`) — this is sample DSL, not UI chrome. Leave as-is (already excluded from the audit).
- Adding a **third** locale.
- Changing `DEFAULT_LOCALE`.
- `lang-tdsl/hover.ts` DSL keyword documentation (decide explicitly — see Step 4).

## Git workflow

- Suggested branch: `advisor/011-webui-i18n-complete`
- Because the surface is broad, land it in **reviewable slices** (e.g. one commit per file group: hooks, components, editor). Keep each commit green.
- Commit message style: `feat(webui): route <area> strings through i18n`
- Do not push or open a PR unless instructed.

## Threading `t` into non-component code

Components can call `createTranslator(settings.locale)` directly or receive a
`t` prop. Hooks and plain modules (`useExport`, `history.ts`,
`editor/completions.ts`) do **not** own the locale, so pass the translator in:

1. In `App.tsx`, build `const t = createTranslator(settings.locale)` once (App may already do this) and pass `t` down to `useExport(...)`, `useHistorySnapshots(...)`, and to editor extension factories.
2. For editor modules that are constructed once (completions/shortcuts), accept `t` as a factory argument and rebuild the extension when locale changes, mirroring how other locale-dependent editor state is handled. If rebuilding on locale change is not already wired, prefer having these read from a stable translator captured at construction and document that a locale switch requires the existing editor-remount path (check whether `SettingsModal` locale change already remounts / re-derives extensions before assuming).

Keep the key naming consistent with existing conventions in `lib/i18n.ts`
(group by area, e.g. `exportSvgCopied`, `exportPngCopyFailed`,
`historySnapshotBeforeTemplate`, `toolbarFormat`, etc.).

## Steps

### Step 1: Migrate the hooks (`useExport`, `useHistorySnapshots`, `history.ts`)

Replace every hardcoded JA toast/label with `t()` / `t.fmt()`. Add the `ja`
(verbatim current text) and `en` (translated) entries to both dictionaries.
For the `downloadJsonIr` `window.confirm` text and the PDF hint, add keys too
(the confirm dialog itself is addressed separately in Plan 012 — here just
move the *string* into i18n so both plans compose).

**Verify**: `cd apps/webui && npm test` → `lib/i18n.test.ts` passes (proves key parity), exit 0.

### Step 2: Migrate components

Work file-by-file: `Toolbar`, `PreviewPanel`, `SettingsModal`, `StatusBar`,
`MobileTabBar`, `HistoryModal`, `Toast`, `DiagnosticsPanel`. Prefer `aria-label`
/ `title` strings go through `t()` too (accessibility parity across locales).

**Verify after each file**: `cd apps/webui && npm run lint` → exit 0.

### Step 3: Migrate editor modules (`completions.ts`, `shortcuts.ts`, `extensions.ts`)

Thread `t` into the completion/shortcut factories. Confirm the completion
`detail`/`info` and shortcut descriptions render in the active locale.

**Verify**: `cd apps/webui && npm run build` → exit 0 (TypeScript proves all keys resolve).

### Step 4: Decide on `lang-tdsl/hover.ts` explicitly

`hover.ts` documents DSL keywords. Either (a) leave DSL keyword docs in Japanese
as reference content (out of scope), or (b) migrate them too. **Default: leave
out of scope** and note the decision in the PR description. Do not silently skip.

### Step 5: Re-run the audit to prove completeness

Re-run the audit command from "Current state". The only remaining matches should
be intentional (comments, DSL sample content, and — if you chose 4a — `hover.ts`).

**Verify**: audit output contains no user-facing UI chrome strings outside the documented exclusions.

### Step 6: Final gates

**Verify**:

- `cd apps/webui && npm run lint` → exit 0.
- `cd apps/webui && npm test` → exit 0 (i18n key-parity test green).
- `cd apps/webui && npm run build` → exit 0.

## Test plan

- `lib/i18n.test.ts` already fails if any key is missing from either locale — this is your primary safety net; every new key must be added to both `ja` and `en`.
- Add a focused test if practical: assert a representative migrated string differs between `ja` and `en` (guards against copy-paste leaving Japanese in the `en` dict).
- Manual: switch locale to English in Settings and confirm toasts (export SVG/PNG, copy Markdown), Toolbar labels, Settings labels, and at least one completion detail all render in English.

## Done criteria

- [ ] All in-scope files route user-facing strings through `t()` / `t.fmt()`.
- [ ] Every new key exists in both `ja` and `en` (i18n test green).
- [ ] English locale shows no stray Japanese UI chrome (per Step 5 audit).
- [ ] `hover.ts` decision documented in the PR description.
- [ ] `cd apps/webui && npm run lint && npm test && npm run build` all exit 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- Threading `t` into editor modules requires a large refactor of how CodeMirror extensions are constructed/rebuilt on locale change (report the blocker; a smaller-scope slice excluding editor modules may be preferable).
- The `en` translations require product/wording decisions you cannot make confidently (list the ambiguous strings and ask).
- Migrating a string would change behavior (e.g. a label used as a stable key/id elsewhere).

## Maintenance notes

After this lands, add a lightweight guard so new hardcoded Japanese UI strings
are caught early — e.g. document the audit `rg` command in `apps/webui/README.md`
under a "i18n" section, or add it as an optional CI check. New UI strings must be
added to `lib/i18n.ts` in both locales, never inlined.
