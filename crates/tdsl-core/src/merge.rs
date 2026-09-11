use std::collections::HashSet;

use crate::ir::{ImportRecord, Item, Lane, Meta, SourceRecord, TimeParts, TimelineIr};
use crate::validate::compare_ir_time;

/// Merge warnings reported during IR merging.
pub type MergeWarnings = Vec<String>;

/// Merge multiple `TimelineIr` values into one.
///
/// Strategy:
/// - `meta`: first IR wins; `range` expands to cover all files.
/// - `lanes`: first occurrence of each lane ID wins; duplicates emit a warning.
/// - `items`: all items concatenated; duplicate IDs are suffixed with `_<n>`.
/// - `imports`/`sources`: union by `qid`/`id`, first occurrence wins.
pub fn merge_irs(irs: Vec<TimelineIr>) -> (TimelineIr, MergeWarnings) {
    assert!(!irs.is_empty(), "merge_irs requires at least one IR");

    let mut warnings = Vec::new();
    let mut iter = irs.into_iter();
    // 直前の assert! で空でないことを保証済みなので next() は必ず Some
    // （implementation-strict.md §4.1: 例外的に使う場合は理由を 1 行）。
    let first = iter.next().unwrap();

    let mut meta = first.meta;
    let mut lanes: Vec<Lane> = first.lanes;
    let mut items: Vec<Item> = first.items;
    let mut imports: Vec<ImportRecord> = first.imports;
    let mut sources: Vec<SourceRecord> = first.sources;

    let mut seen_lane_ids: HashSet<String> = lanes.iter().map(|l| l.id.clone()).collect();
    let mut seen_item_ids: HashSet<String> = items.iter().map(|i| item_id(i).to_owned()).collect();
    let mut seen_qids: HashSet<String> = imports.iter().map(|r| r.qid.clone()).collect();
    let mut seen_source_ids: HashSet<String> = sources.iter().map(|s| s.id.clone()).collect();

    for other in iter {
        // Expand range to cover this file's range, keeping month/day/... precision
        // consistent with whichever boundary (start/end) is actually chosen
        // (#896: previously only the `i64` year was min/max'd, leaving precision
        // fields stuck at the first IR's values).
        let cur_start_parts = range_start_parts(&meta);
        let other_start_parts = range_start_parts(&other.meta);
        let cur_end_parts = range_end_parts(&meta);
        let other_end_parts = range_end_parts(&other.meta);

        let start_from_other = match compare_ir_time(cur_start_parts, other_start_parts) {
            Some(ordering) => ordering == std::cmp::Ordering::Greater,
            None => {
                warnings.push(format!(
                    "range start comparison between offset-aware and offset-naive times is ambiguous ({} vs {}); falling back to year-only comparison",
                    format_time_parts(cur_start_parts),
                    format_time_parts(other_start_parts)
                ));
                other_start_parts.year < cur_start_parts.year
            }
        };
        let end_from_other = match compare_ir_time(cur_end_parts, other_end_parts) {
            Some(ordering) => ordering == std::cmp::Ordering::Less,
            None => {
                warnings.push(format!(
                    "range end comparison between offset-aware and offset-naive times is ambiguous ({} vs {}); falling back to year-only comparison",
                    format_time_parts(cur_end_parts),
                    format_time_parts(other_end_parts)
                ));
                other_end_parts.year > cur_end_parts.year
            }
        };

        if start_from_other {
            apply_range_start(&mut meta, other_start_parts);
        }
        if end_from_other {
            apply_range_end(&mut meta, other_end_parts);
        }

        // Merge color_map (first occurrence per key wins).
        for (k, v) in other.meta.color_map {
            meta.color_map.entry(k).or_insert(v);
        }

        // Merge lanes.
        for lane in other.lanes {
            if seen_lane_ids.contains(&lane.id) {
                warnings.push(format!(
                    "lane '{}' already defined; skipping duplicate from merged file",
                    lane.id
                ));
            } else {
                seen_lane_ids.insert(lane.id.clone());
                lanes.push(lane);
            }
        }

        // Merge items (deduplicate IDs by appending a counter suffix).
        for item in other.items {
            let base_id = item_id(&item).to_owned();
            let unique_id = if seen_item_ids.contains(&base_id) {
                let mut n = 2u32;
                loop {
                    let candidate = format!("{base_id}_{n}");
                    if !seen_item_ids.contains(&candidate) {
                        break candidate;
                    }
                    n += 1;
                }
            } else {
                base_id.clone()
            };

            if unique_id != base_id {
                warnings.push(format!(
                    "item id '{base_id}' already exists; renamed to '{unique_id}' during merge"
                ));
            }

            seen_item_ids.insert(unique_id.clone());
            items.push(set_item_id(item, unique_id));
        }

        // Merge imports (by QID, first occurrence wins).
        for record in other.imports {
            if seen_qids.insert(record.qid.clone()) {
                imports.push(record);
            }
        }

        // Merge sources (by ID, first occurrence wins).
        for record in other.sources {
            if seen_source_ids.insert(record.id.clone()) {
                sources.push(record);
            }
        }
    }

    // Re-order lanes by their `order` field so the merged result is stable.
    lanes.sort_by_key(|l| l.order);

    let merged = TimelineIr {
        meta,
        lanes,
        items,
        imports,
        sources,
    };
    (merged, warnings)
}

/// `meta.range.0` とその精度フィールドをまとめて `TimeParts` にする。
fn range_start_parts(meta: &Meta) -> TimeParts {
    TimeParts {
        year: meta.range.0,
        month: meta.range_start_month,
        day: meta.range_start_day,
        hour: meta.range_start_hour,
        minute: meta.range_start_minute,
        second: meta.range_start_second,
        offset_minutes: meta.range_start_offset_minutes,
    }
}

/// `meta.range.1` とその精度フィールドをまとめて `TimeParts` にする。
fn range_end_parts(meta: &Meta) -> TimeParts {
    TimeParts {
        year: meta.range.1,
        month: meta.range_end_month,
        day: meta.range_end_day,
        hour: meta.range_end_hour,
        minute: meta.range_end_minute,
        second: meta.range_end_second,
        offset_minutes: meta.range_end_offset_minutes,
    }
}

/// 採用した `TimeParts` を `meta` の start 側フィールド一式に一貫してコピーする
/// （年だけ・精度だけの部分的な混在を避けるため、6 フィールドまとめて上書きする）。
fn apply_range_start(meta: &mut Meta, parts: TimeParts) {
    meta.range.0 = parts.year;
    meta.range_start_month = parts.month;
    meta.range_start_day = parts.day;
    meta.range_start_hour = parts.hour;
    meta.range_start_minute = parts.minute;
    meta.range_start_second = parts.second;
    meta.range_start_offset_minutes = parts.offset_minutes;
}

/// 採用した `TimeParts` を `meta` の end 側フィールド一式に一貫してコピーする。
fn apply_range_end(meta: &mut Meta, parts: TimeParts) {
    meta.range.1 = parts.year;
    meta.range_end_month = parts.month;
    meta.range_end_day = parts.day;
    meta.range_end_hour = parts.hour;
    meta.range_end_minute = parts.minute;
    meta.range_end_second = parts.second;
    meta.range_end_offset_minutes = parts.offset_minutes;
}

/// 曖昧な比較時の警告メッセージ用に `TimeParts` を人間可読な文字列にする。
fn format_time_parts(t: TimeParts) -> String {
    match (t.month, t.day, t.hour, t.minute, t.second, t.offset_minutes) {
        (None, None, None, None, None, None) => t.year.to_string(),
        _ => format!("{t:?}"),
    }
}

fn item_id(item: &Item) -> &str {
    match item {
        Item::Span { id, .. } | Item::Event { id, .. } | Item::EventRange { id, .. } => id,
    }
}

fn set_item_id(item: Item, new_id: String) -> Item {
    match item {
        Item::Span {
            lane,
            start,
            end,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            start_offset_minutes,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            end_offset_minutes,
            end_open,
            source_span,
            ..
        } => Item::Span {
            id: new_id,
            lane,
            start,
            end,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            start_offset_minutes,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            end_offset_minutes,
            end_open,
            source_span,
        },
        Item::Event {
            lane,
            time,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            time_month,
            time_day,
            time_hour,
            time_minute,
            time_second,
            time_offset_minutes,
            source_span,
            ..
        } => Item::Event {
            id: new_id,
            lane,
            time,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            time_month,
            time_day,
            time_hour,
            time_minute,
            time_second,
            time_offset_minutes,
            source_span,
        },
        Item::EventRange {
            lane,
            start,
            end,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            start_offset_minutes,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            end_offset_minutes,
            end_open,
            source_span,
            ..
        } => Item::EventRange {
            id: new_id,
            lane,
            start,
            end,
            label,
            tags,
            source,
            origin,
            note,
            link,
            color,
            start_month,
            start_day,
            start_hour,
            start_minute,
            start_second,
            start_offset_minutes,
            end_month,
            end_day,
            end_hour,
            end_minute,
            end_second,
            end_offset_minutes,
            end_open,
            source_span,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::TimelineIr;
    use std::collections::HashMap;

    fn make_ir(title: &str, range: (i64, i64), lanes: Vec<Lane>, items: Vec<Item>) -> TimelineIr {
        use crate::ir::Meta;
        TimelineIr {
            meta: Meta {
                title: title.to_string(),
                unit: "year".to_string(),
                range,
                calendar: "proleptic_gregorian".to_string(),
                color_map: HashMap::new(),
                ..Default::default()
            },
            lanes,
            items,
            imports: vec![],
            sources: vec![],
        }
    }

    fn lane(id: &str, label: &str, order: i64) -> Lane {
        Lane {
            id: id.to_string(),
            label: label.to_string(),
            kind: "custom".to_string(),
            order,
            group: None,
            color: None,
            source_span: None,
        }
    }

    fn span(id: &str, lane: &str, start: i64, end: i64) -> Item {
        Item::Span {
            id: id.to_string(),
            lane: lane.to_string(),
            start,
            end,
            label: id.to_string(),
            tags: vec![],
            source: None,
            origin: None,
            note: None,
            link: None,
            color: None,
            start_month: None,
            start_day: None,
            start_hour: None,
            start_minute: None,
            start_second: None,
            start_offset_minutes: None,
            end_month: None,
            end_day: None,
            end_hour: None,
            end_minute: None,
            end_second: None,
            end_offset_minutes: None,
            end_open: false,
            source_span: None,
        }
    }

    #[test]
    fn merge_single_ir_is_identity() {
        let ir = make_ir(
            "A",
            (0, 100),
            vec![lane("a", "A", 1)],
            vec![span("s1", "a", 10, 20)],
        );
        let (merged, warnings) = merge_irs(vec![ir.clone()]);
        assert!(warnings.is_empty());
        assert_eq!(merged.meta.title, "A");
        assert_eq!(merged.lanes.len(), 1);
        assert_eq!(merged.items.len(), 1);
    }

    #[test]
    fn merge_two_irs_combines_lanes_and_items() {
        let ir1 = make_ir(
            "A",
            (0, 100),
            vec![lane("a", "A", 1)],
            vec![span("s1", "a", 10, 20)],
        );
        let ir2 = make_ir(
            "B",
            (50, 200),
            vec![lane("b", "B", 2)],
            vec![span("s2", "b", 60, 90)],
        );
        let (merged, warnings) = merge_irs(vec![ir1, ir2]);
        assert!(warnings.is_empty());
        assert_eq!(merged.meta.title, "A"); // first wins
        assert_eq!(merged.meta.range, (0, 200)); // expanded
        assert_eq!(merged.lanes.len(), 2);
        assert_eq!(merged.items.len(), 2);
    }

    #[test]
    fn merge_duplicate_lane_emits_warning() {
        let ir1 = make_ir("A", (0, 100), vec![lane("a", "A", 1)], vec![]);
        let ir2 = make_ir("B", (0, 100), vec![lane("a", "A2", 2)], vec![]);
        let (merged, warnings) = merge_irs(vec![ir1, ir2]);
        assert_eq!(merged.lanes.len(), 1); // duplicate skipped
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("lane 'a'"));
    }

    #[test]
    fn merge_duplicate_item_id_is_renamed() {
        let ir1 = make_ir(
            "A",
            (0, 100),
            vec![lane("a", "A", 1)],
            vec![span("s1", "a", 10, 20)],
        );
        let ir2 = make_ir(
            "B",
            (0, 100),
            vec![lane("b", "B", 2)],
            vec![span("s1", "b", 30, 40)],
        );
        let (merged, warnings) = merge_irs(vec![ir1, ir2]);
        assert_eq!(merged.items.len(), 2);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("renamed to 's1_2'"));
        let ids: Vec<&str> = merged.items.iter().map(item_id).collect();
        assert!(ids.contains(&"s1"));
        assert!(ids.contains(&"s1_2"));
    }

    #[test]
    fn merge_range_expands_to_union() {
        let ir1 = make_ir("A", (-100, 500), vec![], vec![]);
        let ir2 = make_ir("B", (-200, 300), vec![], vec![]);
        let ir3 = make_ir("C", (0, 1000), vec![], vec![]);
        let (merged, _) = merge_irs(vec![ir1, ir2, ir3]);
        assert_eq!(merged.meta.range, (-200, 1000));
    }

    /// `meta.range` の年だけでなく month/day 精度も採用した境界からまとめてコピーする
    /// （#896: 部分的な混在の回帰防止）。
    fn make_ir_with_range_parts(
        title: &str,
        start: TimeParts,
        end: TimeParts,
        lanes: Vec<Lane>,
        items: Vec<Item>,
    ) -> TimelineIr {
        let mut ir = make_ir(title, (start.year, end.year), lanes, items);
        apply_range_start(&mut ir.meta, start);
        apply_range_end(&mut ir.meta, end);
        ir
    }

    fn ymd(year: i64, month: u8, day: u8) -> TimeParts {
        TimeParts {
            year,
            month: Some(month),
            day: Some(day),
            ..Default::default()
        }
    }

    #[test]
    fn merge_range_union_preserves_month_day_precision() {
        // Reproduces #896: a's range is fully inside June 2024, b's range spans
        // the whole year. The union should be 2024-01-01..2024-12-31, not just
        // the year union with a's month/day left over.
        let ir_a = make_ir_with_range_parts("a", ymd(2024, 6, 1), ymd(2024, 6, 30), vec![], vec![]);
        let ir_b =
            make_ir_with_range_parts("b", ymd(2024, 1, 1), ymd(2024, 12, 31), vec![], vec![]);

        for irs in [vec![ir_a.clone(), ir_b.clone()], vec![ir_b, ir_a]] {
            let (merged, _) = merge_irs(irs);
            assert_eq!(merged.meta.range, (2024, 2024));
            assert_eq!(merged.meta.range_start_month, Some(1));
            assert_eq!(merged.meta.range_start_day, Some(1));
            assert_eq!(merged.meta.range_end_month, Some(12));
            assert_eq!(merged.meta.range_end_day, Some(31));
        }
    }

    #[test]
    fn merge_range_union_crosses_year_boundary() {
        let ir_a =
            make_ir_with_range_parts("a", ymd(2023, 12, 25), ymd(2024, 1, 5), vec![], vec![]);
        let ir_b =
            make_ir_with_range_parts("b", ymd(2023, 12, 28), ymd(2024, 1, 2), vec![], vec![]);

        for irs in [vec![ir_a.clone(), ir_b.clone()], vec![ir_b, ir_a]] {
            let (merged, warnings) = merge_irs(irs);
            assert!(warnings.is_empty());
            assert_eq!(merged.meta.range, (2023, 2024));
            assert_eq!(merged.meta.range_start_month, Some(12));
            assert_eq!(merged.meta.range_start_day, Some(25));
            assert_eq!(merged.meta.range_end_month, Some(1));
            assert_eq!(merged.meta.range_end_day, Some(5));
        }
    }

    #[test]
    fn merge_range_union_with_seconds_and_offsets() {
        let start_a = TimeParts {
            year: 2024,
            month: Some(6),
            day: Some(1),
            hour: Some(9),
            minute: Some(0),
            second: Some(0),
            offset_minutes: Some(540), // +09:00
        };
        let end_a = TimeParts {
            year: 2024,
            month: Some(6),
            day: Some(1),
            hour: Some(10),
            minute: Some(0),
            second: Some(0),
            offset_minutes: Some(540),
        };
        // Same instants expressed in UTC (offset 0); start_b is later in wall
        // clock terms but equal after UTC normalization, end_b is later.
        let start_b = TimeParts {
            year: 2024,
            month: Some(6),
            day: Some(1),
            hour: Some(0),
            minute: Some(30),
            second: Some(0),
            offset_minutes: Some(0),
        };
        let end_b = TimeParts {
            year: 2024,
            month: Some(6),
            day: Some(1),
            hour: Some(2),
            minute: Some(0),
            second: Some(0),
            offset_minutes: Some(0),
        };

        let ir_a = make_ir_with_range_parts("a", start_a, end_a, vec![], vec![]);
        let ir_b = make_ir_with_range_parts("b", start_b, end_b, vec![], vec![]);

        let (merged, warnings) = merge_irs(vec![ir_a, ir_b]);
        assert!(warnings.is_empty());
        // start_b (00:30 UTC) is earlier than start_a (09:00+09:00 == 00:00 UTC)? Let's
        // recompute: start_a normalized = 2024-06-01T00:00 UTC, start_b = 2024-06-01T00:30 UTC.
        // So start_a is earlier -> start stays start_a.
        assert_eq!(merged.meta.range_start_hour, Some(9));
        assert_eq!(merged.meta.range_start_offset_minutes, Some(540));
        // end_a normalized = 2024-06-01T01:00 UTC, end_b = 2024-06-01T02:00 UTC.
        // end_b is later -> end becomes end_b.
        assert_eq!(merged.meta.range_end_hour, Some(2));
        assert_eq!(merged.meta.range_end_offset_minutes, Some(0));
    }

    #[test]
    fn merge_range_ambiguous_offset_mix_emits_warning() {
        // start_a has no offset (naive), start_b has an offset -> comparison is
        // ambiguous per ADR 0003 D2; merge must warn rather than silently pick one.
        let start_a = TimeParts {
            year: 2024,
            month: Some(6),
            day: Some(1),
            hour: Some(9),
            minute: Some(0),
            second: Some(0),
            offset_minutes: None,
        };
        let end_a = ymd(2024, 6, 30);
        let start_b = TimeParts {
            year: 2024,
            month: Some(1),
            day: Some(1),
            hour: Some(0),
            minute: Some(0),
            second: Some(0),
            offset_minutes: Some(0),
        };
        let end_b = ymd(2024, 12, 31);

        let ir_a = make_ir_with_range_parts("a", start_a, end_a, vec![], vec![]);
        let ir_b = make_ir_with_range_parts("b", start_b, end_b, vec![], vec![]);

        let (merged, warnings) = merge_irs(vec![ir_a, ir_b]);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("ambiguous")));
        // Deterministic year-only fallback: both starts are year 2024, so the
        // fallback (`other.year < cur.year`) keeps the first IR's start.
        assert_eq!(merged.meta.range.0, 2024);
    }
}
