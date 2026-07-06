use serde::Serialize;
use tdsl_wikidata::entity::{DataValue, time_value_to_year};
use tdsl_wikidata::{WikidataClient, WikidataEntity, parse_wikipedia_url};

/// Wikidata エンティティを取得して表示する。
pub(crate) fn cmd_fetch(
    qid: &str,
    lang: &str,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let langs_owned = super::parse_langs(lang);
        let langs: Vec<&str> = langs_owned.iter().map(String::as_str).collect();
        let entity = WikidataClient::get_entity(&client, qid, &langs)
            .await
            .map_err(|e| e.to_string())?;

        println!("Entity: {}", entity.id);
        for (lang_code, lv) in &entity.labels {
            println!("  label@{lang_code}: {}", lv.value);
        }

        let props = [
            ("P569", "date of birth"),
            ("P570", "date of death"),
            ("P571", "inception"),
            ("P576", "dissolved"),
            ("P580", "start time"),
            ("P582", "end time"),
        ];
        println!("Claims:");
        for (pid, desc) in &props {
            if let Some(dv) = entity.claim(pid) {
                match dv {
                    tdsl_wikidata::entity::DataValue::Time { value } => {
                        match tdsl_wikidata::entity::time_value_to_year(value) {
                            Ok(year) => println!("  {pid} ({desc}): {year}"),
                            Err(_) => println!("  {pid} ({desc}): {}", value.time),
                        }
                    }
                    other => println!("  {pid} ({desc}): {other:?}"),
                }
            }
        }

        let total: usize = entity.claims.values().map(|v| v.len()).sum();
        println!(
            "  ({total} total statements across {} properties)",
            entity.claims.len()
        );

        Ok(())
    })
}

/// Wikidata エンティティをキーワード検索する。
pub(crate) fn cmd_search(
    query: &str,
    lang: &str,
    limit: usize,
    json: bool,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("search query must not be empty".to_string());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let hits = WikidataClient::search_entities(&client, query, lang.trim(), limit)
            .await
            .map_err(|e| e.to_string())?;

        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&hits).map_err(|e| e.to_string())?
            );
            return Ok(());
        }

        if hits.is_empty() {
            println!("No Wikidata items found for query: {query}");
            return Ok(());
        }

        println!("Found {} Wikidata item(s):", hits.len());
        for hit in &hits {
            let label = if hit.label.trim().is_empty() {
                "(no label)"
            } else {
                hit.label.as_str()
            };
            let desc = hit.description.as_deref().unwrap_or("(no description)");
            println!("- {}  {}  {}", hit.id, label, desc);
            if !hit.aliases.is_empty() {
                println!("  aliases: {}", hit.aliases.join(", "));
            }
        }

        Ok(())
    })
}

/// Wikidata エンティティを詳細分析してマッピング戦略を提示する。
pub(crate) fn cmd_inspect(
    qid: &str,
    lang: &str,
    json: bool,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let langs_owned = super::parse_langs(lang);
        let langs: Vec<&str> = langs_owned.iter().map(String::as_str).collect();
        let entity = WikidataClient::get_entity(&client, qid, &langs)
            .await
            .map_err(|e| e.to_string())?;

        let report = build_inspect_report(&entity, &langs_owned);
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
            return Ok(());
        }

        print_inspect_report(&report);
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub(crate) struct ResolveReport {
    qid: String,
    site: String,
    title: String,
    labels: Vec<InspectLabel>,
}

/// Wikipedia URL を Wikidata QID に解決する。
pub(crate) fn cmd_resolve(
    url: &str,
    lang: &str,
    json: bool,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    let page = parse_wikipedia_url(url).map_err(|e| e.to_string())?;
    let langs_owned = super::parse_langs(lang);
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let report = rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let langs: Vec<&str> = langs_owned.iter().map(String::as_str).collect();
        let entity =
            WikidataClient::get_entity_by_sitelink(&client, &page.site, &page.title, &langs)
                .await
                .map_err(|e| e.to_string())?;

        let mut labels = Vec::new();
        for lang in &langs_owned {
            if let Some(lv) = entity.labels.get(lang) {
                labels.push(InspectLabel {
                    lang: lang.clone(),
                    value: lv.value.clone(),
                });
            }
        }
        if labels.is_empty() {
            for (lang, lv) in &entity.labels {
                labels.push(InspectLabel {
                    lang: lang.clone(),
                    value: lv.value.clone(),
                });
                if labels.len() >= 3 {
                    break;
                }
            }
        }

        Ok::<ResolveReport, String>(ResolveReport {
            qid: entity.id,
            site: page.site.clone(),
            title: page.title.clone(),
            labels,
        })
    })?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
        return Ok(());
    }

    println!("Resolved QID: {}", report.qid);
    println!("  site: {}", report.site);
    println!("  title: {}", report.title);
    if report.labels.is_empty() {
        println!("  labels: (none)");
    } else {
        for label in &report.labels {
            println!("  label@{}: {}", label.lang, label.value);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct InspectReport {
    entity_id: String,
    labels: Vec<InspectLabel>,
    claims: Vec<InspectClaim>,
    suggestions: Vec<MapSuggestion>,
}

#[derive(Debug, Serialize)]
pub(crate) struct InspectLabel {
    pub(crate) lang: String,
    pub(crate) value: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct InspectClaim {
    property: String,
    description: String,
    year: Option<i64>,
    raw: String,
}

#[derive(Debug, Serialize)]
struct MapSuggestion {
    target: String,
    reason: String,
    start: Option<String>,
    end: Option<String>,
    time: Option<String>,
    label_expr: String,
}

pub(crate) fn build_inspect_report(entity: &WikidataEntity, langs: &[String]) -> InspectReport {
    const TIMELINE_PROPS: [(&str, &str); 7] = [
        ("P569", "date of birth"),
        ("P570", "date of death"),
        ("P571", "inception"),
        ("P576", "dissolved"),
        ("P580", "start time"),
        ("P582", "end time"),
        ("P585", "point in time"),
    ];

    let mut labels = Vec::new();
    for lang in langs {
        if let Some(lv) = entity.labels.get(lang) {
            labels.push(InspectLabel {
                lang: lang.clone(),
                value: lv.value.clone(),
            });
        }
    }
    if labels.is_empty() {
        for (lang, lv) in &entity.labels {
            labels.push(InspectLabel {
                lang: lang.clone(),
                value: lv.value.clone(),
            });
            if labels.len() >= 3 {
                break;
            }
        }
    }

    let mut claims = Vec::new();
    for (pid, desc) in TIMELINE_PROPS {
        if let Some(dv) = entity.claim(pid) {
            let (year, raw) = summarize_claim_value(dv);
            claims.push(InspectClaim {
                property: pid.to_string(),
                description: desc.to_string(),
                year,
                raw,
            });
        }
    }

    let suggestions = suggest_map_targets(&claims, langs);

    InspectReport {
        entity_id: entity.id.clone(),
        labels,
        claims,
        suggestions,
    }
}

fn summarize_claim_value(dv: &DataValue) -> (Option<i64>, String) {
    match dv {
        DataValue::Time { value } => (time_value_to_year(value).ok(), value.time.clone()),
        DataValue::String { value } => (None, value.clone()),
        DataValue::MonolingualText { value } => {
            (None, format!("{}@{}", value.text, value.language))
        }
        DataValue::WikibaseEntityId { value } => (None, value.id.clone()),
        DataValue::Quantity { value } => (None, value.to_string()),
        DataValue::GlobeCoordinate { value } => (None, value.to_string()),
    }
}

fn suggest_map_targets(claims: &[InspectClaim], langs: &[String]) -> Vec<MapSuggestion> {
    let has = |pid: &str| claims.iter().any(|c| c.property == pid);
    let label_expr = langs
        .iter()
        .map(|lang| format!("label@{lang}"))
        .collect::<Vec<_>>()
        .join(" ?? ");
    let label_expr = if label_expr.is_empty() {
        "label@en".to_string()
    } else {
        label_expr
    };

    let mut out = Vec::new();

    if has("P571") && has("P576") {
        out.push(MapSuggestion {
            target: "span".to_string(),
            reason: "inception と dissolved があるため".to_string(),
            start: Some("claim(P571).year".to_string()),
            end: Some("claim(P576).year".to_string()),
            time: None,
            label_expr: label_expr.clone(),
        });
    }

    if has("P569") && has("P570") {
        out.push(MapSuggestion {
            target: "span".to_string(),
            reason: "date of birth と date of death があるため".to_string(),
            start: Some("claim(P569).year".to_string()),
            end: Some("claim(P570).year".to_string()),
            time: None,
            label_expr: label_expr.clone(),
        });
    }

    if has("P580") && has("P582") {
        out.push(MapSuggestion {
            target: "event_range".to_string(),
            reason: "start time と end time があるため".to_string(),
            start: Some("claim(P580).year".to_string()),
            end: Some("claim(P582).year".to_string()),
            time: None,
            label_expr: label_expr.clone(),
        });
    }

    if has("P585") {
        out.push(MapSuggestion {
            target: "event".to_string(),
            reason: "point in time があるため".to_string(),
            start: None,
            end: None,
            time: Some("claim(P585).year".to_string()),
            label_expr: label_expr.clone(),
        });
    }

    if out.is_empty() {
        if has("P571") {
            out.push(MapSuggestion {
                target: "event".to_string(),
                reason: "inception のみ確認できたため".to_string(),
                start: None,
                end: None,
                time: Some("claim(P571).year".to_string()),
                label_expr: label_expr.clone(),
            });
        } else if has("P580") {
            out.push(MapSuggestion {
                target: "event".to_string(),
                reason: "start time のみ確認できたため".to_string(),
                start: None,
                end: None,
                time: Some("claim(P580).year".to_string()),
                label_expr: label_expr.clone(),
            });
        }
    }

    out
}

pub(crate) fn print_inspect_report(report: &InspectReport) {
    println!("Entity: {}", report.entity_id);
    if report.labels.is_empty() {
        println!("Labels: (none in requested languages)");
    } else {
        println!("Labels:");
        for label in &report.labels {
            println!("  {}: {}", label.lang, label.value);
        }
    }

    if report.claims.is_empty() {
        println!("Timeline-relevant claims: (none)");
    } else {
        println!("Timeline-relevant claims:");
        for claim in &report.claims {
            match claim.year {
                Some(year) => println!(
                    "  {} ({}) = {} (raw: {})",
                    claim.property, claim.description, year, claim.raw
                ),
                None => println!(
                    "  {} ({}) = {}",
                    claim.property, claim.description, claim.raw
                ),
            }
        }
    }

    if report.suggestions.is_empty() {
        println!("Suggested map targets: (none)");
        return;
    }
    println!("Suggested map targets:");
    for s in &report.suggestions {
        println!("- {}: {}", s.target, s.reason);
        if let (Some(start), Some(end)) = (&s.start, &s.end) {
            println!("  start: {start}");
            println!("  end:   {end}");
        }
        if let Some(time) = &s.time {
            println!("  time:  {time}");
        }
        println!("  label: {}", s.label_expr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cmd_search` validates the query before spinning up the Tokio runtime /
    /// touching the network, so empty/whitespace-only queries can be asserted
    /// offline and deterministically.
    #[test]
    fn cmd_search_rejects_empty_query() {
        let err = cmd_search("", "ja", 10, false, std::time::Duration::from_secs(30))
            .expect_err("empty query must error");
        assert!(
            err.contains("must not be empty"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn cmd_search_rejects_whitespace_only_query() {
        let err = cmd_search("   ", "ja", 10, false, std::time::Duration::from_secs(30))
            .expect_err("whitespace-only query must error");
        assert!(
            err.contains("must not be empty"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn suggest_span_from_inception_and_dissolved() {
        let claims = vec![
            InspectClaim {
                property: "P571".to_string(),
                description: "inception".to_string(),
                year: Some(-206),
                raw: "+0000-00-00T00:00:00Z".to_string(),
            },
            InspectClaim {
                property: "P576".to_string(),
                description: "dissolved".to_string(),
                year: Some(220),
                raw: "+0220-00-00T00:00:00Z".to_string(),
            },
        ];
        let suggestions = suggest_map_targets(&claims, &["ja".to_string(), "en".to_string()]);
        assert!(suggestions.iter().any(|s| s.target == "span"));
        assert!(
            suggestions
                .iter()
                .any(|s| s.start.as_deref() == Some("claim(P571).year"))
        );
    }
}
