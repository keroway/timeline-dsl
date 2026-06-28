# Plan 002 — Synchronize stale documentation with implemented behavior

Written against commit `9d6d02f`.

## Problem

Several docs describe behavior that has since changed:

- Formatter/LSP docs say comments are dropped, but `tdsl_parser::format_source` now preserves and re-emits comments.
- `docs/cli-spec.md` still says VS Code LSP client integration is future work, while `editors/vscode/src/extension.ts` starts `tdsl lsp` via `vscode-languageclient`.
- `editors/vscode/README.md` contains an old invalid DSL example.
- `docs/spec-date-precision.md` describes date precision as design-only and omits implemented `DateTime`/minute precision in AST/PEG examples.
- `docs/error-catalog.md` labels template/apply errors as future/unimplemented even though template/apply is implemented.

## Implementation steps

1. Update README and README.ja LSP formatting descriptions to say comments are preserved, with the caveat that comments inside blocks may be relocated by canonical formatting.
2. Update `docs/cli-spec.md` `fmt` and LSP sections to match current formatter and VS Code client behavior.
3. Replace the stale DSL example in `editors/vscode/README.md` with current `import wikidata as wd { entity ... }` and `map wd.alias to span { ... }` syntax.
4. Refresh `docs/spec-date-precision.md` status and snippets so they include `YYYY-MM-DDTHH:MM` / `DateTime` and no longer present already-shipped parser/AST work as pending.
5. Update `docs/error-catalog.md` E109/E110 explanations to remove “future/unimplemented”.
6. Verify `cargo test --workspace` and docs sample parsing where practical.

## Done criteria

- No doc says comments are lost by `tdsl fmt` / LSP formatting.
- No doc says VS Code LSP client is future work.
- VS Code README example parses with current grammar.
- Template/apply docs consistently treat the feature as implemented.
