# Plan 001 — Fix minute/hour precision validation

Written against commit `9d6d02f`.

## Problem

`TimelineIr` stores hour/minute precision for timeline ranges and item ranges, but `tdsl_core::validate` compares only `(year, month, day)`. A range such as `2020-01-01T12:00..2020-01-01T11:00` currently passes `tdsl check` even though the end time is earlier than the start time.

Evidence:

- `crates/tdsl-core/src/ir.rs` has `range_*_hour/_minute` and item `*_hour/_minute` fields.
- `crates/tdsl-core/src/validate.rs` uses a three-part `sortable_tuple(year, month, day)`.

## Implementation steps

1. Extend validation tuple comparison to include hour and minute: `(year, month_or_0, day_or_0, hour_or_0, minute_or_0)`.
2. Update span, event_range, and timeline range validation to pass hour/minute fields.
3. Add regression tests in `crates/tdsl-core/src/tests/validation.rs` for:
   - span with same date but start minute after end minute;
   - event_range with same date but start hour after end hour;
   - timeline range with same date but start minute equal/after end minute.
4. Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.

## Done criteria

- Invalid same-day minute/hour ranges fail validation.
- Existing year/month/day validation behavior remains unchanged.
- All Rust tests and clippy pass.
