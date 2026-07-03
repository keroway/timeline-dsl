use tdsl_parser::ast;

use crate::error::LoweringError;
use crate::ir::{Meta, TimelineUnit, supported_timeline_units};

use super::context::LoweringContext;

impl LoweringContext {
    /// Pass 1: Collect timeline meta and lane declarations.
    pub(crate) fn pass1_declarations(&mut self, file: &ast::File, line_offsets: Option<&[usize]>) {
        for stmt in &file.statements {
            match &stmt.node {
                ast::Statement::Timeline(t) => {
                    if self.meta.is_some() {
                        self.errors.push(LoweringError::MultipleTimelines);
                        continue;
                    }
                    let color_map = t
                        .color_map
                        .iter()
                        .cloned()
                        .collect::<std::collections::HashMap<_, _>>();
                    let (
                        range_yy,
                        range_start_month,
                        range_start_day,
                        range_start_hour,
                        range_start_minute,
                        range_end_month,
                        range_end_day,
                        range_end_hour,
                        range_end_minute,
                    ) = match t.range.as_ref() {
                        Some(r) => (
                            (r.start.year(), r.end.year()),
                            r.start.month(),
                            r.start.day(),
                            r.start.hour(),
                            r.start.minute(),
                            r.end.month(),
                            r.end.day(),
                            r.end.hour(),
                            r.end.minute(),
                        ),
                        None => ((0, 2000), None, None, None, None, None, None, None, None),
                    };

                    let unit = match t.unit.as_deref() {
                        Some(value) => match TimelineUnit::parse(value) {
                            Some(unit) => unit,
                            None => {
                                self.errors.push(LoweringError::UnknownTimelineUnit {
                                    value: value.to_string(),
                                    expected: supported_timeline_units(),
                                });
                                continue;
                            }
                        },
                        None => TimelineUnit::Year,
                    };

                    self.meta = Some(Meta {
                        title: t.title.clone().unwrap_or_else(|| t.name.clone()),
                        unit: unit.as_str().to_string(),
                        range: range_yy,
                        range_start_month,
                        range_start_day,
                        range_start_hour,
                        range_start_minute,
                        range_end_month,
                        range_end_day,
                        range_end_hour,
                        range_end_minute,
                        calendar: t
                            .calendar
                            .clone()
                            .unwrap_or_else(|| "proleptic_gregorian".to_string()),
                        color_map,
                    });
                }
                ast::Statement::Lane(l) => {
                    self.lower_lane_decl(l, None, &stmt.span, line_offsets);
                }
                ast::Statement::Group(g) => {
                    for l in &g.lanes {
                        self.lower_lane_decl(l, Some(&g.label), &stmt.span, line_offsets);
                    }
                }
                ast::Statement::Template(t) => {
                    let key = t.alias.clone().unwrap_or_else(|| t.name.clone());
                    if self.templates.contains_key(&key) {
                        self.errors
                            .push(LoweringError::DuplicateTemplate(key.clone()));
                        continue;
                    }
                    self.templates.insert(key, t.clone());
                }
                _ => {}
            }
        }
        if self.meta.is_none() {
            self.errors.push(LoweringError::NoTimeline);
        }
    }
}
