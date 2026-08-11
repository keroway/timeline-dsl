use std::fmt::Write;

/// 最小限の .tdsl テンプレートを生成する。
pub(crate) fn cmd_init(
    output: Option<&std::path::Path>,
    timeline: &str,
    range_start: i64,
    range_end: i64,
    lanes: &str,
) -> Result<(), String> {
    let title = timeline.trim();
    if title.is_empty() {
        return Err("timeline must not be empty".to_string());
    }
    if range_start >= range_end {
        return Err("range_start must be less than range_end".to_string());
    }

    let lane_specs = parse_lane_specs(lanes)?;
    let doc = render_init_tdsl(title, range_start, range_end, &lane_specs);

    if let Some(path) = output {
        std::fs::write(path, &doc)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("Written template to {}", path.display());
    } else {
        println!("{doc}");
    }

    Ok(())
}

/// CSV から timeline アイテムをインポートして .tdsl スニペットを生成する。
pub(crate) fn cmd_import_csv(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    append: Option<&std::path::Path>,
) -> Result<(), String> {
    let items = parse_csv_items(input)?;
    let snippet = render_imported_csv_items(&items);

    if let Some(path) = append {
        let existing = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let mut out = String::with_capacity(existing.len() + snippet.len() + 2);
        out.push_str(&existing);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.ends_with("\n\n") {
            out.push('\n');
        }
        out.push_str(&snippet);
        std::fs::write(path, out)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("Appended {} item(s) to {}", items.len(), path.display());
        return Ok(());
    }

    if let Some(path) = output {
        std::fs::write(path, &snippet)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("Written {} item(s) to {}", items.len(), path.display());
        return Ok(());
    }

    println!("{snippet}");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CsvItemType {
    Span,
    Event,
    EventRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImportedCsvItem {
    lane: String,
    item_type: CsvItemType,
    start: Option<tdsl_parser::ast::TimeValue>,
    end: Option<tdsl_parser::ast::TimeValue>,
    time: Option<tdsl_parser::ast::TimeValue>,
    label: String,
    tags: Vec<String>,
    id: Option<String>,
    /// #608: `export-csv` が出力する `source` 列（例 `wd:Q7209`）を任意列として受理し、
    /// 往復で保持する。
    source: Option<tdsl_parser::ast::SourceRef>,
    /// #608: `export-csv` が出力する `origin` 列（例 `wikidata`）を任意列として受理し、
    /// 往復で保持する。
    origin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InitLaneSpec {
    label: String,
    alias: Option<String>,
}

fn parse_lane_specs(input: &str) -> Result<Vec<InitLaneSpec>, String> {
    let mut lanes = Vec::new();
    let mut seen_aliases = std::collections::HashSet::new();
    for part in input.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (label_raw, alias_raw) = if let Some((label, alias)) = trimmed.split_once(':') {
            (label.trim(), Some(alias.trim()))
        } else {
            (trimmed, None)
        };
        if label_raw.is_empty() {
            return Err("lane label must not be empty".to_string());
        }

        let alias = match alias_raw {
            Some("") => return Err("lane alias must not be empty".to_string()),
            Some(a) => {
                if !is_valid_ident(a) {
                    return Err(format!(
                        "invalid lane alias `{a}` (must match [A-Za-z_][A-Za-z0-9_-]*)"
                    ));
                }
                if !seen_aliases.insert(a.to_string()) {
                    return Err(format!("duplicate lane alias `{a}`"));
                }
                Some(a.to_string())
            }
            None => None,
        };

        lanes.push(InitLaneSpec {
            label: label_raw.to_string(),
            alias,
        });
    }
    Ok(lanes)
}

fn render_init_tdsl(
    title: &str,
    range_start: i64,
    range_end: i64,
    lane_specs: &[InitLaneSpec],
) -> String {
    // 以降の write!/writeln! は書き込み先が String で、std::fmt::Write の実装は
    // 決して失敗しない（infallible）。返る Result を握り潰す unwrap は
    // その意味で安全（implementation-strict.md §4.1: 理由を 1 行残す）。
    let mut out = String::new();
    let escaped_title = super::escape_tdsl_string(title);
    writeln!(
        out,
        r#"timeline "{title}" {{
    title "{title}";
    unit year;
    range {start}..{end};
    calendar proleptic_gregorian;
}}"#,
        title = escaped_title,
        start = range_start,
        end = range_end
    )
    .unwrap();

    if lane_specs.is_empty() {
        out.push_str("\n// lane を追加してください\n");
        return out;
    }

    out.push('\n');
    let mut lane_alias_seen = std::collections::HashSet::new();
    for (i, lane) in lane_specs.iter().enumerate() {
        let alias = if let Some(alias) = &lane.alias {
            // 明示的なエイリアスは make_unique_alias を通さないため手動で登録
            lane_alias_seen.insert(alias.clone());
            alias.clone()
        } else {
            let base = super::slug_ascii(&lane.label);
            let seed = if base.is_empty() {
                format!("lane_{}", i + 1)
            } else {
                base
            };
            // make_unique_alias 内部で lane_alias_seen に挿入済み
            super::make_unique_alias(&seed, &mut lane_alias_seen)
        };
        writeln!(
            out,
            r#"lane "{label}" as {alias} {{ kind custom; order {order}; }}"#,
            label = super::escape_tdsl_string(&lane.label),
            alias = alias,
            order = ((i as i64) + 1) * 10
        )
        .unwrap();
    }

    out
}

fn is_valid_ident(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn parse_csv_items(path: &std::path::Path) -> Result<Vec<ImportedCsvItem>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

    let headers = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV header from {}: {e}", path.display()))?
        .clone();
    let required = [
        "lane", "type", "start", "end", "time", "label", "tags", "id",
    ];
    for key in required {
        if !headers.iter().any(|h| h == key) {
            return Err(format!("CSV is missing required column: {key}"));
        }
    }
    // #608: source/origin は任意列だが、重複ヘッダは (既存の必須列も含め) 一律に拒否する。
    for key in required.iter().chain(["source", "origin"].iter()) {
        let count = headers.iter().filter(|h| h == key).count();
        if count > 1 {
            return Err(format!("CSV header `{key}` is duplicated"));
        }
    }
    let has_source_column = headers.iter().any(|h| h == "source");
    let has_origin_column = headers.iter().any(|h| h == "origin");

    let mut items = Vec::new();
    for (idx, record) in reader.records().enumerate() {
        let row_no = idx + 2;
        let record = record.map_err(|e| format!("CSV row {row_no}: {e}"))?;
        let get = |name: &str| -> Result<String, String> {
            let pos = headers
                .iter()
                .position(|h| h == name)
                .ok_or_else(|| format!("CSV is missing required column: {name}"))?;
            Ok(record.get(pos).unwrap_or("").trim().to_string())
        };
        // 任意列用: ヘッダ自体がなければ常に空文字列を返す（旧8列CSVとの後方互換）。
        let get_optional = |name: &str, present: bool| -> String {
            if !present {
                return String::new();
            }
            let pos = headers.iter().position(|h| h == name);
            match pos {
                Some(pos) => record.get(pos).unwrap_or("").trim().to_string(),
                None => String::new(),
            }
        };

        let lane = get("lane")?;
        if lane.is_empty() {
            return Err(format!("CSV row {row_no}: lane must not be empty"));
        }

        let label = get("label")?;
        if label.is_empty() {
            return Err(format!("CSV row {row_no}: label must not be empty"));
        }

        let row_type = get("type")?.to_ascii_lowercase();
        let item_type = match row_type.as_str() {
            "span" => CsvItemType::Span,
            "event" => CsvItemType::Event,
            "event_range" => CsvItemType::EventRange,
            other => {
                return Err(format!(
                    "CSV row {row_no}: invalid type `{other}` (expected span/event/event_range)"
                ));
            }
        };

        let start_raw = get("start")?;
        let end_raw = get("end")?;
        let time_raw = get("time")?;

        let parse_required_time = |field: &str,
                                   raw: &str|
         -> Result<tdsl_parser::ast::TimeValue, String> {
            if raw.is_empty() {
                return Err(format!("CSV row {row_no}: {field} must not be empty"));
            }
            tdsl_parser::parse_time_literal(raw).map_err(|e| {
                format!(
                    "CSV row {row_no}: {field} must be YYYY-MM-DDTHH:MM, YYYY-MM-DD, YYYY-MM, or YYYY (got `{raw}`): {e}"
                )
            })
        };

        let (start, end, time) = match item_type {
            CsvItemType::Span | CsvItemType::EventRange => (
                Some(parse_required_time("start", &start_raw)?),
                Some(parse_required_time("end", &end_raw)?),
                None,
            ),
            CsvItemType::Event => (None, None, Some(parse_required_time("time", &time_raw)?)),
        };

        let tags_raw = get("tags")?;
        let tags: Vec<String> = tags_raw
            .split(['|', ','])
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        let id = {
            let raw = get("id")?;
            if raw.is_empty() { None } else { Some(raw) }
        };

        // #608: source / origin は任意列。値があれば DSL の source_ref / ident 文法で検証し、
        // 不正を silent に破棄しない（CLAUDE.md「No silent fallback」原則）。
        let source = {
            let raw = get_optional("source", has_source_column);
            if raw.is_empty() {
                None
            } else {
                Some(tdsl_parser::parse_source_ref_literal(&raw).map_err(|e| {
                    format!(
                        "CSV row {row_no}: source must be `<ident>:<QID>` (e.g. `wd:Q7209`), got `{raw}`: {e}"
                    )
                })?)
            }
        };
        let origin = {
            let raw = get_optional("origin", has_origin_column);
            if raw.is_empty() {
                None
            } else {
                Some(tdsl_parser::parse_ident_literal(&raw).map_err(|e| {
                    format!("CSV row {row_no}: origin must be a valid identifier, got `{raw}`: {e}")
                })?)
            }
        };
        // #608 provenance 契約: origin=wikidata は source=wd:Q<id> を必須とする。
        // wd:Q… source で他/無 origin の場合は static provenance として保持（書き換えない）。
        if origin.as_deref() == Some("wikidata") {
            let source_is_wd = matches!(&source, Some(s) if s.prefix == "wd");
            if !source_is_wd {
                return Err(format!(
                    "CSV row {row_no}: origin=wikidata requires a source column value in the form `wd:Q<id>`"
                ));
            }
        }

        items.push(ImportedCsvItem {
            lane,
            item_type,
            start,
            end,
            time,
            label,
            tags,
            id,
            source,
            origin,
        });
    }

    if items.is_empty() {
        return Err(format!("CSV {} contains no data rows", path.display()));
    }

    Ok(items)
}

pub(crate) fn render_imported_csv_items(items: &[ImportedCsvItem]) -> String {
    let mut out = String::new();
    for item in items {
        let mut options = String::new();
        if !item.tags.is_empty() {
            let tags = item
                .tags
                .iter()
                .map(|t| format!(r#""{}""#, super::escape_tdsl_string(t)))
                .collect::<Vec<_>>()
                .join(", ");
            write!(options, "tags [{tags}]; ").unwrap();
        }
        if let Some(source) = &item.source {
            // SourceRef の Display は `<prefix>:<qid>`（例: `wd:Q7209`）を出力する。
            write!(options, "source {source}; ").unwrap();
        }
        if let Some(id) = &item.id {
            write!(options, r#"id "{}"; "#, super::escape_tdsl_string(id)).unwrap();
        }
        if let Some(origin) = &item.origin {
            write!(options, "origin {origin}; ").unwrap();
        }
        let block_options = if options.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {} }}", options)
        };

        match item.item_type {
            CsvItemType::Span => {
                writeln!(
                    out,
                    r#"span {lane} {start}..{end} "{label}" {options};"#,
                    lane = item.lane,
                    start = item.start.as_ref().expect("validated start"),
                    end = item.end.as_ref().expect("validated end"),
                    label = super::escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
            CsvItemType::Event => {
                writeln!(
                    out,
                    r#"event {lane} {time} "{label}" {options};"#,
                    lane = item.lane,
                    time = item.time.as_ref().expect("validated time"),
                    label = super::escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
            CsvItemType::EventRange => {
                writeln!(
                    out,
                    r#"event_range {lane} {start}..{end} "{label}" {options};"#,
                    lane = item.lane,
                    start = item.start.as_ref().expect("validated start"),
                    end = item.end.as_ref().expect("validated end"),
                    label = super::escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_csv(contents: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = std::thread::current().id();
        let path = std::env::temp_dir().join(format!("tdsl_cli_test_{thread_id:?}_{nanos}.csv"));
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn render_init_tdsl_generates_valid_template_with_lanes() {
        let doc = render_init_tdsl(
            "架空世界年表",
            1000,
            1300,
            &[
                InitLaneSpec {
                    label: "王国".to_string(),
                    alias: Some("kingdom".to_string()),
                },
                InitLaneSpec {
                    label: "事件".to_string(),
                    alias: Some("incidents".to_string()),
                },
            ],
        );
        assert!(doc.contains(r#"timeline "架空世界年表""#));
        assert!(doc.contains("range 1000..1300;"));
        assert!(doc.contains(r#"lane "王国" as kingdom"#));
        assert!(doc.contains(r#"lane "事件" as incidents"#));
    }

    #[test]
    fn parse_lane_specs_accepts_alias_syntax() {
        let lanes = parse_lane_specs("王国:kingdom,事件:incidents").unwrap();
        assert_eq!(
            lanes,
            vec![
                InitLaneSpec {
                    label: "王国".to_string(),
                    alias: Some("kingdom".to_string())
                },
                InitLaneSpec {
                    label: "事件".to_string(),
                    alias: Some("incidents".to_string())
                }
            ]
        );
    }

    #[test]
    fn parse_lane_specs_rejects_invalid_alias() {
        let err = parse_lane_specs("王国:123bad").unwrap_err();
        assert!(err.contains("invalid lane alias"));
    }

    #[test]
    fn parse_lane_specs_label_only_has_no_alias() {
        let lanes = parse_lane_specs("王国,事件").unwrap();
        assert_eq!(lanes.len(), 2);
        assert_eq!(lanes[0].label, "王国");
        assert!(lanes[0].alias.is_none());
        assert_eq!(lanes[1].label, "事件");
        assert!(lanes[1].alias.is_none());
    }

    #[test]
    fn parse_csv_items_accepts_span_event_event_range() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
kingdom,span,1001,1180,,アルカディア王国,dynasty|fictional,span:arcadia\n\
incidents,event,,,1042,竜騎士団の創設,founding,event:knights\n\
incidents,event_range,1175,1180,,黒霧戦争,war|fictional,range:black_mist\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].item_type, CsvItemType::Span);
        assert_eq!(items[1].item_type, CsvItemType::Event);
        assert_eq!(items[2].item_type, CsvItemType::EventRange);
        assert_eq!(items[0].tags, vec!["dynasty", "fictional"]);
    }

    #[test]
    fn parse_csv_items_rejects_missing_required_columns() {
        let path = write_temp_csv("lane,type,start,end,time,label,tags\na,event,,,10,foo,tag\n");
        let err = parse_csv_items(&path).unwrap_err();
        std::fs::remove_file(path).ok();
        assert!(err.contains("missing required column: id"));
    }

    #[test]
    fn parse_csv_items_rejects_invalid_type_and_number() {
        let path_bad_type = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
a,unknown,1,2,,foo,,\n",
        );
        let err = parse_csv_items(&path_bad_type).unwrap_err();
        std::fs::remove_file(path_bad_type).ok();
        assert!(err.contains("invalid type"));

        let path_bad_num = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
a,event,,,abc,foo,,\n",
        );
        let err = parse_csv_items(&path_bad_num).unwrap_err();
        std::fs::remove_file(path_bad_num).ok();
        assert!(
            err.contains("time must be YYYY-MM-DD"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_csv_items_accepts_month_day_precision() {
        use tdsl_parser::ast::TimeValue;
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
ww2,span,1939-09-01,1945-09-02,,第二次世界大戦,war,span:ww2\n\
mission,event,,,1969-07-20,アポロ11号着陸,space,event:apollo\n\
months,event_range,1939-09,1945-09,,WW2期間,war,range:ww2\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].start, Some(TimeValue::Date(1939, 9, 1)));
        assert_eq!(items[0].end, Some(TimeValue::Date(1945, 9, 2)));
        assert_eq!(items[1].time, Some(TimeValue::Date(1969, 7, 20)));
        assert_eq!(items[2].start, Some(TimeValue::YearMonth(1939, 9)));
        assert_eq!(items[2].end, Some(TimeValue::YearMonth(1945, 9)));
    }

    #[test]
    fn parse_csv_items_rejects_invalid_month() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
a,event,,,2020-13-01,foo,,\n",
        );
        let err = parse_csv_items(&path).unwrap_err();
        std::fs::remove_file(path).ok();
        assert!(err.contains("CSV row 2"), "missing row no: {err}");
        assert!(
            err.contains("time") && err.contains("2020-13-01"),
            "missing field/raw: {err}"
        );
    }

    #[test]
    fn parse_csv_items_accepts_negative_year_with_month() {
        // #520: 紀元前も月日精度を保持する
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
a,event,,,-0206-01,foo,,\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();
        assert!(matches!(
            items[0].time,
            Some(tdsl_parser::ast::TimeValue::YearMonth(-206, 1))
        ));
    }

    // ─── source / origin 往復保持 (#608) ───

    #[test]
    fn parse_csv_items_accepts_legacy_8_column_csv() {
        // 旧8列CSV（source/origin ヘッダなし）は引き続き受理され、source/origin は None になる。
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
a,event,,,2020,foo,,legacy:1\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source, None);
        assert_eq!(items[0].origin, None);
    }

    #[test]
    fn parse_csv_items_accepts_source_and_origin_columns() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,1969,アポロ11号,,event:apollo,wd:Q1,wikidata\n\
a,event,,,1201,手作りの記録,,event:manual,,\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();
        assert_eq!(items.len(), 2);
        let source0 = items[0].source.as_ref().expect("source should be Some");
        assert_eq!(source0.prefix, "wd");
        assert_eq!(source0.qid, "Q1");
        assert_eq!(items[0].origin.as_deref(), Some("wikidata"));
        // 空欄は None（両列とも任意）
        assert_eq!(items[1].source, None);
        assert_eq!(items[1].origin, None);
    }

    #[test]
    fn parse_csv_items_rejects_invalid_source_format() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,2020,foo,,,badvalue,\n",
        );
        let err = parse_csv_items(&path).unwrap_err();
        std::fs::remove_file(path).ok();
        assert!(err.contains("CSV row 2"), "missing row no: {err}");
        assert!(err.contains("source"), "missing field: {err}");
    }

    #[test]
    fn parse_csv_items_rejects_invalid_origin_format() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,2020,foo,,,,123bad\n",
        );
        let err = parse_csv_items(&path).unwrap_err();
        std::fs::remove_file(path).ok();
        assert!(err.contains("CSV row 2"), "missing row no: {err}");
        assert!(err.contains("origin"), "missing field: {err}");
    }

    #[test]
    fn parse_csv_items_rejects_origin_wikidata_without_wd_source() {
        // origin=wikidata なのに source が空欄 -> エラー
        let path_missing = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,2020,foo,,,,wikidata\n",
        );
        let err = parse_csv_items(&path_missing).unwrap_err();
        std::fs::remove_file(path_missing).ok();
        assert!(
            err.contains("origin=wikidata requires"),
            "unexpected error: {err}"
        );

        // origin=wikidata なのに source が wd: 以外 -> エラー
        let path_wrong_prefix = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,2020,foo,,,other:Q1,wikidata\n",
        );
        let err = parse_csv_items(&path_wrong_prefix).unwrap_err();
        std::fs::remove_file(path_wrong_prefix).ok();
        assert!(
            err.contains("origin=wikidata requires"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn parse_csv_items_accepts_wd_source_with_non_wikidata_origin() {
        // wd:Q… source で origin が wikidata 以外（or 無）は static provenance として保持される（書き換えない）。
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,2020,foo,,,wd:Q1,manual\n\
a,event,,,2021,bar,,,wd:Q2,\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();
        assert_eq!(items[0].source.as_ref().unwrap().qid, "Q1");
        assert_eq!(items[0].origin.as_deref(), Some("manual"));
        assert_eq!(items[1].source.as_ref().unwrap().qid, "Q2");
        assert_eq!(items[1].origin, None);
    }

    #[test]
    fn parse_csv_items_rejects_duplicate_source_header() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,source\n\
a,event,,,2020,foo,,,wd:Q1,wd:Q1\n",
        );
        let err = parse_csv_items(&path).unwrap_err();
        std::fs::remove_file(path).ok();
        assert!(err.contains("duplicated"), "unexpected error: {err}");
    }

    #[test]
    fn render_imported_csv_items_emits_source_and_origin_and_round_trips() {
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id,source,origin\n\
a,event,,,1969,アポロ11号,,event:apollo,wd:Q1,wikidata\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();

        let snippet = render_imported_csv_items(&items);
        assert!(
            snippet.contains("source wd:Q1;"),
            "missing source: {snippet}"
        );
        assert!(
            snippet.contains("origin wikidata;"),
            "missing origin: {snippet}"
        );

        let file = tdsl_parser::parse(&snippet)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\n--- snippet ---\n{snippet}"));
        assert_eq!(file.statements.len(), 1);
    }

    #[test]
    fn render_imported_csv_items_emits_date_literal_round_trip() {
        // CSV → render → 再パースが成功し、TimeValue の precision が往復で一致する
        let path = write_temp_csv(
            "lane,type,start,end,time,label,tags,id\n\
ww2,span,1939-09-01,1945-09-02,,第二次世界大戦,war,span:ww2\n\
mission,event,,,1969-07-20,着陸,space,event:apollo\n\
qin,span,-221,-206,,秦,dynasty,span:qin\n",
        );
        let items = parse_csv_items(&path).unwrap();
        std::fs::remove_file(path).ok();

        let snippet = render_imported_csv_items(&items);
        // 月日リテラルが Display 経由で正しく出力される
        assert!(
            snippet.contains("1939-09-01..1945-09-02"),
            "missing date range: {snippet}"
        );
        assert!(
            snippet.contains("event mission 1969-07-20"),
            "missing date event: {snippet}"
        );
        assert!(
            snippet.contains("-221..-206"),
            "missing negative year: {snippet}"
        );

        // 再パース可能であること
        let file = tdsl_parser::parse(&snippet)
            .unwrap_or_else(|e| panic!("re-parse failed: {e}\n--- snippet ---\n{snippet}"));
        assert_eq!(file.statements.len(), 3);
    }
}
