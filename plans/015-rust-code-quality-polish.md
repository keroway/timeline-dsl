<!-- markdownlint-disable MD013 MD060 -->

# Plan 015: Rust code-quality polish batch (decompile unwrap, silent LSP fallback, thin CLI tests)

> **Executor instructions**: Small, independent nits — do them together but keep them in separate commits so any one can be dropped in review. Run all gates. Update the status row in `plans/README.md` when done.
>
> **Drift check (run first)**: `git diff --stat 1c22dac..HEAD -- crates/tdsl-core/src/decompile.rs crates/tdsl-lsp/src/backend.rs crates/tdsl-cli`

## Status

- **Priority**: P3
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: code quality / test coverage
- **Planned at**: commit `1c22dac`, 2026-07-05
- **Tracking issue**: #593
- **Source**: carried over from `docs/reviews/2026-06-10-project-review.md` ("issue 化しなかった軽微な指摘")

## Why this matters

These are minor items the last full review recorded but did not file. They do
not affect correctness today but tighten conformance with the repo's own
conventions (no `unwrap` in production code, no silent fallbacks, fail-fast) and
close a coverage gap in the CLI.

## Item A — `decompile.rs` `writeln!(...).unwrap()`

**Where**: `crates/tdsl-core/src/decompile.rs` (around lines 23–47) uses
`writeln!(...).unwrap()` writing into a `String`. Writing to a `String` is
infallible, so this never panics — but it violates the "no `unwrap` in
`crates/*/src` non-test code" convention on its face.

**Fix (pick one)**:

- Preferred: build output with `let mut s = String::new();` + `s.push_str(...)` / `write!`-free formatting where practical; or
- Keep `writeln!` but switch to `use std::fmt::Write;` and `let _ = writeln!(...)` **with** a one-line comment noting writes to `String` are infallible; or
- Replace `.unwrap()` with `.expect("writing to String is infallible")` plus the same comment.

Choose whichever keeps the code readable; the goal is to remove a bare
`.unwrap()` and make the infallibility explicit.

**Verify**: `cargo test -p tdsl-core` exits 0; decompile snapshot/round-trip tests unchanged.

## Item B — `backend.rs` silent capability fallback

**Where**: `crates/tdsl-lsp/src/backend.rs:86` — a client-capability check uses
`unwrap_or(false)`, silently degrading a feature when the capability is absent.

**Fix**: keep the `unwrap_or(false)` behavior (correct default) but make it
**observable** — add a `tracing`/log line (or at minimum an explanatory comment)
stating which capability was absent and which feature is therefore disabled.
Match whatever logging the LSP backend already uses; do not introduce a new
logging dependency.

**Verify**: `cargo test -p tdsl-lsp` exits 0 (all ~95 LSP tests green).

## Item C — Thin CLI unit tests for `build` / `fetch`

**Where**: `crates/tdsl-cli` — the `build` and `fetch` command handlers are
covered mainly by `tests/cli_integration_test.rs` and e2e-smoke, with thin unit
coverage.

**Fix**: add targeted unit tests for the pure/decomposable parts of the `build`
and `fetch` handlers (argument→option mapping, output-path resolution, error
messages for missing input, format selection). Do **not** add tests that hit the
live Wikidata network; mock or restrict to offline paths (the CLI already
supports offline / cache modes — reuse those). Keep new tests deterministic.

**Verify**: `cargo test -p tdsl-cli` exits 0 with the new tests running.

## Scope

**In scope**: the three files/areas above and their tests.

**Out of scope**:

- Behavior changes to decompile output, LSP capabilities, or CLI semantics.
- Broad refactors; keep each item minimal.
- The `gen-grammar-keywords.mjs` brittleness item from the same review — **already resolved** (the generator now reads `keywords.json` as the single source; do not re-open).

## Git workflow

- Suggested branch: `advisor/015-rust-polish`
- Three commits: `refactor(core): make decompile String writes infallible-explicit`, `chore(lsp): log capability fallback instead of silent degrade`, `test(cli): add unit coverage for build/fetch handlers`.
- Do not push or open a PR unless instructed.

## Final gates

- `cargo fmt --all -- --check` → exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings` → exit 0.
- `cargo test --workspace --all-targets` → exit 0.

## Done criteria

- [ ] No bare `.unwrap()` on `String` writes in `decompile.rs`; infallibility explicit.
- [ ] LSP capability fallback is logged/commented, not silent.
- [ ] New deterministic, offline unit tests for CLI `build`/`fetch` handlers.
- [ ] fmt / clippy / test gates all green.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- Making the CLI handlers unit-testable requires a non-trivial refactor of `main`/dispatch (report; a smaller slice covering only argument/path mapping is acceptable).
- The LSP backend has no existing logging facility (then use an explanatory comment only and note it).

## Maintenance notes

These are the last recorded minor items from the 2026-06-10 review. After this
lands, that review's "補遺" list is fully addressed except items intentionally
deferred elsewhere.
