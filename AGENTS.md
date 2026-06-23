# AGENTS.md

This document defines how AI coding agents (Claude Code, Codex, etc.) should work with this repository.

---

# 1. Project Overview

This project is a **Timeline DSL + Wikidata import engine**.

Core architecture:

DSL → Parser → AST → IR (normalized model) → Renderer / CLI / Export

Key design principles:

- DSL expresses **semantic timeline data**, not rendering
- IR is the **single source of truth**
- Wikidata import is a **first-class feature**
- Static data and imported data must coexist
- Implementation, README, spec, and tests must always match

---

# 2. Repository Structure

- `tdsl-parser` → DSL parsing (pest-based)
- `tdsl-core` → IR, lowering, validation
- `tdsl-wikidata` → Wikidata integration
- `tdsl-cli` → CLI interface

Do NOT mix responsibilities across crates.

---

# 3. Core DSL Concepts

### Timeline model

- `timeline` → global settings
- `lane` → vertical axis (entities: people, countries, etc.)
- `span` → existence period
- `event` → point event
- `event_range` → duration event

### Import model

- `import wikidata { entity Qxxxx; }`
- `map wd.xxx to span/event/event_range`

### Important constraints

- DSL must remain **rendering-independent**
- Lane IDs must be **stable**
- All references must be **strictly validated (no silent fallback)**

---

# 4. Critical Rules (MUST FOLLOW)

## 4.1 No silent fallback

Never allow:

- unknown `wd.xxx`
- unknown `lane`
- unknown `map target`

These must produce errors, not warnings.

---

## 4.2 Spec and implementation must match

If you change:

- parser
- AST
- IR
- lowering behavior

You MUST update:

- README.md
- docs/dsl-spec.md
- tests

---

## 4.3 Imported data rules

- All imported items must have:
  - `source = wd:<QID>`
  - `origin = "wikidata"` (set by lowering in `lower/mapping.rs`)

- Static items carry the `origin` value declared in the DSL (`origin` option); do NOT
  overwrite it during lowering.

---

## 4.4 Lane ID stability

- `lane "漢"` without alias must still produce valid ID
- Never generate empty IDs
- Use deterministic fallback (e.g. lane_1, lane_2)

---

## 4.5 IR is authoritative

- Renderer / CLI / Export must depend only on IR
- Parser output must not directly drive rendering

---

# 5. Unsupported / Deferred Features

> NOTE: The features below are the *current* gaps. `query "..." as alias`,
> `template` / `apply`, and qualifier mapping (`claim(P39).qualifier(P580)`) are
> already implemented and tested — do NOT treat them as unimplemented. See
> `CLAUDE.md` for the authoritative implementation-status list.

Still intentionally NOT implemented:

- `map source` — a `source:` property inside a `map` block (`MapProp` has no
  `Source` variant; only item-level `source wd:<QID>` exists)
- Sub-year precision beyond month/day (e.g. time-of-day)
- BCE (`year < 0`) month/day precision — imported BCE data is rounded to year
  precision in `lower/mapping.rs` (`strip_bc`)

If encountered:

- reject in parser OR
- mark clearly as "not implemented"

---

# 6. Error Handling Policy

Errors (must fail):

- unknown lane
- unknown import reference
- unknown map target
- invalid DSL structure

Warnings (allowed):

- missing optional data
- time precision loss (future TODO)

---

# 7. Development Workflow

## Step 1: Plan first

Before implementation:

- identify affected files
- define behavior changes
- check spec impact

## Step 2: Implement in layers

Order:

1. parser
2. AST
3. lowering
4. IR
5. CLI / output

## Step 3: Update documentation

Always update:

- README.md
- docs/dsl-spec.md

## Step 4: Add tests

Required:

- parser tests
- lowering tests
- integration / snapshot tests

---

# 8. Testing Requirements

Must cover:

- DSL parsing success/failure
- strict reference validation
- imported vs static item consistency
- lane ID generation
- IR correctness (snapshot)

---

# 9. Coding Style Guidelines

- Prefer explicit types over implicit behavior
- Avoid magic strings → use enums
- Fail fast on invalid input
- Keep lowering logic deterministic

---

# 10. When in doubt

If unsure:

1. Prefer **strictness over permissiveness**
2. Prefer **explicit errors over silent behavior**
3. Prefer **simpler MVP over feature expansion**

---

# 11. Future Extension Points

These are expected to evolve:

- re-import / merge policy
- time precision model
- template system
- UI editor integration
- CSV import/export

Do NOT prematurely implement them.

---

# 12. Output Expectations (for agents)

When making changes, always report:

1. What was changed
2. Why it was changed
3. What spec was updated
4. What tests were added
5. What remains unimplemented

---

End of AGENTS.md
