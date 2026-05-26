use std::fmt::Write;

use tdsl_wikidata::entity::{DataValue, time_value_to_year};
use tdsl_wikidata::{WikidataClient, WikidataEntity};

use crate::{ScaffoldLaneMode, ScaffoldTargetType};

#[allow(clippy::too_many_arguments)]
pub(crate) fn cmd_scaffold_wikidata(
    qids: &str,
    timeline: &str,
    output: Option<&std::path::Path>,
    lang: &str,
    target: ScaffoldTargetType,
    lane_mode: ScaffoldLaneMode,
    single_lane_label: &str,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let qids = parse_qids(qids)?;
    let langs = super::parse_langs(lang);
    let timeline = timeline.trim();
    if timeline.is_empty() {
        return Err("timeline must not be empty".to_string());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let doc = rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let mut entities = Vec::new();
        let langs_ref: Vec<&str> = langs.iter().map(String::as_str).collect();
        for qid in &qids {
            let entity = WikidataClient::get_entity(&client, qid, &langs_ref)
                .await
                .map_err(|e| format!("{qid}: {e}"))?;
            entities.push(entity);
        }
        Ok::<String, String>(render_scaffold_tdsl(
            timeline,
            &langs,
            &entities,
            target,
            lane_mode,
            single_lane_label,
        ))
    })?;

    if let Some(path) = output {
        std::fs::write(path, &doc)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        eprintln!("Written scaffold to {}", path.display());
    } else {
        println!("{doc}");
    }

    Ok(())
}

fn parse_qids(input: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for part in input.split(',') {
        let qid = part.trim().to_ascii_uppercase();
        if qid.is_empty() {
            continue;
        }
        let valid =
            qid.starts_with('Q') && qid.len() > 1 && qid[1..].chars().all(|c| c.is_ascii_digit());
        if !valid {
            return Err(format!("invalid QID: {qid}"));
        }
        if !out.iter().any(|x| x == &qid) {
            out.push(qid);
        }
    }
    if out.is_empty() {
        return Err("qids must include at least one QID".to_string());
    }
    Ok(out)
}

fn render_scaffold_tdsl(
    timeline_title: &str,
    langs: &[String],
    entities: &[WikidataEntity],
    target: ScaffoldTargetType,
    lane_mode: ScaffoldLaneMode,
    single_lane_label: &str,
) -> String {
    let label_expr = build_label_expr(langs);
    let mut rows = Vec::new();
    let mut alias_seen = std::collections::HashSet::new();
    let mut lane_alias_seen = std::collections::HashSet::new();

    for entity in entities {
        let label = entity_label(entity, langs);
        let import_alias = super::make_unique_alias(
            &format!("q{}", entity.id[1..].to_ascii_lowercase()),
            &mut alias_seen,
        );
        let lane_alias = match lane_mode {
            ScaffoldLaneMode::Single => "main".to_string(),
            ScaffoldLaneMode::ByKind => {
                if is_person_entity(entity) {
                    "persons".to_string()
                } else {
                    "entities".to_string()
                }
            }
            ScaffoldLaneMode::PerEntity => {
                let base = super::slug_ascii(&label);
                let fallback = entity.id.to_ascii_lowercase();
                let seed = if base.is_empty() { fallback } else { base };
                super::make_unique_alias(&seed, &mut lane_alias_seen)
            }
        };
        rows.push(ScaffoldRow {
            qid: entity.id.clone(),
            label,
            import_alias,
            lane_alias,
            map_plan: choose_map_plan(entity, target),
            entity: entity.clone(),
        });
    }

    let lanes = collect_lanes(&rows, lane_mode, single_lane_label);
    let (range_start, range_end) = estimate_range(&rows);
    let escaped_timeline = super::escape_tdsl_string(timeline_title);

    let mut s = String::new();
    writeln!(
        s,
        r#"timeline "{title}" {{
    title "{title}";
    unit year;
    range {start}..{end};
    calendar proleptic_gregorian;
}}"#,
        title = escaped_timeline,
        start = range_start,
        end = range_end
    )
    .unwrap();
    s.push('\n');

    for lane in &lanes {
        writeln!(
            s,
            r#"lane "{label}" as {alias} {{ kind {kind}; order {order}; }}"#,
            label = super::escape_tdsl_string(&lane.label),
            alias = lane.alias,
            kind = lane.kind,
            order = lane.order
        )
        .unwrap();
    }
    s.push('\n');

    s.push_str("import wikidata as wd {\n");
    for row in &rows {
        writeln!(s, "    entity {} as {};", row.qid, row.import_alias).unwrap();
    }
    s.push_str("    policy merge_by_source;\n");
    s.push_str("}\n\n");

    for row in &rows {
        writeln!(
            s,
            r#"map wd.{import_alias} to {target} {{
    lane {lane_alias};"#,
            import_alias = row.import_alias,
            target = row.map_plan.target,
            lane_alias = row.lane_alias
        )
        .unwrap();

        if let (Some(start), Some(end)) = (row.map_plan.start, row.map_plan.end) {
            writeln!(s, "    start {start};").unwrap();
            writeln!(s, "    end {end};").unwrap();
        }
        if let Some(time) = row.map_plan.time {
            writeln!(s, "    time {time};").unwrap();
        }

        writeln!(s, "    label {label_expr};").unwrap();
        s.push_str("    tags [\"imported\"];\n");
        writeln!(s, "}} // {}", row.map_plan.reason).unwrap();
        s.push('\n');
    }

    s
}

#[derive(Clone)]
struct ScaffoldRow {
    qid: String,
    label: String,
    import_alias: String,
    lane_alias: String,
    map_plan: MapPlan,
    entity: WikidataEntity,
}

#[derive(Clone)]
struct LaneDef {
    alias: String,
    label: String,
    kind: String,
    order: i64,
}

#[derive(Clone, Copy)]
struct MapPlan {
    target: &'static str,
    start: Option<&'static str>,
    end: Option<&'static str>,
    time: Option<&'static str>,
    reason: &'static str,
}

fn collect_lanes(
    rows: &[ScaffoldRow],
    lane_mode: ScaffoldLaneMode,
    single_lane_label: &str,
) -> Vec<LaneDef> {
    match lane_mode {
        ScaffoldLaneMode::Single => vec![LaneDef {
            alias: "main".to_string(),
            label: single_lane_label.trim().to_string(),
            kind: "custom".to_string(),
            order: 10,
        }],
        ScaffoldLaneMode::ByKind => vec![
            LaneDef {
                alias: "persons".to_string(),
                label: "人物".to_string(),
                kind: "person".to_string(),
                order: 10,
            },
            LaneDef {
                alias: "entities".to_string(),
                label: "組織・王朝".to_string(),
                kind: "entity".to_string(),
                order: 20,
            },
        ],
        ScaffoldLaneMode::PerEntity => rows
            .iter()
            .enumerate()
            .map(|(i, row)| LaneDef {
                alias: row.lane_alias.clone(),
                label: row.label.clone(),
                kind: if is_person_entity(&row.entity) {
                    "person".to_string()
                } else {
                    "entity".to_string()
                },
                order: ((i as i64) + 1) * 10,
            })
            .collect(),
    }
}

fn choose_map_plan(entity: &WikidataEntity, target: ScaffoldTargetType) -> MapPlan {
    let has = |pid: &str| entity.claim(pid).is_some();

    let span_from_inception = MapPlan {
        target: "span",
        start: Some("claim(P571).year"),
        end: Some("claim(P576).year"),
        time: None,
        reason: "inception/dissolved を利用",
    };
    let span_from_life = MapPlan {
        target: "span",
        start: Some("claim(P569).year"),
        end: Some("claim(P570).year"),
        time: None,
        reason: "date of birth/date of death を利用",
    };
    let range_from_start_end = MapPlan {
        target: "event_range",
        start: Some("claim(P580).year"),
        end: Some("claim(P582).year"),
        time: None,
        reason: "start time/end time を利用",
    };
    let event_from_point = MapPlan {
        target: "event",
        start: None,
        end: None,
        time: Some("claim(P585).year"),
        reason: "point in time を利用",
    };
    let fallback_event = MapPlan {
        target: "event",
        start: None,
        end: None,
        time: Some("claim(P571).year"),
        reason: "候補不足のため inception を暫定使用（要確認）",
    };

    match target {
        ScaffoldTargetType::Span => {
            if has("P571") && has("P576") {
                span_from_inception
            } else if has("P569") && has("P570") {
                span_from_life
            } else if has("P580") && has("P582") {
                range_from_start_end
            } else {
                fallback_event
            }
        }
        ScaffoldTargetType::EventRange => {
            if has("P580") && has("P582") {
                range_from_start_end
            } else if has("P571") && has("P576") {
                span_from_inception
            } else if has("P569") && has("P570") {
                span_from_life
            } else {
                fallback_event
            }
        }
        ScaffoldTargetType::Event => {
            if has("P585") {
                event_from_point
            } else if has("P571") {
                fallback_event
            } else if has("P580") {
                MapPlan {
                    target: "event",
                    start: None,
                    end: None,
                    time: Some("claim(P580).year"),
                    reason: "start time を利用",
                }
            } else if has("P569") {
                MapPlan {
                    target: "event",
                    start: None,
                    end: None,
                    time: Some("claim(P569).year"),
                    reason: "date of birth を利用",
                }
            } else {
                fallback_event
            }
        }
        ScaffoldTargetType::Auto => {
            if has("P571") && has("P576") {
                span_from_inception
            } else if has("P569") && has("P570") {
                span_from_life
            } else if has("P580") && has("P582") {
                range_from_start_end
            } else if has("P585") {
                event_from_point
            } else {
                fallback_event
            }
        }
    }
}

fn build_label_expr(langs: &[String]) -> String {
    let expr = langs
        .iter()
        .map(|lang| format!("label@{lang}"))
        .collect::<Vec<_>>()
        .join(" ?? ");
    if expr.is_empty() {
        "label@en".to_string()
    } else {
        expr
    }
}

fn entity_label(entity: &WikidataEntity, langs: &[String]) -> String {
    for lang in langs {
        if let Some(v) = entity.labels.get(lang) {
            return v.value.clone();
        }
    }
    if let Some(v) = entity.labels.values().next() {
        return v.value.clone();
    }
    entity.id.clone()
}

fn estimate_range(rows: &[ScaffoldRow]) -> (i64, i64) {
    let mut years = Vec::new();
    for row in rows {
        for pid in ["P569", "P570", "P571", "P576", "P580", "P582", "P585"] {
            if let Some(year) = claim_year(&row.entity, pid) {
                years.push(year);
            }
        }
    }
    if years.is_empty() {
        return (0, 2000);
    }
    let min = years.iter().min().copied().unwrap();
    let max = years.iter().max().copied().unwrap();
    (min - 20, max + 20)
}

fn claim_year(entity: &WikidataEntity, pid: &str) -> Option<i64> {
    match entity.claim(pid)? {
        DataValue::Time { value } => time_value_to_year(value).ok(),
        _ => None,
    }
}

fn is_person_entity(entity: &WikidataEntity) -> bool {
    entity.claim("P569").is_some() || entity.claim("P570").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_qids_dedup_and_validate() {
        let qids = parse_qids("q7209, Q7183, q7209").unwrap();
        assert_eq!(qids, vec!["Q7209", "Q7183"]);
        assert!(parse_qids("X123").is_err());
    }

    #[test]
    fn parse_qids_empty_returns_error() {
        assert!(parse_qids("").is_err());
    }

    #[test]
    fn parse_qids_normalizes_lowercase_to_uppercase() {
        let qids = parse_qids("q1, q2").unwrap();
        assert_eq!(qids, vec!["Q1", "Q2"]);
    }

    #[test]
    fn parse_qids_rejects_non_q_prefix() {
        assert!(parse_qids("P569").is_err());
        assert!(parse_qids("123").is_err());
    }

    #[test]
    fn render_scaffold_contains_import_and_maps() {
        let mut labels = std::collections::HashMap::new();
        labels.insert(
            "ja".to_string(),
            tdsl_wikidata::entity::LabelValue {
                language: "ja".to_string(),
                value: "漢".to_string(),
            },
        );
        let mut claims = std::collections::HashMap::new();
        claims.insert(
            "P571".to_string(),
            vec![tdsl_wikidata::entity::Statement {
                mainsnak: tdsl_wikidata::entity::Snak {
                    snaktype: "value".to_string(),
                    property: "P571".to_string(),
                    datavalue: Some(DataValue::Time {
                        value: tdsl_wikidata::entity::TimeValue {
                            time: "-0206-01-01T00:00:00Z".to_string(),
                            precision: 9,
                            calendarmodel: String::new(),
                        },
                    }),
                },
                rank: "normal".to_string(),
                qualifiers: std::collections::HashMap::new(),
            }],
        );
        claims.insert(
            "P576".to_string(),
            vec![tdsl_wikidata::entity::Statement {
                mainsnak: tdsl_wikidata::entity::Snak {
                    snaktype: "value".to_string(),
                    property: "P576".to_string(),
                    datavalue: Some(DataValue::Time {
                        value: tdsl_wikidata::entity::TimeValue {
                            time: "+0220-01-01T00:00:00Z".to_string(),
                            precision: 9,
                            calendarmodel: String::new(),
                        },
                    }),
                },
                rank: "normal".to_string(),
                qualifiers: std::collections::HashMap::new(),
            }],
        );

        let entity = WikidataEntity {
            id: "Q7209".to_string(),
            labels,
            claims,
        };
        let doc = render_scaffold_tdsl(
            "中国王朝",
            &["ja".to_string(), "en".to_string()],
            &[entity],
            ScaffoldTargetType::Auto,
            ScaffoldLaneMode::PerEntity,
            "項目",
        );
        assert!(doc.contains("import wikidata as wd"));
        assert!(doc.contains("entity Q7209 as q7209;"));
        assert!(doc.contains("map wd.q7209 to span"));
        assert!(doc.contains("label label@ja ?? label@en;"));
    }
}
