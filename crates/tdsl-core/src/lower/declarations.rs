use tdsl_parser::ast;

use crate::error::LoweringError;
use crate::ir::{Meta, TimelineUnit, supported_timeline_units};

use super::compare_time_values;
use super::context::LoweringContext;

impl LoweringContext {
    /// Pass 1: Collect timeline meta and lane declarations.
    pub(crate) fn pass1_declarations(&mut self, file: &ast::File, line_offsets: Option<&[usize]>) {
        for stmt in &file.statements {
            // エラーに添える位置。push_error() がこれを読む（#760）。
            self.current_span = Some(stmt.span);
            match &stmt.node {
                ast::Statement::Timeline(t) => {
                    if self.meta.is_some() {
                        self.push_error(LoweringError::MultipleTimelines);
                        continue;
                    }
                    if let Some(range) = &t.range
                        && let Err(err) = compare_time_values(&range.start, &range.end)
                    {
                        self.push_error(err);
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
                        range_start_second,
                        range_start_offset_minutes,
                        range_end_month,
                        range_end_day,
                        range_end_hour,
                        range_end_minute,
                        range_end_second,
                        range_end_offset_minutes,
                    ) = match t.range.as_ref() {
                        Some(r) => (
                            (r.start.year(), r.end.year()),
                            r.start.month(),
                            r.start.day(),
                            r.start.hour(),
                            r.start.minute(),
                            r.start.second(),
                            r.start.offset_minutes(),
                            r.end.month(),
                            r.end.day(),
                            r.end.hour(),
                            r.end.minute(),
                            r.end.second(),
                            r.end.offset_minutes(),
                        ),
                        None => (
                            (0, 2000),
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ),
                    };

                    let unit = match t.unit.as_deref() {
                        Some(value) => match TimelineUnit::parse(value) {
                            Some(unit) => unit,
                            None => {
                                self.push_error(LoweringError::UnknownTimelineUnit {
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
                        range_start_second,
                        range_start_offset_minutes,
                        range_end_month,
                        range_end_day,
                        range_end_hour,
                        range_end_minute,
                        range_end_second,
                        range_end_offset_minutes,
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
                ast::Statement::Import(imp) => {
                    // alias は「`as` 指定 ?? source_type」。Pass 3 の
                    // `import_alias` と同じ規則で揃えること（片方だけ変えると
                    // 検出漏れになる）。
                    let alias = imp.alias.clone().unwrap_or_else(|| imp.source_type.clone());
                    if !self.import_aliases_seen.insert(alias.clone()) {
                        self.push_error(LoweringError::DuplicateImportAlias(alias));
                    }
                }
                ast::Statement::Template(t) => {
                    let key = t.alias.clone().unwrap_or_else(|| t.name.clone());
                    if self.templates.contains_key(&key) {
                        self.push_error(LoweringError::DuplicateTemplate(key.clone()));
                        continue;
                    }
                    self.templates.insert(key, t.clone());
                }
                _ => {}
            }
        }
        // ループを抜けたら位置を捨てる。以降のエラー（NoTimeline 等、
        // ファイル全体に対するもの）に直前 statement の位置を
        // 添えてしまわないため（#760）。
        self.current_span = None;
        if self.meta.is_none() {
            self.push_error(LoweringError::NoTimeline);
        }
    }
}
