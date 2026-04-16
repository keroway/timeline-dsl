use crate::ir::TimelineIr;

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

    // Check range coherence
    let (range_start, range_end) = ir.meta.range;
    if range_start >= range_end {
        warnings.push(format!(
            "Timeline range is invalid: {range_start}..{range_end}"
        ));
    }

    warnings
}
