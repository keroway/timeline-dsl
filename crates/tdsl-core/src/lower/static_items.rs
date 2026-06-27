use tdsl_parser::ast;

use crate::ir::{Item, SourceSpan};

use super::context::LoweringContext;
use super::{format_id_time, offset_to_line_col, source_str};

impl LoweringContext {
    /// Pass 2: Lower static items (span, event, event_range).
    pub(crate) fn pass2_static_items(&mut self, file: &ast::File, line_offsets: Option<&[usize]>) {
        for stmt in &file.statements {
            match &stmt.node {
                ast::Statement::Span(s) => {
                    if !self.lanes_map.contains_key(&s.lane_ref) {
                        let err = self.make_unknown_lane_error(&s.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = s.props.id.clone().unwrap_or_else(|| {
                        format!("span:{}:{}", s.lane_ref, format_id_time(&s.start))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&s.props.source);
                    let source_span = line_offsets.map(|lo| {
                        let (line, col_start) = offset_to_line_col(stmt.span.start, lo);
                        let (_, col_end) = offset_to_line_col(stmt.span.end, lo);
                        SourceSpan {
                            line,
                            col_start,
                            col_end,
                        }
                    });
                    self.items.push(Item::Span {
                        id,
                        lane: s.lane_ref.clone(),
                        start: s.start.year(),
                        end: s.end.year(),
                        label: s.label.clone(),
                        tags: s.props.tags.clone(),
                        source: source_str(&s.props.source),
                        origin: s.props.origin.clone(),
                        start_month: s.start.month(),
                        start_day: s.start.day(),
                        start_hour: s.start.hour(),
                        start_minute: s.start.minute(),
                        end_month: s.end.month(),
                        end_day: s.end.day(),
                        end_hour: s.end.hour(),
                        end_minute: s.end.minute(),
                        source_span,
                    });
                }
                ast::Statement::Event(e) => {
                    if !self.lanes_map.contains_key(&e.lane_ref) {
                        let err = self.make_unknown_lane_error(&e.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = e.props.id.clone().unwrap_or_else(|| {
                        format!("event:{}:{}", e.lane_ref, format_id_time(&e.time))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&e.props.source);
                    let source_span = line_offsets.map(|lo| {
                        let (line, col_start) = offset_to_line_col(stmt.span.start, lo);
                        let (_, col_end) = offset_to_line_col(stmt.span.end, lo);
                        SourceSpan {
                            line,
                            col_start,
                            col_end,
                        }
                    });
                    self.items.push(Item::Event {
                        id,
                        lane: e.lane_ref.clone(),
                        time: e.time.year(),
                        label: e.label.clone(),
                        tags: e.props.tags.clone(),
                        source: source_str(&e.props.source),
                        origin: e.props.origin.clone(),
                        time_month: e.time.month(),
                        time_day: e.time.day(),
                        time_hour: e.time.hour(),
                        time_minute: e.time.minute(),
                        source_span,
                    });
                }
                ast::Statement::EventRange(er) => {
                    if !self.lanes_map.contains_key(&er.lane_ref) {
                        let err = self.make_unknown_lane_error(&er.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = er.props.id.clone().unwrap_or_else(|| {
                        format!("event_range:{}:{}", er.lane_ref, format_id_time(&er.start))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&er.props.source);
                    let source_span = line_offsets.map(|lo| {
                        let (line, col_start) = offset_to_line_col(stmt.span.start, lo);
                        let (_, col_end) = offset_to_line_col(stmt.span.end, lo);
                        SourceSpan {
                            line,
                            col_start,
                            col_end,
                        }
                    });
                    self.items.push(Item::EventRange {
                        id,
                        lane: er.lane_ref.clone(),
                        start: er.start.year(),
                        end: er.end.year(),
                        label: er.label.clone(),
                        tags: er.props.tags.clone(),
                        source: source_str(&er.props.source),
                        origin: er.props.origin.clone(),
                        start_month: er.start.month(),
                        start_day: er.start.day(),
                        start_hour: er.start.hour(),
                        start_minute: er.start.minute(),
                        end_month: er.end.month(),
                        end_day: er.end.day(),
                        end_hour: er.end.hour(),
                        end_minute: er.end.minute(),
                        source_span,
                    });
                }
                _ => {}
            }
        }
    }
}
