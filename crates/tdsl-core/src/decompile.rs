use std::fmt::Write;

use crate::ir::{Item, TimelineIr};

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// IR の年 + 月日・時分精度を `YYYY` / `YYYY-MM` / `YYYY-MM-DD` / `YYYY-MM-DDTHH:MM` 形式の文字列に整形する。
fn format_time(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
) -> String {
    match (month, day, hour, minute) {
        (Some(m), Some(d), Some(h), Some(min)) => {
            format!("{year:04}-{m:02}-{d:02}T{h:02}:{min:02}")
        }
        (Some(m), Some(d), _, _) => format!("{year:04}-{m:02}-{d:02}"),
        (Some(m), _, _, _) => format!("{year:04}-{m:02}"),
        _ => format!("{year}"),
    }
}

/// IR (`TimelineIr`) を `.tdsl` ソース文字列へ逆変換する。
///
/// **コメント非対応**: decompile は JSON IR を起点とし、IR にはコメント情報が
/// 一切含まれない（コメントは AST 段階で失われ IR には現れない）。そのため
/// 元ソースにあった `//` 行コメント・`/* */` ブロックコメントは復元できない。
/// これは IR を単一の真実とする設計上の恒久的な制約であり、将来も変わらない。
pub fn decompile(ir: &TimelineIr) -> String {
    // 以降の `write!` / `writeln!` は `String` への書き込みであり、`fmt::Write` の
    // 実装は決して失敗しない（infallible）。そのため `.unwrap()` がパニックする
    // 経路は存在しない（AGENTS §4.1 の unwrap 規約に対する明示的な理由付け）。
    let mut out = String::new();

    let title = escape(&ir.meta.title);
    writeln!(out, r#"timeline "{title}" {{"#).unwrap();
    writeln!(out, r#"    title "{title}";"#).unwrap();
    writeln!(out, "    unit {};", ir.meta.unit).unwrap();
    let range_start_str = format_time(
        ir.meta.range.0,
        ir.meta.range_start_month,
        ir.meta.range_start_day,
        ir.meta.range_start_hour,
        ir.meta.range_start_minute,
    );
    let range_end_str = format_time(
        ir.meta.range.1,
        ir.meta.range_end_month,
        ir.meta.range_end_day,
        ir.meta.range_end_hour,
        ir.meta.range_end_minute,
    );
    writeln!(out, "    range {range_start_str}..{range_end_str};").unwrap();
    writeln!(out, "    calendar {};", ir.meta.calendar).unwrap();
    if !ir.meta.color_map.is_empty() {
        let mut entries: Vec<_> = ir.meta.color_map.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());
        writeln!(out, "    color_map {{").unwrap();
        for (k, v) in &entries {
            writeln!(out, r#"        {k}: "{v}";"#).unwrap();
        }
        writeln!(out, "    }}").unwrap();
    }
    writeln!(out, "}}").unwrap();

    for lane in &ir.lanes {
        out.push('\n');
        let label = escape(&lane.label);
        write!(
            out,
            r#"lane "{label}" as {} {{ kind {}; order {}; }}"#,
            lane.id, lane.kind, lane.order
        )
        .unwrap();
        out.push('\n');
    }

    for item in &ir.items {
        out.push('\n');
        match item {
            Item::Span {
                lane,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                end_month,
                end_day,
                end_hour,
                end_minute,
                label,
                tags,
                source,
                origin,
                id,
                ..
            } => {
                let props = render_props(id, tags, source, origin);
                let start_s =
                    format_time(*start, *start_month, *start_day, *start_hour, *start_minute);
                let end_s = format_time(*end, *end_month, *end_day, *end_hour, *end_minute);
                writeln!(
                    out,
                    r#"span {lane} {start_s}..{end_s} "{}" {props};"#,
                    escape(label)
                )
                .unwrap();
            }
            Item::Event {
                lane,
                time,
                time_month,
                time_day,
                time_hour,
                time_minute,
                label,
                tags,
                source,
                origin,
                id,
                ..
            } => {
                let props = render_props(id, tags, source, origin);
                let time_s = format_time(*time, *time_month, *time_day, *time_hour, *time_minute);
                writeln!(out, r#"event {lane} {time_s} "{}" {props};"#, escape(label)).unwrap();
            }
            Item::EventRange {
                lane,
                start,
                end,
                start_month,
                start_day,
                start_hour,
                start_minute,
                end_month,
                end_day,
                end_hour,
                end_minute,
                label,
                tags,
                source,
                origin,
                id,
                ..
            } => {
                let props = render_props(id, tags, source, origin);
                let start_s =
                    format_time(*start, *start_month, *start_day, *start_hour, *start_minute);
                let end_s = format_time(*end, *end_month, *end_day, *end_hour, *end_minute);
                writeln!(
                    out,
                    r#"event_range {lane} {start_s}..{end_s} "{}" {props};"#,
                    escape(label)
                )
                .unwrap();
            }
        }
    }

    out
}

fn render_props(
    id: &str,
    tags: &[String],
    source: &Option<String>,
    origin: &Option<String>,
) -> String {
    let mut parts = Vec::new();

    if !tags.is_empty() {
        let joined = tags
            .iter()
            .map(|t| format!(r#""{}""#, escape(t)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("tags [{joined}];"));
    }
    if let Some(src) = source {
        parts.push(format!("source {src};"));
    }
    parts.push(format!(r#"id "{}";"#, escape(id)));
    if let Some(orig) = origin {
        parts.push(format!("origin {orig};"));
    }

    format!("{{ {} }}", parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Item, Lane, Meta, TimelineIr};
    use std::collections::HashMap;

    fn make_ir() -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "Test Timeline".to_string(),
                unit: "year".to_string(),
                range: (-100, 500),
                calendar: "proleptic_gregorian".to_string(),
                color_map: HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "a".to_string(),
                label: "Lane A".to_string(),
                kind: "dynasty".to_string(),
                order: 10,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "span:a:0".to_string(),
                    lane: "a".to_string(),
                    start: 0,
                    end: 200,
                    label: "Span One".to_string(),
                    tags: vec!["tag1".to_string()],
                    source: None,
                    origin: None,
                    start_month: None,
                    start_day: None,
                    start_hour: None,
                    start_minute: None,
                    end_month: None,
                    end_day: None,
                    end_hour: None,
                    end_minute: None,
                    source_span: None,
                },
                Item::Event {
                    id: "event:a:100".to_string(),
                    lane: "a".to_string(),
                    time: 100,
                    label: "Event One".to_string(),
                    tags: vec![],
                    source: Some("wd:Q1".to_string()),
                    origin: Some("imported".to_string()),
                    time_month: None,
                    time_day: None,
                    time_hour: None,
                    time_minute: None,
                    source_span: None,
                },
                Item::EventRange {
                    id: "event_range:a:50".to_string(),
                    lane: "a".to_string(),
                    start: 50,
                    end: 150,
                    label: "Range One".to_string(),
                    tags: vec!["war".to_string(), "conflict".to_string()],
                    source: None,
                    origin: None,
                    start_month: None,
                    start_day: None,
                    start_hour: None,
                    start_minute: None,
                    end_month: None,
                    end_day: None,
                    end_hour: None,
                    end_minute: None,
                    source_span: None,
                },
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn decompile_produces_parseable_output() {
        let ir = make_ir();
        let tdsl = decompile(&ir);

        let file = tdsl_parser::parse(&tdsl).expect("decompiled output must parse");
        let ir2 =
            crate::lower::lower_static(&file).expect("decompiled output must lower without errors");

        assert_eq!(ir2.meta.title, "Test Timeline");
        assert_eq!(ir2.meta.range, (-100, 500));
        assert_eq!(ir2.lanes.len(), 1);
        assert_eq!(ir2.items.len(), 3);
    }

    #[test]
    fn decompile_roundtrip_preserves_meta() {
        let ir = make_ir();
        let tdsl = decompile(&ir);

        let file = tdsl_parser::parse(&tdsl).unwrap();
        let ir2 = crate::lower::lower_static(&file).unwrap();

        assert_eq!(ir2.meta.unit, "year");
        assert_eq!(ir2.meta.calendar, "proleptic_gregorian");
    }

    #[test]
    fn decompile_roundtrip_preserves_lanes() {
        let ir = make_ir();
        let tdsl = decompile(&ir);

        let file = tdsl_parser::parse(&tdsl).unwrap();
        let ir2 = crate::lower::lower_static(&file).unwrap();

        assert_eq!(ir2.lanes[0].id, "a");
        assert_eq!(ir2.lanes[0].label, "Lane A");
        assert_eq!(ir2.lanes[0].kind, "dynasty");
        assert_eq!(ir2.lanes[0].order, 10);
    }

    #[test]
    fn decompile_roundtrip_preserves_items() {
        let ir = make_ir();
        let tdsl = decompile(&ir);

        let file = tdsl_parser::parse(&tdsl).unwrap();
        let ir2 = crate::lower::lower_static(&file).unwrap();

        match &ir2.items[0] {
            Item::Span {
                start,
                end,
                label,
                tags,
                ..
            } => {
                assert_eq!(*start, 0);
                assert_eq!(*end, 200);
                assert_eq!(label, "Span One");
                assert_eq!(tags, &["tag1"]);
            }
            _ => panic!("expected span"),
        }
        match &ir2.items[1] {
            Item::Event {
                time,
                label,
                source,
                ..
            } => {
                assert_eq!(*time, 100);
                assert_eq!(label, "Event One");
                assert_eq!(source.as_deref(), Some("wd:Q1"));
            }
            _ => panic!("expected event"),
        }
        match &ir2.items[2] {
            Item::EventRange {
                start,
                end,
                label,
                tags,
                ..
            } => {
                assert_eq!(*start, 50);
                assert_eq!(*end, 150);
                assert_eq!(label, "Range One");
                assert_eq!(tags, &["war", "conflict"]);
            }
            _ => panic!("expected event_range"),
        }
    }

    #[test]
    fn decompile_output_contains_escaped_quotes_in_string() {
        let ir = TimelineIr {
            meta: Meta {
                title: r#"Title with "quotes""#.to_string(),
                unit: "year".to_string(),
                range: (0, 100),
                calendar: "proleptic_gregorian".to_string(),
                color_map: HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "x".to_string(),
                label: "X".to_string(),
                kind: "custom".to_string(),
                order: 1,
                group: None,
                source_span: None,
            }],
            items: vec![],
            imports: vec![],
            sources: vec![],
        };

        let tdsl = decompile(&ir);
        assert!(tdsl.contains(r#"timeline "Title with \"quotes\"""#));
        tdsl_parser::parse(&tdsl).expect("escaped output must parse");
    }
}
