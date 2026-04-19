use std::fmt::Write;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tdsl_wikidata::entity::{DataValue, time_value_to_year};
use tdsl_wikidata::{WikidataClient, WikidataEntity, parse_wikipedia_url};

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

    /// Resolve a Wikipedia article URL to a Wikidata QID
    Resolve {
        /// Wikipedia article URL
        url: String,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Output as JSON
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Generate a .tdsl template from Wikidata entities
    Scaffold {
        #[command(subcommand)]
        target: ScaffoldTarget,
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

    /// Generate a minimal .tdsl template for manual authoring
    Init {
        /// Output .tdsl file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Timeline display title
        #[arg(long, default_value = "新しい年表")]
        timeline: String,

        /// Range start year
        #[arg(long, default_value_t = 0)]
        range_start: i64,

        /// Range end year
        #[arg(long, default_value_t = 2000)]
        range_end: i64,

        /// Comma-separated lane labels (e.g. "王朝,事件,人物")
        #[arg(long, default_value = "")]
        lanes: String,
    },

    /// Import timeline items from CSV (`lane,type,start,end,time,label,tags,id`)
    ImportCsv {
        /// Input CSV file path (UTF-8 with header row)
        #[arg(value_name = "CSV")]
        input: PathBuf,

        /// Output .tdsl snippet path (default: stdout)
        #[arg(short, long, conflicts_with = "append")]
        output: Option<PathBuf>,

        /// Append generated items to an existing .tdsl file
        #[arg(long, conflicts_with = "output")]
        append: Option<PathBuf>,
    },

    /// Lint a .tdsl file and optionally apply safe fixes
    Lint {
        /// Input .tdsl file path
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Apply safe fixes in-place
        #[arg(long, default_value_t = false)]
        fix: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = LintOutputFormat::Text)]
        format: LintOutputFormat,
    },
}

#[derive(Subcommand)]
enum ScaffoldTarget {
    /// Scaffold a timeline from a list of Wikidata QIDs
    Wikidata {
        /// Comma-separated QIDs (e.g., Q7183,Q7209)
        #[arg(long)]
        qids: String,

        /// Timeline display title
        #[arg(long)]
        timeline: String,

        /// Output .tdsl file path (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Label fallback languages (comma-separated)
        #[arg(short, long, default_value = "ja,en")]
        lang: String,

        /// Mapping target strategy
        #[arg(long, value_enum, default_value_t = ScaffoldTargetType::Auto)]
        target: ScaffoldTargetType,

        /// Lane assignment strategy
        #[arg(long, value_enum, default_value_t = ScaffoldLaneMode::PerEntity)]
        lane_mode: ScaffoldLaneMode,

        /// Label of the shared lane used when lane-mode=single
        #[arg(long, default_value = "項目")]
        single_lane_label: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum ScaffoldTargetType {
    #[default]
    Auto,
    Span,
    Event,
    EventRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum ScaffoldLaneMode {
    Single,
    #[default]
    PerEntity,
    ByKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum LintOutputFormat {
    Text,
    Json,
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
        Commands::Resolve { url, lang, json } => cmd_resolve(&url, &lang, json),
        Commands::Scaffold { target } => match target {
            ScaffoldTarget::Wikidata {
                qids,
                timeline,
                output,
                lang,
                target,
                lane_mode,
                single_lane_label,
            } => cmd_scaffold_wikidata(
                &qids,
                &timeline,
                output.as_deref(),
                &lang,
                target,
                lane_mode,
                &single_lane_label,
            ),
        },
        Commands::Render {
            input,
            output,
            scale,
            offline,
        } => cmd_render(&input, output.as_deref(), scale, offline),
        Commands::Init {
            output,
            timeline,
            range_start,
            range_end,
            lanes,
        } => cmd_init(output.as_deref(), &timeline, range_start, range_end, &lanes),
        Commands::ImportCsv {
            input,
            output,
            append,
        } => cmd_import_csv(&input, output.as_deref(), append.as_deref()),
        Commands::Lint { input, fix, format } => cmd_lint(&input, fix, format),
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
fn load_ir(input: &std::path::Path, offline: bool) -> Result<tdsl_core::ir::TimelineIr, String> {
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

#[derive(Debug, Serialize)]
struct ResolveReport {
    qid: String,
    site: String,
    title: String,
    labels: Vec<InspectLabel>,
}

fn cmd_resolve(url: &str, lang: &str, json: bool) -> Result<(), String> {
    let page = parse_wikipedia_url(url).map_err(|e| e.to_string())?;
    let langs_owned = parse_langs(lang);
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let report = rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
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
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
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

fn cmd_scaffold_wikidata(
    qids: &str,
    timeline: &str,
    output: Option<&std::path::Path>,
    lang: &str,
    target: ScaffoldTargetType,
    lane_mode: ScaffoldLaneMode,
    single_lane_label: &str,
) -> Result<(), String> {
    let qids = parse_qids(qids)?;
    let langs = parse_langs(lang);
    let timeline = timeline.trim();
    if timeline.is_empty() {
        return Err("timeline must not be empty".to_string());
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    let doc = rt.block_on(async {
        let client = tdsl_wikidata::client::HttpWikidataClient::new();
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
        let import_alias = make_unique_alias(
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
                let base = slug_ascii(&label);
                let fallback = entity.id.to_ascii_lowercase();
                let seed = if base.is_empty() { fallback } else { base };
                make_unique_alias(&seed, &mut lane_alias_seen)
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
    let escaped_timeline = escape_tdsl_string(timeline_title);

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
            label = escape_tdsl_string(&lane.label),
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
    if min == max {
        (min - 20, max + 20)
    } else {
        (min - 20, max + 20)
    }
}

fn claim_year(entity: &WikidataEntity, pid: &str) -> Option<i64> {
    match entity.claim(pid)? {
        DataValue::Time { value } => time_value_to_year(value).ok(),
        _ => None,
    }
}

fn make_unique_alias(seed: &str, seen: &mut std::collections::HashSet<String>) -> String {
    let mut alias = if seed.is_empty() {
        "item".to_string()
    } else {
        seed.to_string()
    };
    if !alias
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false)
    {
        alias = format!("_{}", alias);
    }

    if seen.insert(alias.clone()) {
        return alias;
    }
    let mut i = 2usize;
    loop {
        let cand = format!("{alias}_{i}");
        if seen.insert(cand.clone()) {
            return cand;
        }
        i += 1;
    }
}

fn slug_ascii(s: &str) -> String {
    s.chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '-' || c == '_' {
                Some('_')
            } else {
                None
            }
        })
        .collect()
}

fn is_person_entity(entity: &WikidataEntity) -> bool {
    entity.claim("P569").is_some() || entity.claim("P570").is_some()
}

fn escape_tdsl_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
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

fn cmd_init(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CsvItemType {
    Span,
    Event,
    EventRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImportedCsvItem {
    lane: String,
    item_type: CsvItemType,
    start: Option<i64>,
    end: Option<i64>,
    time: Option<i64>,
    label: String,
    tags: Vec<String>,
    id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InitLaneSpec {
    label: String,
    alias: Option<String>,
}

fn cmd_import_csv(
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
            Some(a) if a.is_empty() => return Err("lane alias must not be empty".to_string()),
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
    let mut out = String::new();
    let escaped_title = escape_tdsl_string(title);
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
            let base = slug_ascii(&lane.label);
            let seed = if base.is_empty() {
                format!("lane_{}", i + 1)
            } else {
                base
            };
            // make_unique_alias 内部で lane_alias_seen に挿入済み
            make_unique_alias(&seed, &mut lane_alias_seen)
        };
        writeln!(
            out,
            r#"lane "{label}" as {alias} {{ kind custom; order {order}; }}"#,
            label = escape_tdsl_string(&lane.label),
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

fn parse_csv_items(path: &std::path::Path) -> Result<Vec<ImportedCsvItem>, String> {
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

        let parse_required_year = |field: &str, raw: &str| -> Result<i64, String> {
            if raw.is_empty() {
                return Err(format!("CSV row {row_no}: {field} must not be empty"));
            }
            raw.parse::<i64>()
                .map_err(|_| format!("CSV row {row_no}: {field} must be an integer"))
        };

        let (start, end, time) = match item_type {
            CsvItemType::Span | CsvItemType::EventRange => (
                Some(parse_required_year("start", &start_raw)?),
                Some(parse_required_year("end", &end_raw)?),
                None,
            ),
            CsvItemType::Event => (None, None, Some(parse_required_year("time", &time_raw)?)),
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

        items.push(ImportedCsvItem {
            lane,
            item_type,
            start,
            end,
            time,
            label,
            tags,
            id,
        });
    }

    if items.is_empty() {
        return Err(format!("CSV {} contains no data rows", path.display()));
    }

    Ok(items)
}

fn render_imported_csv_items(items: &[ImportedCsvItem]) -> String {
    let mut out = String::new();
    for item in items {
        let mut options = String::new();
        if !item.tags.is_empty() {
            let tags = item
                .tags
                .iter()
                .map(|t| format!(r#""{}""#, escape_tdsl_string(t)))
                .collect::<Vec<_>>()
                .join(", ");
            write!(options, "tags [{tags}]; ").unwrap();
        }
        if let Some(id) = &item.id {
            write!(options, r#"id "{}"; "#, escape_tdsl_string(id)).unwrap();
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
                    start = item.start.expect("validated start"),
                    end = item.end.expect("validated end"),
                    label = escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
            CsvItemType::Event => {
                writeln!(
                    out,
                    r#"event {lane} {time} "{label}" {options};"#,
                    lane = item.lane,
                    time = item.time.expect("validated time"),
                    label = escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
            CsvItemType::EventRange => {
                writeln!(
                    out,
                    r#"event_range {lane} {start}..{end} "{label}" {options};"#,
                    lane = item.lane,
                    start = item.start.expect("validated start"),
                    end = item.end.expect("validated end"),
                    label = escape_tdsl_string(&item.label),
                    options = block_options
                )
                .unwrap();
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LintSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct LintIssue {
    code: String,
    severity: LintSeverity,
    line: usize,
    message: String,
    fixable: bool,
}

#[derive(Debug, Clone, Serialize)]
struct LintReportOutput {
    file: String,
    fix_applied: usize,
    issue_count: usize,
    ok: bool,
    issues: Vec<LintIssue>,
}

fn cmd_lint(input: &std::path::Path, fix: bool, format: LintOutputFormat) -> Result<(), String> {
    let source = read_source(input)?;
    let mut file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;

    let mut fix_applied = 0usize;
    let mut lint_source = source.clone();
    if fix {
        fix_applied = apply_lint_fixes(&mut file);
        let rewritten = render_tdsl_file(&file);
        if rewritten != source {
            std::fs::write(input, &rewritten)
                .map_err(|e| format!("Failed to write {}: {e}", input.display()))?;
            lint_source = rewritten;
        }
    }

    let issues = lint_issues(&file, &lint_source);
    match format {
        LintOutputFormat::Text => {
            if fix {
                println!("Applied {fix_applied} fix(es) to {}", input.display());
            }
            if issues.is_empty() {
                println!("OK: no lint issues");
                return Ok(());
            }
            println!("Found {} issue(s):", issues.len());
            for issue in &issues {
                println!(
                    "- {severity} [{code}] line {line}: {message}{fixable}",
                    severity = match issue.severity {
                        LintSeverity::Error => "ERROR",
                        LintSeverity::Warning => "WARN",
                    },
                    code = issue.code,
                    line = issue.line,
                    message = issue.message,
                    fixable = if issue.fixable { " (fixable)" } else { "" }
                );
            }
        }
        LintOutputFormat::Json => {
            let report = LintReportOutput {
                file: input.display().to_string(),
                fix_applied,
                issue_count: issues.len(),
                ok: issues.is_empty(),
                issues,
            };
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        }
    }

    Ok(())
}

fn lint_issues(file: &tdsl_parser::ast::File, source: &str) -> Vec<LintIssue> {
    use tdsl_parser::ast::Statement;

    let lane_ids = collect_lane_ids(file);
    let mut seen_ids: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut issues = Vec::new();

    for stmt in &file.statements {
        let line = line_from_offset(source, stmt.span.start);
        match &stmt.node {
            Statement::Span(s) => {
                lint_item_common(
                    &lane_ids,
                    &mut seen_ids,
                    &s.lane_ref,
                    &s.label,
                    &s.props,
                    line,
                    &mut issues,
                );
                if s.start > s.end {
                    issues.push(LintIssue {
                        code: "start_gt_end".to_string(),
                        severity: LintSeverity::Error,
                        line,
                        message: format!("span range is reversed: {}..{}", s.start, s.end),
                        fixable: true,
                    });
                }
            }
            Statement::Event(e) => {
                lint_item_common(
                    &lane_ids,
                    &mut seen_ids,
                    &e.lane_ref,
                    &e.label,
                    &e.props,
                    line,
                    &mut issues,
                );
            }
            Statement::EventRange(er) => {
                lint_item_common(
                    &lane_ids,
                    &mut seen_ids,
                    &er.lane_ref,
                    &er.label,
                    &er.props,
                    line,
                    &mut issues,
                );
                if er.start > er.end {
                    issues.push(LintIssue {
                        code: "start_gt_end".to_string(),
                        severity: LintSeverity::Error,
                        line,
                        message: format!("event_range is reversed: {}..{}", er.start, er.end),
                        fixable: true,
                    });
                }
            }
            _ => {}
        }
    }

    issues
}

fn lint_item_common(
    lane_ids: &std::collections::HashSet<String>,
    seen_ids: &mut std::collections::HashMap<String, usize>,
    lane_ref: &str,
    label: &str,
    props: &tdsl_parser::ast::ItemProps,
    line: usize,
    issues: &mut Vec<LintIssue>,
) {
    if !lane_ids.contains(lane_ref) {
        issues.push(LintIssue {
            code: "unknown_lane".to_string(),
            severity: LintSeverity::Error,
            line,
            message: format!("unknown lane reference `{lane_ref}`"),
            fixable: false,
        });
    }

    if label.trim().is_empty() {
        issues.push(LintIssue {
            code: "empty_label".to_string(),
            severity: LintSeverity::Error,
            line,
            message: "label must not be empty".to_string(),
            fixable: false,
        });
    }

    let mut tag_seen = std::collections::HashSet::new();
    let mut has_empty_tag = false;
    let mut has_duplicate_tag = false;
    for tag in &props.tags {
        let normalized = tag.trim();
        if normalized.is_empty() {
            has_empty_tag = true;
            continue;
        }
        if !tag_seen.insert(normalized.to_string()) {
            has_duplicate_tag = true;
        }
    }
    if has_empty_tag || has_duplicate_tag {
        let reason = match (has_empty_tag, has_duplicate_tag) {
            (true, true) => "tags contain empty and duplicated elements",
            (true, false) => "tags contain empty elements",
            (false, true) => "tags contain duplicated elements",
            (false, false) => unreachable!(),
        };
        issues.push(LintIssue {
            code: "invalid_tags".to_string(),
            severity: LintSeverity::Error,
            line,
            message: reason.to_string(),
            fixable: true,
        });
    }

    match props
        .id
        .as_ref()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        Some(id) => {
            if let Some(first_line) = seen_ids.get(id) {
                issues.push(LintIssue {
                    code: "duplicate_id".to_string(),
                    severity: LintSeverity::Error,
                    line,
                    message: format!("id `{id}` duplicates line {first_line}"),
                    fixable: false,
                });
            } else {
                seen_ids.insert(id.to_string(), line);
            }
        }
        None => {
            issues.push(LintIssue {
                code: "missing_id".to_string(),
                severity: LintSeverity::Warning,
                line,
                message: "id is missing".to_string(),
                fixable: true,
            });
        }
    }
}

fn collect_lane_ids(file: &tdsl_parser::ast::File) -> std::collections::HashSet<String> {
    use tdsl_parser::ast::Statement;

    let mut out = std::collections::HashSet::new();
    let mut auto = 0usize;
    for stmt in &file.statements {
        if let Statement::Lane(lane) = &stmt.node {
            let id = match &lane.alias {
                Some(alias) => alias.clone(),
                None => {
                    let slug = lane_slug(&lane.label);
                    if slug.is_empty() {
                        let generated = format!("lane_{auto}");
                        auto += 1;
                        generated
                    } else {
                        slug
                    }
                }
            };
            out.insert(id);
        }
    }
    out
}

fn lane_slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

fn line_from_offset(source: &str, offset: usize) -> usize {
    let len = source.len();
    let clamped = offset.min(len);
    source.as_bytes()[..clamped]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

fn apply_lint_fixes(file: &mut tdsl_parser::ast::File) -> usize {
    use tdsl_parser::ast::Statement;

    let mut fixed = 0usize;
    let mut used_ids = std::collections::HashSet::new();
    for stmt in &file.statements {
        match &stmt.node {
            Statement::Span(s) => {
                if let Some(id) = s
                    .props
                    .id
                    .as_deref()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                {
                    used_ids.insert(id.to_string());
                }
            }
            Statement::Event(e) => {
                if let Some(id) = e
                    .props
                    .id
                    .as_deref()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                {
                    used_ids.insert(id.to_string());
                }
            }
            Statement::EventRange(er) => {
                if let Some(id) = er
                    .props
                    .id
                    .as_deref()
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                {
                    used_ids.insert(id.to_string());
                }
            }
            _ => {}
        }
    }

    for stmt in &mut file.statements {
        match &mut stmt.node {
            Statement::Span(s) => {
                fixed += fix_tags(&mut s.props.tags);
                if s.start > s.end {
                    std::mem::swap(&mut s.start, &mut s.end);
                    fixed += 1;
                }
                fixed +=
                    ensure_item_id("span", &s.lane_ref, s.start, &mut s.props.id, &mut used_ids);
            }
            Statement::Event(e) => {
                fixed += fix_tags(&mut e.props.tags);
                fixed +=
                    ensure_item_id("event", &e.lane_ref, e.time, &mut e.props.id, &mut used_ids);
            }
            Statement::EventRange(er) => {
                fixed += fix_tags(&mut er.props.tags);
                if er.start > er.end {
                    std::mem::swap(&mut er.start, &mut er.end);
                    fixed += 1;
                }
                fixed += ensure_item_id(
                    "event_range",
                    &er.lane_ref,
                    er.start,
                    &mut er.props.id,
                    &mut used_ids,
                );
            }
            _ => {}
        }
    }

    fixed
}

fn fix_tags(tags: &mut Vec<String>) -> usize {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for tag in tags.iter() {
        let normalized = tag.trim();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            out.push(normalized.to_string());
        }
    }
    if *tags != out {
        *tags = out;
        1
    } else {
        0
    }
}

fn ensure_item_id(
    prefix: &str,
    lane: &str,
    anchor: i64,
    id_slot: &mut Option<String>,
    used_ids: &mut std::collections::HashSet<String>,
) -> usize {
    if let Some(existing) = id_slot
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        used_ids.insert(existing.to_string());
        return 0;
    }

    let base = format!("{prefix}:{lane}:{anchor}");
    let mut candidate = base.clone();
    let mut i = 2usize;
    while used_ids.contains(&candidate) {
        candidate = format!("{base}_{i}");
        i += 1;
    }
    used_ids.insert(candidate.clone());
    *id_slot = Some(candidate);
    1
}

fn render_tdsl_file(file: &tdsl_parser::ast::File) -> String {
    use tdsl_parser::ast::Statement;

    let mut out = String::new();
    for (idx, stmt) in file.statements.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
            out.push('\n');
        }
        match &stmt.node {
            Statement::Timeline(t) => {
                writeln!(out, r#"timeline "{}" {{"#, escape_tdsl_string(&t.name)).unwrap();
                if let Some(title) = &t.title {
                    writeln!(out, r#"    title "{}";"#, escape_tdsl_string(title)).unwrap();
                }
                if let Some(unit) = &t.unit {
                    writeln!(out, "    unit {unit};").unwrap();
                }
                if let Some(range) = &t.range {
                    writeln!(out, "    range {}..{};", range.start, range.end).unwrap();
                }
                if let Some(calendar) = &t.calendar {
                    writeln!(out, "    calendar {calendar};").unwrap();
                }
                write!(out, "}}").unwrap();
            }
            Statement::Lane(l) => {
                write!(out, r#"lane "{}""#, escape_tdsl_string(&l.label)).unwrap();
                if let Some(alias) = &l.alias {
                    write!(out, " as {alias}").unwrap();
                }
                let mut props = Vec::new();
                if let Some(kind) = &l.kind {
                    props.push(format!("kind {kind};"));
                }
                if let Some(order) = l.order {
                    props.push(format!("order {order};"));
                }
                if props.is_empty() {
                    write!(out, " {{}}").unwrap();
                } else {
                    write!(out, " {{ {} }}", props.join(" ")).unwrap();
                }
            }
            Statement::Span(s) => {
                write!(
                    out,
                    r#"span {} {}..{} "{}" {};"#,
                    s.lane_ref,
                    s.start,
                    s.end,
                    escape_tdsl_string(&s.label),
                    render_item_props(&s.props)
                )
                .unwrap();
            }
            Statement::Event(e) => {
                write!(
                    out,
                    r#"event {} {} "{}" {};"#,
                    e.lane_ref,
                    e.time,
                    escape_tdsl_string(&e.label),
                    render_item_props(&e.props)
                )
                .unwrap();
            }
            Statement::EventRange(er) => {
                write!(
                    out,
                    r#"event_range {} {}..{} "{}" {};"#,
                    er.lane_ref,
                    er.start,
                    er.end,
                    escape_tdsl_string(&er.label),
                    render_item_props(&er.props)
                )
                .unwrap();
            }
            Statement::Import(imp) => {
                write!(out, "import {}", imp.source_type).unwrap();
                if let Some(alias) = &imp.alias {
                    write!(out, " as {alias}").unwrap();
                }
                writeln!(out, " {{").unwrap();
                for item in &imp.items {
                    match item {
                        tdsl_parser::ast::ImportItem::Entity { qid, alias } => {
                            write!(out, "    entity {qid}").unwrap();
                            if let Some(alias) = alias {
                                write!(out, " as {alias}").unwrap();
                            }
                            writeln!(out, ";").unwrap();
                        }
                        tdsl_parser::ast::ImportItem::Query { query, alias } => {
                            write!(out, r#"    query "{}""#, escape_tdsl_string(query)).unwrap();
                            if let Some(alias) = alias {
                                write!(out, " as {alias}").unwrap();
                            }
                            writeln!(out, ";").unwrap();
                        }
                    }
                }
                if let Some(policy) = imp.policy {
                    let policy_name = match policy {
                        tdsl_parser::ast::ReimportPolicy::MergeBySource => "merge_by_source",
                        tdsl_parser::ast::ReimportPolicy::OverwriteImported => "overwrite_imported",
                        tdsl_parser::ast::ReimportPolicy::KeepManual => "keep_manual",
                    };
                    writeln!(out, "    policy {policy_name};").unwrap();
                }
                write!(out, "}}").unwrap();
            }
            Statement::Map(m) => {
                let target = match m.target_type {
                    tdsl_parser::ast::MapTargetType::Span => "span",
                    tdsl_parser::ast::MapTargetType::Event => "event",
                    tdsl_parser::ast::MapTargetType::EventRange => "event_range",
                };
                writeln!(out, "map {} to {} {{", m.source_ref, target).unwrap();
                for prop in &m.props {
                    match prop {
                        tdsl_parser::ast::MapProp::Lane(lane) => {
                            writeln!(out, "    lane {lane};").unwrap();
                        }
                        tdsl_parser::ast::MapProp::Start(expr) => {
                            writeln!(out, "    start {};", render_map_expr(expr)).unwrap();
                        }
                        tdsl_parser::ast::MapProp::End(expr) => {
                            writeln!(out, "    end {};", render_map_expr(expr)).unwrap();
                        }
                        tdsl_parser::ast::MapProp::Time(expr) => {
                            writeln!(out, "    time {};", render_map_expr(expr)).unwrap();
                        }
                        tdsl_parser::ast::MapProp::Label(expr) => {
                            writeln!(out, "    label {};", render_label_expr(expr)).unwrap();
                        }
                        tdsl_parser::ast::MapProp::Tags(tags) => {
                            let joined = tags
                                .iter()
                                .map(|t| format!(r#""{}""#, escape_tdsl_string(t)))
                                .collect::<Vec<_>>()
                                .join(", ");
                            writeln!(out, "    tags [{joined}];").unwrap();
                        }
                    }
                }
                write!(out, "}}").unwrap();
            }
        }
    }
    out.push('\n');
    out
}

fn render_item_props(props: &tdsl_parser::ast::ItemProps) -> String {
    let mut parts = Vec::new();
    if !props.tags.is_empty() {
        let joined = props
            .tags
            .iter()
            .map(|t| format!(r#""{}""#, escape_tdsl_string(t)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("tags [{joined}];"));
    }
    if let Some(source) = &props.source {
        parts.push(format!("source {}:{};", source.prefix, source.qid));
    }
    if let Some(id) = &props.id {
        parts.push(format!(r#"id "{}";"#, escape_tdsl_string(id)));
    }
    if let Some(origin) = &props.origin {
        parts.push(format!("origin {origin};"));
    }
    if parts.is_empty() {
        "{}".to_string()
    } else {
        format!("{{ {} }}", parts.join(" "))
    }
}

fn render_map_expr(expr: &tdsl_parser::ast::MapExpr) -> String {
    if let Some(accessor) = &expr.accessor {
        format!("claim({}).{}", expr.claim.property, accessor)
    } else {
        format!("claim({})", expr.claim.property)
    }
}

fn render_label_expr(expr: &tdsl_parser::ast::LabelExpr) -> String {
    expr.fallbacks
        .iter()
        .map(|l| format!("label@{}", l.lang))
        .collect::<Vec<_>>()
        .join(" ?? ")
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
    fn parse_langs_dedup_and_trim() {
        let langs = parse_langs(" ja, en,ja, ,ZH ");
        assert_eq!(langs, vec!["ja", "en", "zh"]);
    }

    #[test]
    fn parse_qids_dedup_and_validate() {
        let qids = parse_qids("q7209, Q7183, q7209").unwrap();
        assert_eq!(qids, vec!["Q7209", "Q7183"]);
        assert!(parse_qids("X123").is_err());
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
    fn lint_issues_detects_initial_rule_set() {
        let src = r#"
timeline "Lint" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span b 20..10 "" { tags ["x", "", "x"]; id "dup"; };
event a 30 "E" { id "dup"; };
event a 40 "No ID" {};
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        let codes: std::collections::HashSet<String> =
            issues.iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains("unknown_lane"));
        assert!(codes.contains("duplicate_id"));
        assert!(codes.contains("start_gt_end"));
        assert!(codes.contains("empty_label"));
        assert!(codes.contains("invalid_tags"));
        assert!(codes.contains("missing_id"));
    }

    #[test]
    fn apply_lint_fixes_normalizes_tags_swaps_ranges_and_generates_ids() {
        let src = r#"
timeline "Fix" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span a 20..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
event_range a 50..40 "R" { tags ["war", "war"]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 5);

        let rendered = render_tdsl_file(&file);
        let reparsed = tdsl_parser::parse(&rendered).unwrap();
        let issues = lint_issues(&reparsed, &rendered);
        assert!(!issues.iter().any(|i| i.code == "start_gt_end"
            || i.code == "invalid_tags"
            || i.code == "missing_id"));

        let ir = tdsl_core::lower::lower_static(&reparsed).unwrap();
        assert_eq!(ir.items.len(), 3);
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
        assert!(err.contains("time must be an integer"));
    }
}
