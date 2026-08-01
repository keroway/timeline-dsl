use tdsl_core::ir::{Item, TimelineIr};

/// IR を CSV へエクスポートする。`import-csv` と対称な往復を可能にする。
///
/// 入力は `.tdsl` ソース（lowering して IR 化）または `.json`（IR を直接読み込み）。
/// 出力カラムは `lane,type,start,end,time,label,tags,id,source,origin` の 10 列。
/// `source` / `origin` も含めて `import-csv` で往復保持される（#608）。`source` は `<ident>:<QID>`
/// 形式（例 `wd:Q7209`）、`origin` は DSL の `ident` 文法を満たす必要があり、不正な値は
/// `import-csv` がエラーとして拒否する（silent に破棄しない、CLAUDE.md「No silent fallback」原則）。
pub(crate) fn cmd_export_csv(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let ir = load_ir_for_export(input, offline, cache_opts, wikidata_timeout)?;
    let csv = render_csv(&ir)?;

    if let Some(path) = output {
        std::fs::write(path, &csv)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("Written {} item(s) to {}", ir.items.len(), path.display());
    } else {
        print!("{csv}");
    }

    Ok(())
}

/// 入力拡張子で IR の取得方法を切り替える。
/// `.json` は IR JSON として読み込み、それ以外は `.tdsl` として lowering する。
fn load_ir_for_export(
    input: &std::path::Path,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    wikidata_timeout: std::time::Duration,
) -> Result<TimelineIr, String> {
    let is_json = input
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));

    if is_json {
        let json = super::read_source(input)?;
        serde_json::from_str(&json).map_err(|e| format!("Invalid IR JSON: {e}"))
    } else {
        super::build::load_ir(input, offline, cache_opts, wikidata_timeout)
    }
}

/// IR の年 + 月日・時分秒・offset 精度を `YYYY` / `YYYY-MM` / `YYYY-MM-DD` /
/// `YYYY-MM-DDTHH:MM` / `YYYY-MM-DDTHH:MM:SS` / `YYYY-MM-DDTHH:MM(:SS)±HH:MM`
/// 形式へ整形する（ADR 0003 D1/D4, #614: `import-csv` が同じ文字列を
/// `tdsl_parser::parse_time_literal` で再パースできるよう round-trip 可能な形式を保つ）。
fn format_time(
    year: i64,
    month: Option<u8>,
    day: Option<u8>,
    hour: Option<u8>,
    minute: Option<u8>,
    second: Option<u8>,
    offset_minutes: Option<i16>,
) -> String {
    match (month, day, hour, minute) {
        (Some(m), Some(d), Some(h), Some(min)) => {
            let base = match second {
                Some(s) => format!("{year:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}"),
                None => format!("{year:04}-{m:02}-{d:02}T{h:02}:{min:02}"),
            };
            match offset_minutes {
                Some(off) => format!("{base}{}", tdsl_core::lower::format_offset_suffix(off)),
                None => base,
            }
        }
        (Some(m), Some(d), _, _) => format!("{year:04}-{m:02}-{d:02}"),
        (Some(m), _, _, _) => format!("{year:04}-{m:02}"),
        _ => format!("{year}"),
    }
}

/// IR アイテムから CSV を生成する（IR を単一の真実源とする）。
fn render_csv(ir: &TimelineIr) -> Result<String, String> {
    let mut wtr = csv::WriterBuilder::new().from_writer(Vec::new());

    wtr.write_record([
        "lane", "type", "start", "end", "time", "label", "tags", "id", "source", "origin",
    ])
    .map_err(|e| format!("CSV write error: {e}"))?;

    for item in &ir.items {
        let row = item_to_row(item);
        wtr.write_record(&row)
            .map_err(|e| format!("CSV write error: {e}"))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| format!("CSV finalize error: {e}"))?;
    String::from_utf8(bytes).map_err(|e| format!("CSV is not valid UTF-8: {e}"))
}

/// 1 アイテムを CSV の 10 列レコードへ変換する。
/// タグは `|` 区切り（`import-csv` は `|` と `,` の両方を受理する）。
fn item_to_row(item: &Item) -> [String; 10] {
    let join_tags = |tags: &[String]| tags.join("|");

    match item {
        Item::Span {
            lane,
            start,
            end,
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
            label,
            tags,
            source,
            origin,
            id,
            ..
        } => [
            lane.clone(),
            "span".to_string(),
            format_time(
                *start,
                *start_month,
                *start_day,
                *start_hour,
                *start_minute,
                *start_second,
                *start_offset_minutes,
            ),
            format_time(
                *end,
                *end_month,
                *end_day,
                *end_hour,
                *end_minute,
                *end_second,
                *end_offset_minutes,
            ),
            String::new(),
            label.clone(),
            join_tags(tags),
            id.clone(),
            source.clone().unwrap_or_default(),
            origin.clone().unwrap_or_default(),
        ],
        Item::Event {
            lane,
            time,
            time_month,
            time_day,
            time_hour,
            time_minute,
            time_second,
            time_offset_minutes,
            label,
            tags,
            source,
            origin,
            id,
            ..
        } => [
            lane.clone(),
            "event".to_string(),
            String::new(),
            String::new(),
            format_time(
                *time,
                *time_month,
                *time_day,
                *time_hour,
                *time_minute,
                *time_second,
                *time_offset_minutes,
            ),
            label.clone(),
            join_tags(tags),
            id.clone(),
            source.clone().unwrap_or_default(),
            origin.clone().unwrap_or_default(),
        ],
        Item::EventRange {
            lane,
            start,
            end,
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
            label,
            tags,
            source,
            origin,
            id,
            ..
        } => [
            lane.clone(),
            "event_range".to_string(),
            format_time(
                *start,
                *start_month,
                *start_day,
                *start_hour,
                *start_minute,
                *start_second,
                *start_offset_minutes,
            ),
            format_time(
                *end,
                *end_month,
                *end_day,
                *end_hour,
                *end_minute,
                *end_second,
                *end_offset_minutes,
            ),
            String::new(),
            label.clone(),
            join_tags(tags),
            id.clone(),
            source.clone().unwrap_or_default(),
            origin.clone().unwrap_or_default(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tdsl_core::ir::{Lane, Meta, TimelineIr};

    fn sample_ir() -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "T".to_string(),
                unit: "year".to_string(),
                range: (-300, 2000),
                calendar: "proleptic_gregorian".to_string(),
                color_map: HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "a".to_string(),
                label: "Lane A".to_string(),
                kind: "custom".to_string(),
                order: 10,
                group: None,
                source_span: None,
            }],
            items: vec![
                Item::Span {
                    id: "span:a:0".to_string(),
                    lane: "a".to_string(),
                    start: 1939,
                    end: 1945,
                    label: "WW2".to_string(),
                    tags: vec!["war".to_string(), "global".to_string()],
                    source: None,
                    origin: None,
                    note: None,
                    link: None,
                    color: None,
                    start_month: Some(9),
                    start_day: Some(1),
                    start_hour: None,
                    start_minute: None,
                    start_second: None,
                    start_offset_minutes: None,
                    end_month: Some(9),
                    end_day: Some(2),
                    end_hour: None,
                    end_minute: None,
                    end_second: None,
                    end_offset_minutes: None,
                    end_open: false,
                    source_span: None,
                },
                Item::Event {
                    id: "event:a:1969".to_string(),
                    lane: "a".to_string(),
                    time: 1969,
                    label: "Apollo 11".to_string(),
                    tags: vec![],
                    source: Some("wd:Q1".to_string()),
                    origin: Some("wikidata".to_string()),
                    note: None,
                    link: None,
                    color: None,
                    time_month: Some(7),
                    time_day: Some(20),
                    time_hour: None,
                    time_minute: None,
                    time_second: None,
                    time_offset_minutes: None,
                    source_span: None,
                },
                Item::EventRange {
                    id: "event_range:a:-221".to_string(),
                    lane: "a".to_string(),
                    start: -221,
                    end: -206,
                    label: "Qin".to_string(),
                    tags: vec!["dynasty".to_string()],
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
                },
            ],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn render_csv_emits_header_and_rows() {
        let csv = render_csv(&sample_ir()).unwrap();
        let mut lines = csv.lines();
        assert_eq!(
            lines.next().unwrap(),
            "lane,type,start,end,time,label,tags,id,source,origin"
        );
        // span row: date precision, |-joined tags
        assert_eq!(
            lines.next().unwrap(),
            "a,span,1939-09-01,1945-09-02,,WW2,war|global,span:a:0,,"
        );
        // event row: time column only, source/origin populated
        assert_eq!(
            lines.next().unwrap(),
            "a,event,,,1969-07-20,Apollo 11,,event:a:1969,wd:Q1,wikidata"
        );
        // event_range row: negative years are year-precision only
        assert_eq!(
            lines.next().unwrap(),
            "a,event_range,-221,-206,,Qin,dynasty,event_range:a:-221,,"
        );
    }

    #[test]
    fn export_then_import_round_trips_items() {
        // export-csv → import-csv の往復で意味的に同値な IR が得られることを検証する。
        let ir = sample_ir();
        let csv = render_csv(&ir).unwrap();

        // import-csv が生成する item スニペットを得て、元の timeline+lane ヘッダと結合し再 lowering。
        let tmp = std::env::temp_dir().join(format!(
            "tdsl_export_roundtrip_{:?}_{}.csv",
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&tmp, &csv).unwrap();
        let items = super::super::init::parse_csv_items(&tmp).expect("import-csv parse");
        std::fs::remove_file(&tmp).ok();
        let snippet = super::super::init::render_imported_csv_items(&items);

        let reconstructed = format!(
            "timeline \"T\" {{\n    title \"T\";\n    unit year;\n    range -300..2000;\n    calendar proleptic_gregorian;\n}}\n\nlane \"Lane A\" as a {{ kind custom; order 10; }}\n\n{snippet}"
        );

        let file = tdsl_parser::parse(&reconstructed)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\n---\n{reconstructed}"));
        let ir2 = tdsl_core::lower::lower_static(&file).expect("re-lower must succeed");

        assert_eq!(ir2.items.len(), ir.items.len());

        // span: lane / 種別 / 月日精度 / ラベル / タグ / id が一致
        match &ir2.items[0] {
            Item::Span {
                lane,
                start,
                end,
                start_month,
                start_day,
                end_month,
                end_day,
                label,
                tags,
                id,
                ..
            } => {
                assert_eq!(lane, "a");
                assert_eq!((*start, *end), (1939, 1945));
                assert_eq!((*start_month, *start_day), (Some(9), Some(1)));
                assert_eq!((*end_month, *end_day), (Some(9), Some(2)));
                assert_eq!(label, "WW2");
                assert_eq!(tags, &["war", "global"]);
                assert_eq!(id, "span:a:0");
            }
            other => panic!("expected span, got {other:?}"),
        }
        // event: 時刻精度が往復。#608: source/origin（wd:Q1 / wikidata）も往復する。
        match &ir2.items[1] {
            Item::Event {
                time,
                time_month,
                time_day,
                label,
                source,
                origin,
                ..
            } => {
                assert_eq!(*time, 1969);
                assert_eq!((*time_month, *time_day), (Some(7), Some(20)));
                assert_eq!(label, "Apollo 11");
                assert_eq!(source.as_deref(), Some("wd:Q1"));
                assert_eq!(origin.as_deref(), Some("wikidata"));
            }
            other => panic!("expected event, got {other:?}"),
        }
        // event_range: 紀元前は year 精度
        match &ir2.items[2] {
            Item::EventRange {
                start, end, label, ..
            } => {
                assert_eq!((*start, *end), (-221, -206));
                assert_eq!(label, "Qin");
            }
            other => panic!("expected event_range, got {other:?}"),
        }
    }

    // ─── #614 (ADR 0003): 秒・オフセットの export-csv → import-csv round-trip ───

    fn ir_with_second_and_offset() -> TimelineIr {
        TimelineIr {
            meta: Meta {
                title: "Sec".to_string(),
                unit: "second".to_string(),
                range: (2024, 2024),
                calendar: "proleptic_gregorian".to_string(),
                color_map: HashMap::new(),
                ..Default::default()
            },
            lanes: vec![Lane {
                id: "a".to_string(),
                label: "Lane A".to_string(),
                kind: "custom".to_string(),
                order: 10,
                group: None,
                source_span: None,
            }],
            items: vec![Item::EventRange {
                id: "event_range:a:sec".to_string(),
                lane: "a".to_string(),
                start: 2024,
                end: 2024,
                label: "Countdown".to_string(),
                tags: vec![],
                source: None,
                origin: None,
                note: None,
                link: None,
                color: None,
                start_month: Some(1),
                start_day: Some(1),
                start_hour: Some(10),
                start_minute: Some(0),
                start_second: Some(30),
                start_offset_minutes: Some(540),
                end_month: Some(1),
                end_day: Some(1),
                end_hour: Some(10),
                end_minute: Some(1),
                end_second: Some(0),
                end_offset_minutes: Some(540),
                end_open: false,
                source_span: None,
            }],
            imports: vec![],
            sources: vec![],
        }
    }

    #[test]
    fn render_csv_includes_second_and_offset_in_time_columns() {
        let csv = render_csv(&ir_with_second_and_offset()).unwrap();
        let mut lines = csv.lines();
        lines.next(); // header
        let row = lines.next().unwrap();
        assert!(
            row.contains("2024-01-01T10:00:30+09:00"),
            "expected start with second+offset in CSV row: {row}"
        );
        assert!(
            row.contains("2024-01-01T10:01:00+09:00"),
            "expected end with second+offset in CSV row: {row}"
        );
    }

    #[test]
    fn export_then_import_round_trips_second_and_offset() {
        let ir = ir_with_second_and_offset();
        let csv = render_csv(&ir).unwrap();

        let tmp = std::env::temp_dir().join(format!(
            "tdsl_export_roundtrip_sec_{:?}_{}.csv",
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&tmp, &csv).unwrap();
        let items = super::super::init::parse_csv_items(&tmp).expect("import-csv parse");
        std::fs::remove_file(&tmp).ok();
        let snippet = super::super::init::render_imported_csv_items(&items);

        let reconstructed = format!(
            "timeline \"Sec\" {{\n    title \"Sec\";\n    unit second;\n    range 2024..2024;\n    calendar proleptic_gregorian;\n}}\n\nlane \"Lane A\" as a {{ kind custom; order 10; }}\n\n{snippet}"
        );

        let file = tdsl_parser::parse(&reconstructed)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\n---\n{reconstructed}"));
        let ir2 = tdsl_core::lower::lower_static(&file).expect("re-lower must succeed");

        assert_eq!(ir2.items.len(), 1);
        match &ir2.items[0] {
            Item::EventRange {
                start_second,
                start_offset_minutes,
                end_second,
                end_offset_minutes,
                ..
            } => {
                assert_eq!(*start_second, Some(30));
                assert_eq!(*start_offset_minutes, Some(540));
                assert_eq!(*end_second, Some(0));
                assert_eq!(*end_offset_minutes, Some(540));
            }
            other => panic!("expected event_range, got {other:?}"),
        }
    }
}
