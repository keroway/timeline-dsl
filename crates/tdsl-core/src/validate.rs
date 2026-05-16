use crate::ir::TimelineIr;

/// `(year, month_or_0, day_or_0)` を返す。月日が `None` の場合はソート上は最小値扱い。
fn sortable_tuple(year: i64, month: Option<u8>, day: Option<u8>) -> (i64, u8, u8) {
    (year, month.unwrap_or(0), day.unwrap_or(0))
}

/// Validate the IR for semantic consistency.
pub fn validate(ir: &TimelineIr) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check that all item lanes exist
    let lane_ids: std::collections::HashSet<&str> =
        ir.lanes.iter().map(|l| l.id.as_str()).collect();

    for item in &ir.items {
        let lane = match item {
            crate::ir::Item::Span { lane, .. }
            | crate::ir::Item::Event { lane, .. }
            | crate::ir::Item::EventRange { lane, .. } => lane,
        };
        if !lane_ids.contains(lane.as_str()) {
            warnings.push(format!("Item references unknown lane: {lane}"));
        }
    }

    // Check start > end for span and event_range items（月日精度を考慮）
    for item in &ir.items {
        match item {
            crate::ir::Item::Span {
                id,
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                ..
            } => {
                let s = sortable_tuple(*start, *start_month, *start_day);
                let e = sortable_tuple(*end, *end_month, *end_day);
                if s > e {
                    warnings.push(format!("Span \"{id}\" has start ({start}) > end ({end})"));
                }
            }
            crate::ir::Item::EventRange {
                id,
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                ..
            } => {
                let s = sortable_tuple(*start, *start_month, *start_day);
                let e = sortable_tuple(*end, *end_month, *end_day);
                if s > e {
                    warnings.push(format!(
                        "EventRange \"{id}\" has start ({start}) > end ({end})"
                    ));
                }
            }
            crate::ir::Item::Event { .. } => {}
        }
    }

    // Check range coherence（月日精度を考慮）
    let (range_start, range_end) = ir.meta.range;
    let r_start = sortable_tuple(
        range_start,
        ir.meta.range_start_month,
        ir.meta.range_start_day,
    );
    let r_end = sortable_tuple(range_end, ir.meta.range_end_month, ir.meta.range_end_day);
    if r_start >= r_end {
        warnings.push(format!(
            "Timeline range is invalid: {range_start}..{range_end}"
        ));
    }

    warnings
}
