use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

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
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn read_source(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

fn cmd_build(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    pretty: bool,
    offline: bool,
) -> Result<(), String> {
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
        let langs: Vec<&str> = lang.split(',').map(|s| s.trim()).collect();
        let entity = tdsl_wikidata::WikidataClient::get_entity(&client, qid, &langs)
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
