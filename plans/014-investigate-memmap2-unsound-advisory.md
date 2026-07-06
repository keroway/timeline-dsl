<!-- markdownlint-disable MD013 MD060 -->

# Plan 014: Investigate and resolve the `memmap2` unsound advisory in the PDF render chain

> **Executor instructions**: This is an **investigation** plan with a conditional fix. Follow the steps, record findings, and only apply a version change if the gates pass. If anything in "STOP conditions" occurs, stop and report. Update the status row in `plans/README.md` when done.
>
> **Drift check (run first)**: `cargo tree -i memmap2 2>/dev/null | head; git log --oneline -5 -- crates/tdsl-render/Cargo.toml`

## Status

- **Priority**: P3
- **Effort**: S (investigation) / M (if an upgrade requires coordinated version bumps)
- **Risk**: MEDIUM (PDF output regressions; ADR-0002 version coupling)
- **Depends on**: none
- **Category**: dependency hygiene / security advisory
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #592

## Why this matters

`cargo audit` (recorded in `docs/reviews/2026-06-10-project-review.md`) reports
`memmap2 0.9.10` as **unsound** (an advisory, not a fixed vuln) pulled in
transitively through `fontdb → svg2pdf / usvg / resvg` in `tdsl-render`'s PDF
path. The prior review deferred it because `svg2pdf` / `pdf-writer` versions are
**explicitly coupled** in `crates/tdsl-render/Cargo.toml` and ADR-0002, so a
naive bump risks breaking PDF output. This plan revisits it now that time has
passed and upstream may have compatible releases.

## Current state

- `crates/tdsl-render/Cargo.toml` — pins the PDF-related crates with documented coupling; ADR-0002 (`docs/adr/0002-pdf-output-method-selection.md`) explains the version constraints.
- The advisory is transitive; `tdsl-render` does not depend on `memmap2` directly.
- PDF output is a `tdsl-render` feature; the WebUI does **not** use this path (it prints HTML — see `useExport.ts` comment / ADR-0002 D5).

## Scope

**In scope**:

- Determine current advisory status and whether a compatible upgrade path exists for `fontdb` / `svg2pdf` / `usvg` / `resvg` that moves `memmap2` to a sound version (or removes the mmap feature).
- If a clean upgrade exists and PDF output still passes, apply it.
- Otherwise, formally document the accepted risk (e.g. `cargo audit` ignore with a dated rationale, or an ADR-0002 addendum).

**Out of scope**:

- Replacing the PDF backend / changing the PDF rendering method (that is a separate ADR-level decision).
- Touching non-PDF render paths.

## Steps

### Step 1: Reproduce and scope the advisory

```bash
cargo install cargo-audit --locked   # if not present
cargo audit
cargo tree -i memmap2
```

Record: the exact advisory id (RUSTSEC-xxxx), the version in the tree, and the
full dependency path(s) to `memmap2`.

### Step 2: Check upstream for a sound path

For each of `fontdb`, `usvg`, `resvg`, `svg2pdf`, `pdf-writer`: check latest
versions and whether a combination exists that (a) is mutually compatible per
ADR-0002 coupling and (b) pulls a sound `memmap2` (or a `fontdb` build without
the mmap feature — `fontdb` exposes a feature to disable memory-mapping).

Note: `fontdb`'s memory-map usage is often gated behind a feature; disabling it
(using file reads instead) may drop `memmap2` entirely. Investigate this as the
**lowest-risk** option.

### Step 3a: If a clean fix exists — apply and verify

Apply the minimal change (prefer disabling `fontdb`'s mmap feature over broad
version bumps). Then verify PDF output is unbroken:

```bash
cargo build -p tdsl-render --features pdf   # or the actual feature name
cargo test --workspace --all-targets
```

Manually render a `.tdsl` with CJK labels to PDF via the CLI and confirm fonts
still shape and the file opens. Re-run `cargo audit` → advisory gone.

### Step 3b: If no clean fix exists — document the accepted risk

- Add a dated entry to ADR-0002 (or a short ADR addendum) stating the advisory id, why it is low-risk here (transitive, PDF-only, not reachable from WebUI, unsound-not-exploited), and the revisit trigger.
- Optionally add a `deny.toml` / `audit.toml` ignore with the advisory id and the same rationale so CI `cargo audit` (if/when added) stays green intentionally.

## Test plan

- `cargo test --workspace --all-targets` green.
- Manual CLI PDF render (including CJK) opens correctly and fonts shape.
- `cargo audit` either reports clean or reports only the explicitly-ignored, documented advisory.

## Done criteria

- [ ] Advisory id + full dependency path recorded.
- [ ] Either: advisory resolved via minimal dependency change **and** PDF output verified; or: accepted-risk documented in ADR-0002 + audit ignore with a revisit trigger.
- [ ] `cargo test --workspace --all-targets` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- Resolving the advisory requires bumping `svg2pdf` / `pdf-writer` across the ADR-0002 coupling in a way that changes PDF output (needs an ADR decision, not a silent bump).
- Disabling `fontdb` mmap degrades font resolution / CJK shaping.
- The advisory is withdrawn/updated upstream (record and close).

## Maintenance notes

Consider adding `cargo audit` (or `cargo deny`) as a scheduled CI job so future
advisories surface automatically rather than during manual review. If added, wire
any documented ignore from Step 3b so the job is meaningful.
