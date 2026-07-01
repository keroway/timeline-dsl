<!-- markdownlint-disable MD013 MD060 -->

# Plan 009: Add a typed i18n foundation for WebUI strings

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 3176e9d..HEAD -- apps/webui/src/App.tsx apps/webui/src/components apps/webui/src/hooks apps/webui/src/lib/settings.ts apps/webui/src/App.css apps/webui/src/*.test.ts`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: LOW-MED
- **Depends on**: none
- **Category**: direction
- **Planned at**: commit `3176e9d`, 2026-06-28

## Why this matters

Timeline DSL has English and Japanese documentation and a public WebUI, but WebUI copy is currently embedded directly in React components. Adding English support later would require broad, error-prone edits across toolbar, settings, preview, history, diagnostics, and toast text. This plan introduces a typed i18n foundation and migrates the main shell strings first, creating a pattern future UI work can follow without forcing a complete translation of every string in one PR.

## Current state

Relevant files:

- `apps/webui/src/App.tsx` — top-level shell and toast messages.
- `apps/webui/src/components/Toolbar.tsx` — many visible menu/action labels.
- `apps/webui/src/components/SettingsModal.tsx` — settings labels and options.
- `apps/webui/src/components/PreviewPanel.tsx` — preview controls, filter/detail panel labels.
- `apps/webui/src/components/HistoryModal.tsx` and `GalleryModal.tsx` — modal labels and messages.
- `apps/webui/src/lib/settings.ts` — persisted settings; suitable place to add a locale preference if no better file exists.

Examples of hardcoded Japanese UI strings:

`apps/webui/src/components/Toolbar.tsx:58-64`:

```tsx
<button
  className="btn"
  onClick={() => setFileMenuOpen((v) => !v)}
  title="ファイル操作"
>
  ファイル ▾
</button>
```

`apps/webui/src/components/SettingsModal.tsx:70-77`:

```tsx
<div className="settings-section">
  <div className="settings-label">フォントサイズ</div>
  <select
    className="toolbar-select"
    value={fontSize}
```

`apps/webui/src/components/PreviewPanel.tsx:170-175`:

```tsx
<button
  className="filter-reset-btn"
  onClick={() => setFilterState({ hiddenLanes: new Set(), tagSearch: '' })}
>
  リセット
</button>
```

`apps/webui/src/App.tsx:103-119`:

```ts
showToast(`整形に失敗しました: ${msg}`, 'error')
// ...
showToast('既に整形済みです', 'info')
// ...
showToast('整形しました', 'success')
```

Repo conventions:

- Keep settings in `apps/webui/src/lib/settings.ts` and persist via the existing localStorage pattern.
- Components are small and prop-driven; keep translation access explicit rather than adding global mutable state.
- Existing WebUI tests are Vitest tests such as `apps/webui/src/history.test.ts`, `share.test.ts`, and `gallery-meta.test.ts`.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| WebUI tests | `cd apps/webui && npm test` | exit 0; i18n dictionary tests pass |
| WebUI lint | `cd apps/webui && npm run lint` | exit 0 |
| WebUI build | `cd apps/webui && npm run build` | exit 0 |

## Scope

**In scope**:

- New `apps/webui/src/i18n.ts` or `apps/webui/src/lib/i18n.ts`
- New `apps/webui/src/i18n.test.ts` or `apps/webui/src/lib/i18n.test.ts`
- `apps/webui/src/lib/settings.ts` and `apps/webui/src/hooks/useSettings.ts` if locale is persisted
- Main user-facing shell components: `Toolbar.tsx`, `SettingsModal.tsx`, `PreviewPanel.tsx`, `HistoryModal.tsx`, `GalleryModal.tsx`, `StatusBar.tsx`, `DiagnosticsPanel.tsx` if touched by migrated strings
- `apps/webui/src/App.tsx` for passing translator/locale and replacing top-level toast strings
- `apps/webui/src/App.css` only for locale selector styling if needed

**Out of scope**:

- Translating CodeMirror language hover text, completions, DSL examples, or generated SVG labels.
- Translating Rust CLI diagnostics.
- Adding external i18n libraries.
- Rewriting all WebUI copy in one PR if it becomes too large; main shell + settings + preview controls are enough for this foundation.

## Git workflow

- Suggested branch: `advisor/009-webui-i18n-foundation`
- Commit message style: `feat(webui): add typed i18n foundation`
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Create a typed dictionary module

Add `apps/webui/src/lib/i18n.ts` (or `src/i18n.ts`; pick one and use it consistently) with:

- `export type Locale = 'ja' | 'en'`
- `export const DEFAULT_LOCALE: Locale = 'ja'`
- A `messages` object containing `ja` and `en` dictionaries.
- A `MessageKey` type derived from the Japanese dictionary keys.
- A `createTranslator(locale: Locale)` function returning `t(key, params?)`.
- Simple interpolation for params used by existing strings, e.g. `{count}` or `{message}`.

Use flat keys at first, such as:

```ts
toolbar.file
toolbar.openFile
toolbar.export
settings.title
preview.reset
history.empty
toast.formatFailed
```

Do not add runtime dependencies.

**Verify**: `cd apps/webui && npm run build` → TypeScript accepts the new module.

### Step 2: Add dictionary completeness tests

Add a Vitest test that verifies:

- English and Japanese dictionaries have the same keys.
- `createTranslator('en')('toolbar.file')` returns the English string.
- Interpolation replaces params and leaves no `{param}` placeholder for known params.
- Invalid stored locale fallback logic (added in Step 3) returns `DEFAULT_LOCALE`.

Model the test style after existing `apps/webui/src/history.test.ts`: keep it pure and independent of React rendering.

**Verify**: `cd apps/webui && npm test -- i18n` → i18n tests pass.

### Step 3: Persist locale in settings

In `apps/webui/src/lib/settings.ts` and `apps/webui/src/hooks/useSettings.ts`:

1. Add a `locale: Locale` field to `Settings`, defaulting to `DEFAULT_LOCALE`.
2. When reading from localStorage, validate that the stored value is `ja` or `en`; otherwise fall back to `DEFAULT_LOCALE`.
3. Keep backward compatibility with existing settings JSON that has no `locale` field.
4. Expose `updateSetting('locale', value)` through the existing settings update path.

**Verify**: `cd apps/webui && npm test` → existing settings/localStorage tests still pass; add or adjust tests if settings parsing is already covered.

### Step 4: Add a language selector to SettingsModal

In `apps/webui/src/components/SettingsModal.tsx`:

1. Accept a translator `t` or messages object via props, plus current `settings.locale`.
2. Add a settings section for language / 表示言語.
3. Use a `<select>` or radio buttons to set `locale` to `ja` or `en` through `updateSetting`.
4. Translate the modal title, close button aria-label, section labels, and option labels that are already in this component.

Prefer passing `t` down from `App.tsx` rather than importing mutable global state in every component.

**Verify**: `cd apps/webui && npm run lint` → no prop/type errors.

### Step 5: Migrate main shell strings

In `apps/webui/src/App.tsx`, create a translator from `settings.locale`, for example:

```ts
const t = useMemo(() => createTranslator(settings.locale), [settings.locale])
```

Pass `t` to components that need it and replace hardcoded strings in these priority areas:

1. `Toolbar.tsx`: button labels, menu section labels, titles.
2. `PreviewPanel.tsx`: preview control labels, filter/detail panel labels, placeholder text.
3. `HistoryModal.tsx`: modal title, empty state, action labels.
4. `GalleryModal.tsx`: modal title and network note.
5. `App.tsx`: format/lint toast messages and split divider title.

If migrating all five components makes the diff too large, STOP after Toolbar + SettingsModal + PreviewPanel and record remaining components as follow-up in the plan status note. Do not leave partially translated text within the same component if it can be avoided.

**Verify**: `cd apps/webui && npm run build` → TypeScript passes.

### Step 6: Run final WebUI gates

**Verify**:

- `cd apps/webui && npm test` → exit 0.
- `cd apps/webui && npm run lint` → exit 0.
- `cd apps/webui && npm run build` → exit 0.

## Test plan

- Add pure i18n dictionary tests for key parity and interpolation.
- Add settings parser tests for invalid/missing locale if settings tests exist or can be added without React test dependencies.
- Manual QA after build:
  - Switch language to English; toolbar/settings/preview labels update.
  - Reload page; locale persists.
  - Switch back to Japanese; labels update and existing settings remain intact.

## Done criteria

- [ ] A typed i18n dictionary exists with `ja` and `en` entries.
- [ ] Dictionary parity is tested.
- [ ] Locale is persisted and invalid stored values fall back safely.
- [ ] Settings UI exposes language switching.
- [ ] Main shell components migrated, or any deferred components are explicitly listed in `plans/README.md` status note.
- [ ] `cd apps/webui && npm test` exits 0.
- [ ] `cd apps/webui && npm run lint` exits 0.
- [ ] `cd apps/webui && npm run build` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- Adding `locale` to settings breaks migration of existing localStorage data and cannot be fixed with a small parser change.
- The diff grows beyond main shell/settings/preview and starts touching editor language support or generated WASM files.
- A component requires significant redesign just to accept translated strings.
- You need an external i18n package to complete the plan.

## Maintenance notes

This plan intentionally creates a foundation, not full localization of every text source. Future PRs should add keys to both dictionaries in the same change; the parity test should fail if one language is missing. Keep generated DSL output and user-authored timeline labels untranslated.
