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
    let file = tdsl_parser::parse(&source)
        .map_err(|e| super::check::render_parse_error(&e, &source, &filename))?;

    let ir = if offline {
        tdsl_core::lower::lower_static_with_diagnostics(&file, None)
    } else {
        let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
        let http_client = tdsl_wikidata::client::HttpWikidataClient::with_timeout(wikidata_timeout)
            .map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal but complete valid `.tdsl` source, used to exercise `load_ir` /
    /// `cmd_build` offline (no Wikidata network access) and deterministically.
    const MINIMAL_TDSL: &str = r#"
timeline "Test" {
    title "Test";
    unit year;
    range 0..100;
    calendar proleptic_gregorian;
}

lane "A" as a { kind custom; order 1; }

event a 10 "E1" { id "e1"; };
"#;

    fn write_temp_tdsl(name: &str, contents: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "tdsl_build_test_{}_{}_{}",
            std::process::id(),
            n,
            name
        ));
        std::fs::write(&path, contents).expect("failed to write temp .tdsl fixture");
        path
    }

    fn default_cache_opts() -> tdsl_wikidata::CacheOptions {
        tdsl_wikidata::CacheOptions {
            no_cache: false,
            ttl: std::time::Duration::from_secs(86400),
        }
    }

    #[test]
    fn cmd_build_errors_when_inputs_empty() {
        let err = cmd_build(
            &[],
            None,
            false,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect_err("empty inputs must error");
        assert!(
            err.contains("at least one input FILE is required"),
            "unexpected error message: {err}"
        );
        assert!(
            err.contains("--json-schema"),
            "error should mention the --json-schema escape hatch: {err}"
        );
    }

    #[test]
    fn cmd_build_offline_writes_output_file() {
        let input = write_temp_tdsl("in.tdsl", MINIMAL_TDSL);
        let out_path = std::env::temp_dir().join(format!(
            "tdsl_build_test_out_{}_{}.json",
            std::process::id(),
            line!()
        ));

        cmd_build(
            std::slice::from_ref(&input),
            Some(&out_path),
            false,
            true, // offline: must not touch the network
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect("offline build of minimal valid source should succeed");

        let written = std::fs::read_to_string(&out_path).expect("output file must be written");
        let value: serde_json::Value =
            serde_json::from_str(&written).expect("output must be valid JSON");
        assert_eq!(value["meta"]["title"], "Test");
        assert_eq!(value["lanes"].as_array().map(|a| a.len()), Some(1));

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn cmd_build_offline_pretty_vs_compact_output_differs() {
        let input = write_temp_tdsl("in_pretty.tdsl", MINIMAL_TDSL);
        let pretty_path = std::env::temp_dir().join(format!(
            "tdsl_build_test_pretty_{}_{}.json",
            std::process::id(),
            line!()
        ));
        let compact_path = std::env::temp_dir().join(format!(
            "tdsl_build_test_compact_{}_{}.json",
            std::process::id(),
            line!()
        ));

        cmd_build(
            std::slice::from_ref(&input),
            Some(&pretty_path),
            true,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect("pretty offline build should succeed");
        cmd_build(
            std::slice::from_ref(&input),
            Some(&compact_path),
            false,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect("compact offline build should succeed");

        let pretty = std::fs::read_to_string(&pretty_path).unwrap();
        let compact = std::fs::read_to_string(&compact_path).unwrap();
        assert!(pretty.contains('\n'), "pretty output should be multi-line");
        assert!(
            !compact.contains('\n'),
            "compact output should be single-line"
        );
        // Both must parse to the same semantic JSON value.
        let pv: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        let cv: serde_json::Value = serde_json::from_str(&compact).unwrap();
        assert_eq!(pv, cv);

        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&pretty_path);
        let _ = std::fs::remove_file(&compact_path);
    }

    #[test]
    fn cmd_build_offline_merges_multiple_inputs() {
        let input_a = write_temp_tdsl("merge_a.tdsl", MINIMAL_TDSL);
        let second = MINIMAL_TDSL
            .replace("lane \"A\" as a", "lane \"B\" as b")
            .replace("event a 10", "event b 20")
            .replace("\"e1\"", "\"e2\"");
        let input_b = write_temp_tdsl("merge_b.tdsl", &second);
        let out_path = std::env::temp_dir().join(format!(
            "tdsl_build_test_merged_{}_{}.json",
            std::process::id(),
            line!()
        ));

        cmd_build(
            &[input_a.clone(), input_b.clone()],
            Some(&out_path),
            false,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect("offline merge build should succeed");

        let written = std::fs::read_to_string(&out_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&written).unwrap();
        assert_eq!(
            value["lanes"].as_array().map(|a| a.len()),
            Some(2),
            "merged IR should contain both lanes: {value}"
        );

        let _ = std::fs::remove_file(&input_a);
        let _ = std::fs::remove_file(&input_b);
        let _ = std::fs::remove_file(&out_path);
    }

    #[test]
    fn cmd_build_returns_actionable_parse_error() {
        let input = write_temp_tdsl(
            "invalid_syntax.tdsl",
            r#"
                timeline "Test" {
                    unit year;
                    range 0..100;
            "#,
        );

        let err = cmd_build(
            std::slice::from_ref(&input),
            None,
            false,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect_err("invalid DSL must return its parse diagnostic");

        assert!(!err.trim().is_empty(), "parse error must not be empty");
        assert!(
            err.contains("構文エラー"),
            "parse error must contain an actionable diagnostic: {err}"
        );

        let _ = std::fs::remove_file(&input);
    }

    #[test]
    fn cmd_build_errors_on_missing_input_file() {
        let missing = std::env::temp_dir().join(format!(
            "tdsl_build_test_missing_{}_{}.tdsl",
            std::process::id(),
            line!()
        ));
        let err = cmd_build(
            std::slice::from_ref(&missing),
            None,
            false,
            true,
            default_cache_opts(),
            std::time::Duration::from_secs(30),
        )
        .expect_err("missing input file must error");
        assert!(
            err.contains("Failed to read"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn cmd_json_schema_pretty_vs_compact() {
        let pretty_path = std::env::temp_dir().join(format!(
            "tdsl_schema_test_pretty_{}_{}.json",
            std::process::id(),
            line!()
        ));
        let compact_path = std::env::temp_dir().join(format!(
            "tdsl_schema_test_compact_{}_{}.json",
            std::process::id(),
            line!()
        ));

        cmd_json_schema(Some(&pretty_path), true).expect("pretty schema write should succeed");
        cmd_json_schema(Some(&compact_path), false).expect("compact schema write should succeed");

        let pretty = std::fs::read_to_string(&pretty_path).unwrap();
        let compact = std::fs::read_to_string(&compact_path).unwrap();
        assert!(pretty.contains('\n'));
        assert!(!compact.contains('\n'));
        assert!(pretty.contains("$schema"));

        let _ = std::fs::remove_file(&pretty_path);
        let _ = std::fs::remove_file(&compact_path);
    }

    #[test]
    fn cmd_json_schema_errors_on_unwritable_output_path() {
        let bad_path = std::path::Path::new("/nonexistent-dir-for-tdsl-tests/out.json");
        let err = cmd_json_schema(Some(bad_path), false).expect_err("unwritable path must error");
        assert!(
            err.contains("Failed to write"),
            "unexpected error message: {err}"
        );
    }
}
