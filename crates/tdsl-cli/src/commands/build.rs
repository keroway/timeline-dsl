use std::path::PathBuf;

use indicatif::ProgressBar;
use schemars::schema_for;

/// `TimelineIr` の JSON Schema を標準出力（または `output` ファイル）へ書き出す。
pub(crate) fn cmd_json_schema(
    output: Option<&std::path::Path>,
    pretty: bool,
) -> Result<(), String> {
    let schema = schema_for!(tdsl_core::ir::TimelineIr);
    let json = if pretty {
        serde_json::to_string_pretty(&schema).map_err(|e| e.to_string())?
    } else {
        serde_json::to_string(&schema).map_err(|e| e.to_string())?
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

/// .tdsl ファイルを IR にコンパイルして JSON 出力する。
/// 複数ファイルは merge する。
pub(crate) fn cmd_build(
    inputs: &[PathBuf],
    output: Option<&std::path::Path>,
    pretty: bool,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    wikidata_timeout: std::time::Duration,
) -> Result<(), String> {
    if inputs.is_empty() {
        return Err(
            "at least one input FILE is required (or use --json-schema to output the schema)"
                .to_string(),
        );
    }

    let ir = if inputs.len() == 1 {
        load_ir(&inputs[0], offline, cache_opts, wikidata_timeout)?
    } else {
        let mut irs = Vec::with_capacity(inputs.len());
        for path in inputs {
            irs.push(load_ir(
                path,
                offline,
                cache_opts.clone(),
                wikidata_timeout,
            )?);
        }
        let (merged, warnings) = tdsl_core::merge::merge_irs(irs);
        for w in &warnings {
            eprintln!("Warning: {w}");
        }
        merged
    };

    let json = if pretty {
        serde_json::to_string_pretty(&ir).map_err(|e| e.to_string())?
    } else {
        serde_json::to_string(&ir).map_err(|e| e.to_string())?
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

/// Parse and lower a .tdsl file into an IR. Shared by `build` and `render`.
pub(crate) fn load_ir(
    input: &std::path::Path,
    offline: bool,
    cache_opts: tdsl_wikidata::CacheOptions,
    wikidata_timeout: std::time::Duration,
) -> Result<tdsl_core::ir::TimelineIr, String> {
    let source = super::read_source(input)?;
    let filename = input.display().to_string();
    let file = tdsl_parser::parse(&source).map_err(|e| {
        super::check::print_parse_error(&e, &source, &filename);
        String::new()
    })?;

    let ir = if offline {
        tdsl_core::lower::lower_static_with_diagnostics(&file, None)
    } else {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        let http_client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout);
        let client = tdsl_wikidata::CachedWikidataClient::new(http_client, cache_opts);
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            indicatif::ProgressStyle::with_template("{spinner} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        spinner.set_message("Wikidata からエンティティを取得中...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        let result = rt.block_on(tdsl_core::lower::lower_with_wikidata_and_diagnostics(
            &file, &client, None,
        ));
        spinner.finish_and_clear();
        result
    };

    let (ir, lower_warnings) = ir.map_err(|errs| {
        errs.iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    // Lowering 由来の非致命的警告（マップ対象が未解決でアイテム未生成 等）。
    for w in &lower_warnings {
        eprintln!("Warning: {w}");
    }

    let warnings = tdsl_core::validate::validate(&ir);
    for w in &warnings {
        eprintln!("Warning: {w}");
    }

    Ok(ir)
}
