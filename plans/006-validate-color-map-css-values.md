<!-- markdownlint-disable MD013 MD060 -->

# Plan 006: Validate color_map CSS values before emitting SVG style attributes

> **Executor instructions**: Follow this plan step by step. Run every verification command and confirm the expected result before moving to the next step. If anything in the "STOP conditions" section occurs, stop and report — do not improvise. When done, update the status row for this plan in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 3176e9d..HEAD -- crates/tdsl-render/src/layout.rs crates/tdsl-render/src/svg.rs README.md README.ja.md docs/dsl-spec.md docs/dsl-spec.en.md docs/styling.md`
> If any in-scope file changed since this plan was written, compare the "Current state" excerpts against the live code before proceeding; on a mismatch, treat it as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED
- **Depends on**: none; pairs well with Plan 005
- **Category**: security
- **Planned at**: commit `3176e9d`, 2026-06-28

## Why this matters

TDSL source can contain `color_map` values, and WebUI renders generated SVG by assigning it through `dangerouslySetInnerHTML`. Today a color string from DSL or CLI options is copied into an SVG `style` attribute without validation or style-value escaping. The safest small fix is to accept a documented, conservative color subset for `color_map` and fall back to the lane palette for invalid values; advanced styling remains available through explicit `custom_css` in CLI HTML rendering.

## Current state

Relevant files:

- `crates/tdsl-render/src/layout.rs` — resolves tag colors before SVG emission.
- `crates/tdsl-render/src/svg.rs` — emits resolved color inside SVG `style` attributes.
- `docs/dsl-spec.md` / `docs/dsl-spec.en.md` and README pair — document `color_map` behavior.
- `docs/styling.md` — styling-related user documentation if present sections mention `color_map`.

Current renderer behavior (`crates/tdsl-render/src/layout.rs:712-719`):

```rust
pub(crate) fn resolve_item_color(
    tags: &[String],
    color_map: &HashMap<String, String>,
    lane_id: &str,
    lane_colors: &HashMap<String, String>,
) -> String {
    for tag in tags {
        if let Some(color) = color_map.get(tag.as_str()) {
            return color.clone();
        }
    }
```

Current SVG emission (`crates/tdsl-render/src/svg.rs:355-370`):

```rust
let aria_label = escape_xml_attr(&item_aria_label(item, tooltip, lane_label));
let fill_style = format!("fill:{color};");
let tags = item_tags(item);
let mut data_attrs = format!(r#" data-lane=\"{}\""#, escape_xml_attr(lane_id));
// ...
writeln!(
    s,
    r#"  <g class=\"tdsl-item tdsl-item-span\" ...><rect class=\"tdsl-span\" style=\"{fill_style}\" ...>"#,
```

Existing XML attribute escaping helper (`crates/tdsl-render/src/svg.rs:682-695`) handles attribute syntax, but the current `style` construction does not validate CSS values before interpolation.

Repo constraints:

- No silent fallback for semantic references still applies to lanes/imports/map targets. This plan is not about references; invalid optional visual color values may fall back if documented and tested.
- If parser, AST, IR, or user-visible lowering/render behavior changes, update README, `docs/dsl-spec.md`, `docs/dsl-spec.en.md`, and tests.
- Keep renderer independent from parser AST.

## Commands you will need

| Purpose | Command | Expected on success |
|---|---|---|
| Focused renderer tests | `cargo test -p tdsl-render color` | exit 0; new invalid-color tests pass |
| Full renderer tests | `cargo test -p tdsl-render` | exit 0 |
| Workspace tests | `cargo test --workspace --all-targets` | exit 0 |
| Format check | `cargo fmt --all -- --check` | exit 0 |
| Docs grep sanity | `rg -n "color_map\|CSS color\|hex" README.md README.ja.md docs/dsl-spec.md docs/dsl-spec.en.md docs/styling.md` | docs mention the accepted subset consistently |

## Scope

**In scope**:

- `crates/tdsl-render/src/layout.rs`
- `crates/tdsl-render/src/svg.rs` only if you choose to add a style-value escape helper there instead of validating in layout
- Renderer tests in `crates/tdsl-render/src/layout.rs` or `crates/tdsl-render/src/svg.rs`
- `README.md`, `README.ja.md`, `docs/dsl-spec.md`, `docs/dsl-spec.en.md`, `docs/styling.md` only where they describe `color_map`

**Out of scope**:

- Changing the DSL grammar for `color_map`.
- Accepting arbitrary CSS functions such as `url(...)`, `var(...)`, or `color-mix(...)` from DSL `color_map`.
- Sanitizing `custom_css`; it is an explicit CLI-provided stylesheet and should remain separate.
- Adding a WebUI color picker.

## Git workflow

- Suggested branch: `advisor/006-color-map-validation`
- Commit message style: `fix(render): validate color_map CSS values`
- Do not push or open a PR unless instructed.

## Steps

### Step 1: Define a conservative color validator in tdsl-render

In `crates/tdsl-render/src/layout.rs`, add a private helper near `resolve_item_color`, for example:

```rust
fn is_safe_color_value(value: &str) -> bool {
    // Accept only:
    // - hex colors: #RGB, #RGBA, #RRGGBB, #RRGGBBAA
    // - simple CSS named colors / keywords: ASCII identifier with optional hyphen
}
```

Recommended exact policy:

- Trim ASCII whitespace before validating.
- Accept hex values only when the first char is `#`, the remaining length is 3, 4, 6, or 8, and every remaining char is ASCII hex.
- Accept simple identifiers only when all chars are ASCII alphanumeric or `-`, the first char is ASCII alphabetic, and the string contains no `;`, `:`, `(`, `)`, quotes, backslashes, or whitespace.
- Reject empty values and anything else.

This keeps common values like `#3366cc`, `red`, `currentColor`, and `rebeccapurple`, but rejects strings that can break out of a CSS declaration or invoke URL/function parsing.

**Verify**: `cargo test -p tdsl-render color` → existing color tests still pass if no tests have been added yet.

### Step 2: Apply validation in resolve_item_color

Update `resolve_item_color` so tag override values are returned only if valid. If invalid, skip that tag's override and continue to the next tag; if no valid tag override exists, fall back to lane palette as today.

Suggested behavior:

```rust
for tag in tags {
    if let Some(color) = color_map.get(tag.as_str()) {
        let color = color.trim();
        if is_safe_color_value(color) {
            return color.to_string();
        }
    }
}
```

Do not emit warnings from this low-level renderer function; it has no diagnostics channel. Document the fallback in docs and cover with tests.

**Verify**: `cargo test -p tdsl-render color` → exit 0.

### Step 3: Add regression tests for malicious and valid values

Add tests in the same module that already contains `color_map_tag_overrides_lane_palette` in `crates/tdsl-render/src/svg.rs`, or add unit tests for the helper in `layout.rs` if it remains private there.

Required cases:

- Valid `#3366cc` override appears in SVG.
- Valid named color such as `rebeccapurple` appears in SVG.
- Invalid value containing a semicolon and an extra declaration does not appear anywhere in SVG output.
- Invalid value containing `url(` or quotes does not appear anywhere in SVG output.
- Fallback lane palette still appears for invalid override so the item remains visible.

Do not include any runnable exploit instructions in comments; describe cases as invalid CSS value boundary tests.

**Verify**: `cargo test -p tdsl-render color` → all new cases pass.

### Step 4: Update docs to match the accepted color subset

Update user docs that describe `color_map`. Use wording like:

- English: "`color_map` accepts hex colors (`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`) and simple CSS named color keywords. More complex CSS values are intentionally ignored by the renderer; use CLI `--custom-css` for advanced styling."
- Japanese: "`color_map` は hex 色（`#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`）と単純な CSS 色キーワードを受け付けます。複雑な CSS 値は安全のため renderer が無視します。高度な装飾は CLI の `--custom-css` を使ってください。"

Keep Japanese and English DSL specs synchronized.

**Verify**: `rg -n "color_map|CSS color|hex" README.md README.ja.md docs/dsl-spec.md docs/dsl-spec.en.md docs/styling.md` → docs show the same accepted subset in both languages.

### Step 5: Run full gates

**Verify**:

- `cargo fmt --all -- --check` → exit 0.
- `cargo test -p tdsl-render` → exit 0.
- `cargo test --workspace --all-targets` → exit 0.

## Test plan

- Add renderer tests for valid hex, valid named keyword, invalid declaration-breaking strings, invalid function-like strings, and fallback behavior.
- Existing snapshot tests should not change except if an existing fixture used a newly invalid complex color. If so, STOP and report because this plan assumes fixtures use simple hex colors.

## Done criteria

- [ ] Invalid `color_map` values cannot appear raw in rendered SVG.
- [ ] Valid documented color values still work.
- [ ] `cargo test -p tdsl-render color` exits 0 with new tests.
- [ ] `cargo test --workspace --all-targets` exits 0.
- [ ] README and both DSL specs describe the same accepted subset.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- The project already added a color validator elsewhere; do not create a duplicate competing policy.
- Existing examples rely on complex CSS color functions in `color_map`.
- The fix appears to require changing parser grammar or IR schema.
- Documentation cannot be updated consistently in both Japanese and English.

## Maintenance notes

If future product work needs gradients, CSS variables, or color functions in `color_map`, add a structured color model or a dedicated advanced styling channel rather than reopening arbitrary style injection. Reviewers should check that invalid values are absent from the final SVG string, not merely escaped in a way browsers may still interpret unexpectedly.
