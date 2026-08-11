use tdsl_parser::ast;

use crate::error::LoweringError;
use crate::ir::{Item, SourceSpan};

use super::context::LoweringContext;
use super::{compare_time_values, format_id_time, offset_to_line_col, source_str};

fn validate_link(link: &Option<String>) -> Result<Option<String>, LoweringError> {
    match link {
        Some(value) => {
            let trimmed = value.trim();
            let lower = trimmed.to_ascii_lowercase();
            if lower.starts_with("https://") || lower.starts_with("http://") {
                Ok(Some(trimmed.to_string()))
            } else {
                Err(LoweringError::InvalidItemLink(value.clone()))
            }
        }
        None => Ok(None),
    }
}

fn is_safe_color_value(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic() && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn validate_color(color: &Option<String>) -> Result<Option<String>, LoweringError> {
    match color {
        Some(value) => {
            let trimmed = value.trim();
            if is_safe_color_value(trimmed) {
                Ok(Some(trimmed.to_string()))
            } else {
                Err(LoweringError::InvalidItemColor(value.clone()))
            }
        }
        None => Ok(None),
    }
}

impl LoweringContext {
    /// Pass 2: Lower static items (span, event, event_range).
    pub(crate) fn pass2_static_items(&mut self, file: &ast::File, line_offsets: Option<&[usize]>) {
        for stmt in &file.statements {
            // エラーに添える位置。push_error() がこれを読む（#760）。
            self.current_span = Some(stmt.span);
            match &stmt.node {
                ast::Statement::Span(s) => {
                    if !self.lanes_map.contains_key(&s.lane_ref) {
                        let err = self.make_unknown_lane_error(&s.lane_ref);
                        self.push_error(err);
                        continue;
                    }
                    let id = s.props.id.clone().unwrap_or_else(|| {
                        format!("span:{}:{}", s.lane_ref, format_id_time(&s.start))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    let link = match validate_link(&s.props.link) {
                        Ok(link) => link,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
                    let color = match validate_color(&s.props.color) {
                        Ok(color) => color,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
                    if let Err(err) = compare_time_values(&s.start, &s.end) {
                        // 比較自体の結果(start>end等)は validate.rs の診断に任せるが、
                        // offset付き/なしの混在比較(MixedOffsetComparison)は曖昧さを残さず
                        // lowering 段階で明示エラーとする(ADR 0003 D2)。
                        self.push_error(err);
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
                        note: s.props.note.clone(),
                        link,
                        color,
                        start_month: s.start.month(),
                        start_day: s.start.day(),
                        start_hour: s.start.hour(),
                        start_minute: s.start.minute(),
                        start_second: s.start.second(),
                        start_offset_minutes: s.start.offset_minutes(),
                        end_month: s.end.month(),
                        end_day: s.end.day(),
                        end_hour: s.end.hour(),
                        end_minute: s.end.minute(),
                        end_second: s.end.second(),
                        end_offset_minutes: s.end.offset_minutes(),
                        end_open: s.end_open,
                        source_span,
                    });
                }
                ast::Statement::Event(e) => {
                    if !self.lanes_map.contains_key(&e.lane_ref) {
                        let err = self.make_unknown_lane_error(&e.lane_ref);
                        self.push_error(err);
                        continue;
                    }
                    let id = e.props.id.clone().unwrap_or_else(|| {
                        format!("event:{}:{}", e.lane_ref, format_id_time(&e.time))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    let link = match validate_link(&e.props.link) {
                        Ok(link) => link,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
                    let color = match validate_color(&e.props.color) {
                        Ok(color) => color,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
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
                        note: e.props.note.clone(),
                        link,
                        color,
                        time_month: e.time.month(),
                        time_day: e.time.day(),
                        time_hour: e.time.hour(),
                        time_minute: e.time.minute(),
                        time_second: e.time.second(),
                        time_offset_minutes: e.time.offset_minutes(),
                        source_span,
                    });
                }
                ast::Statement::EventRange(er) => {
                    if !self.lanes_map.contains_key(&er.lane_ref) {
                        let err = self.make_unknown_lane_error(&er.lane_ref);
                        self.push_error(err);
                        continue;
                    }
                    let id = er.props.id.clone().unwrap_or_else(|| {
                        format!("event_range:{}:{}", er.lane_ref, format_id_time(&er.start))
                    });
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    let link = match validate_link(&er.props.link) {
                        Ok(link) => link,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
                    let color = match validate_color(&er.props.color) {
                        Ok(color) => color,
                        Err(err) => {
                            self.push_error(err);
                            continue;
                        }
                    };
                    if let Err(err) = compare_time_values(&er.start, &er.end) {
                        // Span と同様、MixedOffsetComparison のみを lowering 段階で明示エラーとする。
                        self.push_error(err);
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
                        note: er.props.note.clone(),
                        link,
                        color,
                        start_month: er.start.month(),
                        start_day: er.start.day(),
                        start_hour: er.start.hour(),
                        start_minute: er.start.minute(),
                        start_second: er.start.second(),
                        start_offset_minutes: er.start.offset_minutes(),
                        end_month: er.end.month(),
                        end_day: er.end.day(),
                        end_hour: er.end.hour(),
                        end_minute: er.end.minute(),
                        end_second: er.end.second(),
                        end_offset_minutes: er.end.offset_minutes(),
                        end_open: er.end_open,
                        source_span,
                    });
                }
                _ => {}
            }
        }
        // ループを抜けたら位置を捨てる。以降のエラー（NoTimeline 等、
        // ファイル全体に対するもの）に直前 statement の位置を
        // 添えてしまわないため（#760）。
        self.current_span = None;
    }
}
