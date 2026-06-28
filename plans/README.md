# Advisor Plans

Written against commit `9d6d02f`.

| ID | Plan | Status | Dependencies |
|---|---|---|---|
| 001 | Fix minute/hour precision validation | DONE | None |
| 002 | Synchronize stale documentation with implemented DSL/LSP/formatter behavior | DONE | None |
| 003 | Add CI coverage for VS Code extension tests | DONE | None |

Recommended order: 001 first because it fixes a correctness bug; 002 can land independently; 003 can land after 002 or independently.

## Considered but not planned

- CSV `source`/`origin` full round-trip remains intentionally unimplemented and is already documented as a future extension.
