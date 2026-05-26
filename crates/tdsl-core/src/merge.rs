use std::collections::HashSet;

use crate::ir::{ImportRecord, Item, Lane, SourceRecord, TimelineIr};

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
        // Expand range to cover this file's range.
        let (other_start, other_end) = other.meta.range;
        let (cur_start, cur_end) = meta.range;
        meta.range = (cur_start.min(other_start), cur_end.max(other_end));

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
            start_month,
            start_day,
            end_month,
            end_day,
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
            start_month,
            start_day,
            end_month,
            end_day,
            source_span,
        },
        Item::Event {
            lane,
            time,
            label,
            tags,
            source,
            origin,
            time_month,
            time_day,
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
            time_month,
            time_day,
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
            start_month,
            start_day,
            end_month,
            end_day,
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
            start_month,
            start_day,
            end_month,
            end_day,
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
            start_month: None,
            start_day: None,
            end_month: None,
            end_day: None,
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
}
