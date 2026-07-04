//! Wall-clock "current year" resolution for the `now` keyword (#550).
//!
//! `span` / `event_range` may write `now` in place of an explicit `end` time
//! value to mark an open-ended (still ongoing) period. This module resolves
//! that keyword to a concrete UTC year at parse time, without requiring a
//! date/time crate dependency (`std::time::SystemTime` on native targets,
//! `js_sys::Date` on `wasm32` + a small days-since-epoch → civil-date
//! conversion is enough for year-only precision).

/// Returns the current Unix time in whole seconds.
///
/// `std::time::SystemTime::now()` is unimplemented on `wasm32-unknown-unknown`
/// (no OS clock) and panics with a hard, uncatchable WASM trap rather than a
/// recoverable error (#583), so the wall-clock source is gated by target:
/// `js-sys::Date` on wasm32, `std::time::SystemTime` everywhere else.
#[cfg(target_arch = "wasm32")]
fn now_unix_secs() -> i64 {
    (js_sys::Date::now() / 1000.0) as i64
}

#[cfg(not(target_arch = "wasm32"))]
fn now_unix_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Returns the current UTC year, resolved from the wall clock.
///
/// Falls back to the Unix epoch year (1970) if the clock is somehow set
/// before `UNIX_EPOCH` (e.g. a misconfigured sandbox); this only affects
/// the `now` keyword and never causes a parse/lowering failure.
pub fn current_year_utc() -> i64 {
    let secs = now_unix_secs();
    let days = secs.div_euclid(86_400);
    civil_year_from_days(days)
}

/// Howard Hinnant's `civil_from_days` algorithm, year component only.
/// `days` is the number of days since 1970-01-01 (may be negative).
/// See: <http://howardhinnant.github.io/date_algorithms.html>
fn civil_year_from_days(days: i64) -> i64 {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    if m <= 2 { y + 1 } else { y }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_day_zero_is_1970() {
        assert_eq!(civil_year_from_days(0), 1970);
    }

    #[test]
    fn known_date_2024_03_01_is_year_2024() {
        // 2024-03-01 is 19783 days after 1970-01-01 (includes leap day 2024-02-29).
        assert_eq!(civil_year_from_days(19_783), 2024);
    }

    #[test]
    fn known_date_1999_12_31_is_year_1999() {
        // 1999-12-31 is -1 day before 2000-01-01 (day 10957).
        assert_eq!(civil_year_from_days(10_956), 1999);
    }

    #[test]
    fn negative_days_before_epoch_resolve_correctly() {
        // 1969-12-31 is day -1.
        assert_eq!(civil_year_from_days(-1), 1969);
    }

    #[test]
    fn current_year_utc_is_plausible() {
        // Sanity check only: must be a reasonable modern year, not a crash or
        // an obviously-wrong sentinel value.
        let y = current_year_utc();
        assert!((2020..=2200).contains(&y), "unexpected current year: {y}");
    }
}
