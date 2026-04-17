use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use serde::Serialize;
use tdsl_wikidata::entity::{DataValue, time_value_to_year};
use tdsl_wikidata::{WikidataClient, WikidataEntity};

#[derive(Parser)]
#[command(name = "tdsl", version, about = "Timeline DSL compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .tdsl file to IR JSON
    Build {
        /// Input .tdsl file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output JSON file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty-print JSON output
        #[arg(long, default_value_t = false)]
        pretty: bool,

        /// Skip Wikidata fetching (only process static items)
        #[arg(long, default_value_t = false)]
        offline: bool,
    },

    /// Check a .tdsl file for syntax and semantic errors
    Check {
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },

    /// Dump the parsed AST (for debugging)
    Ast {
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },

    /// Fetch a Wikidata entity and display its data
    Fetch {
        /// Wikidata QID (e.g., Q7209)
        qid: String,

        /// Languages to fetch labels for
        #[arg(short, long, default_value = "ja,en")]
        lang: String,
    },

    /// Search Wikidata entities by keyword and list candidate QIDs
    Search {
        /// Free-text query (e.g., "漢王朝")
        query: String,

        /// Language used by Wikidata search
        #[arg(short, long, default_value = "ja")]
        lang: String,

        /// Max number of results (1..50)
        #[arg(short = 'n', long, default_value_t = 10)]
        limit: usize,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Inspect one Wikidata entity and suggest timeline mapping strategy
    Inspect {
        /// Wikidata QID (e.g., Q7209)
        qid: String,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Render a .tdsl file to a standalone HTML timeline
    Render {
        /// Input .tdsl file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output HTML file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pixels per year on the horizontal axis
        #[arg(long, default_value_t = 2.0)]
        scale: f64,

        /// Skip Wikidata fetching (only process static items)
        #[arg(long, default_value_t = false)]
        offline: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Build {
            input,
            output,
            pretty,
            offline,
        } => cmd_build(&input, output.as_deref(), pretty, offline),
        Commands::Check { input } => cmd_check(&input),
        Commands::Ast { input } => cmd_ast(&input),
        Commands::Fetch { qid, lang } => cmd_fetch(&qid, &lang),
        Commands::Search {
            query,
            lang,
            limit,
            json,
        } => cmd_search(&query, &lang, limit, json),
        Commands::Inspect { qid, lang, json } => cmd_inspect(&qid, &lang, json),
        Commands::Render {
            input,
            output,
            scale,
            offline,
        } => cmd_render(&input, output.as_deref(), scale, offline),
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn read_source(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

/// Parse and lower a .tdsl file into an IR. Shared by `build` and `render`.
fn load_ir(
    input: &std::path::Path,
    offline: bool,
) -> Result<tdsl_core::ir::TimelineIr, String> {
    let source = read_source(input)?;
    let file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;

    let ir = if offline {
        tdsl_core::lower::lower_static(&file)
    } else {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
        rt.block_on(tdsl_core::lower::lower_with_wikidata(&file, &client))
    };

    let ir = ir.map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let warnings = tdsl_core::validate::validate(&ir);
    for w in &warnings {
        eprintln!("Warning: {w}");
    }

    Ok(ir)
}

fn cmd_build(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    pretty: bool,
    offline: bool,
) -> Result<(), String> {
    let ir = load_ir(input, offline)?;

    let json = if pretty {
        serde_json::to_string_pretty(&ir).unwrap()
    } else {
        serde_json::to_string(&ir).unwrap()
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &json)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

fn cmd_check(input: &std::path::Path) -> Result<(), String> {
    let source = read_source(input)?;
    let file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;
    let ir = tdsl_core::lower::lower_static(&file).map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let warnings = tdsl_core::validate::validate(&ir);
    for w in &warnings {
        eprintln!("Warning: {w}");
    }

    eprintln!("OK: {} lanes, {} items", ir.lanes.len(), ir.items.len());
    Ok(())
}

fn cmd_ast(input: &std::path::Path) -> Result<(), String> {
    let source = read_source(input)?;
    let file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;
    println!("{file:#?}");
    Ok(())
}

fn cmd_fetch(qid: &str, lang: &str) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
        let langs_owned = parse_langs(lang);
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

fn cmd_search(query: &str, lang: &str, limit: usize, json: bool) -> Result<(), String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("search query must not be empty".to_string());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
        let hits = WikidataClient::search_entities(&client, query, lang.trim(), limit)
            .await
            .map_err(|e| e.to_string())?;

        if json {
            println!("{}", serde_json::to_string_pretty(&hits).unwrap());
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

fn cmd_inspect(qid: &str, lang: &str, json: bool) -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
        let langs_owned = parse_langs(lang);
        let langs: Vec<&str> = langs_owned.iter().map(String::as_str).collect();
        let entity = WikidataClient::get_entity(&client, qid, &langs)
            .await
            .map_err(|e| e.to_string())?;

        let report = build_inspect_report(&entity, &langs_owned);
        if json {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            return Ok(());
        }

        print_inspect_report(&report);
        Ok(())
    })
}

fn cmd_render(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    scale: f64,
    offline: bool,
) -> Result<(), String> {
    let ir = load_ir(input, offline)?;

    let opts = tdsl_render::RenderOptions {
        scale,
        ..Default::default()
    };
    let html = tdsl_render::render_html(&ir, opts);

    if let Some(out_path) = output {
        std::fs::write(out_path, &html)
            .map_err(|e| format!("Failed to write {}: {e}", out_path.display()))?;
        eprintln!("Written to {}", out_path.display());
    } else {
        println!("{html}");
    }

    Ok(())
}

fn parse_langs(lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in lang.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_ascii_lowercase();
        if !out.iter().any(|x| x == &lowered) {
            out.push(lowered);
        }
    }
    if out.is_empty() {
        out.push("en".to_string());
    }
    out
}

#[derive(Debug, Serialize)]
struct InspectReport {
    entity_id: String,
    labels: Vec<InspectLabel>,
    claims: Vec<InspectClaim>,
    suggestions: Vec<MapSuggestion>,
}

#[derive(Debug, Serialize)]
struct InspectLabel {
    lang: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct InspectClaim {
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

fn build_inspect_report(entity: &WikidataEntity, langs: &[String]) -> InspectReport {
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
        DataValue::Time { value } => (
            time_value_to_year(value).ok(),
            value.time.clone(),
        ),
        DataValue::String { value } => (None, value.clone()),
        DataValue::MonolingualText { value } => (None, format!("{}@{}", value.text, value.language)),
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

fn print_inspect_report(report: &InspectReport) {
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

    #[test]
    fn parse_langs_dedup_and_trim() {
        let langs = parse_langs(" ja, en,ja, ,ZH ");
        assert_eq!(langs, vec!["ja", "en", "zh"]);
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
        assert!(suggestions
            .iter()
            .any(|s| s.start.as_deref() == Some("claim(P571).year")));
    }
}
