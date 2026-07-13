/// CLI integration tests for the `tdsl` binary.
///
/// These tests invoke the compiled binary via `std::process::Command` and verify
/// stdout/stderr/exit-code behaviour.  They must be run after `cargo build` so
/// that the binary exists at `target/debug/tdsl` (or the path returned by
/// `env!("CARGO_BIN_EXE_tdsl")`).
use std::path::{Path, PathBuf};
use std::process::Command;

fn tdsl_bin() -> Command {
    // CARGO_BIN_EXE_tdsl is set by Cargo when running integration tests for a
    // workspace member that declares a [[bin]] named "tdsl".
    Command::new(env!("CARGO_BIN_EXE_tdsl"))
}

/// Resolve a path relative to the workspace root, independent of the test's CWD.
///
/// `CARGO_MANIFEST_DIR` points at `crates/tdsl-cli`; the workspace root is two
/// levels up. Passing absolute paths to the binary makes these tests robust
/// regardless of the working directory the harness chooses.
fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// Allocate a unique temp path (process- and call-unique) for output assertions.
fn unique_temp(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("tdsl_it_{}_{}_{}", std::process::id(), n, name))
}

/// Count `lanes` entries in a build's JSON IR stdout.
fn lane_count(stdout: &str) -> usize {
    let value: serde_json::Value = serde_json::from_str(stdout).expect("stdout is not valid JSON");
    value
        .get("lanes")
        .and_then(|l| l.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

/// `tdsl build --json-schema` outputs valid JSON with top-level IR keys.
#[test]
fn build_json_schema_outputs_valid_json() {
    let out = tdsl_bin()
        .args(["build", "--json-schema"])
        .output()
        .expect("failed to run tdsl");

    assert!(
        out.status.success(),
        "tdsl build --json-schema exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is not valid JSON");

    assert!(
        value.get("$schema").is_some(),
        "JSON Schema output should have a $schema key"
    );
}

/// `tdsl build --json-schema` schema contains meta / lanes / items definitions.
#[test]
fn build_json_schema_contains_ir_fields() {
    let out = tdsl_bin()
        .args(["build", "--json-schema"])
        .output()
        .expect("failed to run tdsl");

    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(stdout.contains("\"meta\""), "schema should mention 'meta'");
    assert!(
        stdout.contains("\"lanes\""),
        "schema should mention 'lanes'"
    );
    assert!(
        stdout.contains("\"items\""),
        "schema should mention 'items'"
    );
}

/// `tdsl build --json-schema --pretty` outputs indented JSON.
#[test]
fn build_json_schema_pretty_is_indented() {
    let out = tdsl_bin()
        .args(["build", "--json-schema", "--pretty"])
        .output()
        .expect("failed to run tdsl");

    assert!(out.status.success());

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    // Pretty-printed JSON contains newlines and leading spaces.
    assert!(
        stdout.contains('\n') && stdout.contains("  "),
        "pretty output should be indented"
    );
}

/// `tdsl build` without a FILE argument and without `--json-schema` exits non-zero.
#[test]
fn build_without_file_and_without_json_schema_fails() {
    let out = tdsl_bin()
        .arg("build")
        .output()
        .expect("failed to run tdsl");

    assert!(!out.status.success(), "tdsl build with no args should fail");
}

/// `tdsl render --help` output contains the `--watch` flag.
#[test]
fn render_help_includes_watch_flag() {
    let out = tdsl_bin()
        .args(["render", "--help"])
        .output()
        .expect("failed to run tdsl");

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(
        stdout.contains("--watch"),
        "--watch should appear in render --help output"
    );
    assert!(
        stdout.contains("--show-legend"),
        "--show-legend should appear in render --help output"
    );
}

/// `tdsl render --watch` without `--output` exits non-zero.
#[test]
fn render_watch_without_output_fails() {
    let out = tdsl_bin()
        .args(["render", "examples/china_dynasties.tdsl", "--watch"])
        .output()
        .expect("failed to run tdsl");

    assert!(
        !out.status.success(),
        "--watch without --output should exit non-zero"
    );
}

/// `tdsl render --watch --format png` exits non-zero (png not supported in watch mode).
#[test]
fn render_watch_png_format_fails() {
    let out = tdsl_bin()
        .args([
            "render",
            "examples/china_dynasties.tdsl",
            "--watch",
            "--output",
            "/tmp/out.png",
            "--format",
            "png",
        ])
        .output()
        .expect("failed to run tdsl");

    assert!(
        !out.status.success(),
        "--watch --format png should exit non-zero"
    );
}

// ---------------------------------------------------------------------------
// build: static (offline) compilation to IR JSON
// ---------------------------------------------------------------------------

/// `tdsl build <file> --offline` produces valid IR JSON with the core IR keys.
#[test]
fn build_single_file_offline_outputs_ir_json() {
    let out = tdsl_bin()
        .arg("build")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .arg("--offline")
        .output()
        .expect("failed to run tdsl");

    assert!(
        out.status.success(),
        "build --offline exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is not valid JSON");
    assert!(value.get("meta").is_some(), "IR should contain `meta`");
    assert!(
        value
            .get("lanes")
            .and_then(|l| l.as_array())
            .is_some_and(|a| !a.is_empty()),
        "IR should contain a non-empty `lanes` array"
    );
    assert!(value.get("items").is_some(), "IR should contain `items`");
}

/// `tdsl build f1 f2 --offline` merges multiple files (more lanes than either alone).
#[test]
fn build_multiple_files_merge_offline() {
    let single = tdsl_bin()
        .arg("build")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .arg("--offline")
        .output()
        .expect("failed to run tdsl");
    assert!(single.status.success());
    let single_lanes = lane_count(&String::from_utf8(single.stdout).expect("utf8"));

    let merged = tdsl_bin()
        .arg("build")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .arg(repo_path("examples/apollo_11.tdsl"))
        .arg("--offline")
        .output()
        .expect("failed to run tdsl");
    assert!(
        merged.status.success(),
        "merge build exited non-zero: {}",
        String::from_utf8_lossy(&merged.stderr)
    );
    let merged_lanes = lane_count(&String::from_utf8(merged.stdout).expect("utf8"));

    assert!(
        merged_lanes > single_lanes,
        "merged IR should have more lanes ({merged_lanes}) than the single file ({single_lanes})"
    );
}

/// `tdsl build <file> --offline --output <path>` writes valid JSON to a file.
#[test]
fn build_output_flag_writes_file() {
    let out_path = unique_temp("build_out.json");
    let out = tdsl_bin()
        .arg("build")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .arg("--offline")
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to run tdsl");
    assert!(out.status.success(), "build --output exited non-zero");

    let written = std::fs::read_to_string(&out_path).expect("output file should exist");
    let _: serde_json::Value =
        serde_json::from_str(&written).expect("written file should be valid JSON");
    let _ = std::fs::remove_file(&out_path);
}

// ---------------------------------------------------------------------------
// check / lint: exit codes and strict validation
// ---------------------------------------------------------------------------

/// `tdsl check` on a valid file exits zero.
#[test]
fn check_valid_file_exits_zero() {
    let out = tdsl_bin()
        .arg("check")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .output()
        .expect("failed to run tdsl");
    assert!(
        out.status.success(),
        "check on a valid file should exit zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// `tdsl check` on a syntactically invalid file exits non-zero.
#[test]
fn check_invalid_syntax_exits_nonzero() {
    let out = tdsl_bin()
        .arg("check")
        .arg(repo_path("tests/fixtures/invalid_syntax.tdsl"))
        .output()
        .expect("failed to run tdsl");
    assert!(
        !out.status.success(),
        "check on invalid syntax should exit non-zero"
    );
}

/// `tdsl check` on an unknown-lane reference exits non-zero (no silent fallback).
#[test]
fn check_unknown_lane_reference_exits_nonzero() {
    let out = tdsl_bin()
        .arg("check")
        .arg(repo_path("tests/fixtures/invalid_semantics.tdsl"))
        .output()
        .expect("failed to run tdsl");
    assert!(
        !out.status.success(),
        "check on an unknown lane reference must fail (AGENTS.md §4.1 No silent fallback)"
    );
}

/// `tdsl lint` on a clean file exits zero and reports no issues.
#[test]
fn lint_clean_file_exits_zero() {
    let out = tdsl_bin()
        .arg("lint")
        .arg(repo_path("examples/china_dynasties.tdsl"))
        .output()
        .expect("failed to run tdsl");
    assert!(
        out.status.success(),
        "lint on a clean file should exit zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ---------------------------------------------------------------------------
// import-csv: stdout / --output / --append
// ---------------------------------------------------------------------------

/// `tdsl import-csv <csv>` emits span/event/event_range items to stdout.
#[test]
fn import_csv_stdout_contains_items() {
    let out = tdsl_bin()
        .arg("import-csv")
        .arg(repo_path("examples/fictional_empire_items.csv"))
        .output()
        .expect("failed to run tdsl");
    assert!(
        out.status.success(),
        "import-csv exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(stdout.contains("span kingdom"), "should emit a span item");
    assert!(
        stdout.contains("event_range incidents"),
        "should emit an event_range item"
    );
}

/// `tdsl import-csv <csv> --output <path>` writes the snippet to a file.
#[test]
fn import_csv_output_flag_writes_file() {
    let out_path = unique_temp("csv_out.tdsl");
    let out = tdsl_bin()
        .arg("import-csv")
        .arg(repo_path("examples/fictional_empire_items.csv"))
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to run tdsl");
    assert!(out.status.success(), "import-csv --output exited non-zero");

    let written = std::fs::read_to_string(&out_path).expect("output file should exist");
    assert!(
        written.contains("span kingdom"),
        "written snippet should contain the imported span"
    );
    let _ = std::fs::remove_file(&out_path);
}

/// `tdsl import-csv <csv> --append <existing>` appends to an existing file.
#[test]
fn import_csv_append_grows_existing_file() {
    let target = unique_temp("csv_append.tdsl");
    let seed = "// existing content\n";
    std::fs::write(&target, seed).expect("seed write");
    let before = std::fs::metadata(&target).expect("meta").len();

    let out = tdsl_bin()
        .arg("import-csv")
        .arg(repo_path("examples/fictional_empire_items.csv"))
        .arg("--append")
        .arg(&target)
        .output()
        .expect("failed to run tdsl");
    assert!(
        out.status.success(),
        "import-csv --append exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let after_content = std::fs::read_to_string(&target).expect("target should exist");
    assert!(
        after_content.starts_with(seed),
        "append must preserve existing content"
    );
    assert!(
        after_content.contains("span kingdom"),
        "append must add imported items"
    );
    assert!(
        after_content.len() as u64 > before,
        "file should grow after append"
    );
    let _ = std::fs::remove_file(&target);
}

// ---------------------------------------------------------------------------
// export-csv: stdout / --output / .json input / import round-trip
// ---------------------------------------------------------------------------

/// `tdsl export-csv <tdsl> --offline` emits a CSV header and item rows to stdout.
#[test]
fn export_csv_stdout_contains_header_and_rows() {
    let out = tdsl_bin()
        .arg("export-csv")
        .arg(repo_path("examples/fictional_empire.tdsl"))
        .arg("--offline")
        .output()
        .expect("failed to run tdsl");
    assert!(
        out.status.success(),
        "export-csv exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    let mut lines = stdout.lines();
    assert_eq!(
        lines.next().unwrap(),
        "lane,type,start,end,time,label,tags,id,source,origin",
        "first line must be the CSV header"
    );
    assert!(
        lines.next().is_some_and(|l| l.starts_with("kingdom,span,")),
        "should emit the kingdom span row"
    );
}

/// `tdsl export-csv <tdsl> --output <path>` writes the CSV to a file.
#[test]
fn export_csv_output_flag_writes_file() {
    let out_path = unique_temp("export.csv");
    let out = tdsl_bin()
        .arg("export-csv")
        .arg(repo_path("examples/fictional_empire.tdsl"))
        .arg("--offline")
        .arg("--output")
        .arg(&out_path)
        .output()
        .expect("failed to run tdsl");
    assert!(out.status.success(), "export-csv --output exited non-zero");
    let written = std::fs::read_to_string(&out_path).expect("output file should exist");
    assert!(written.starts_with("lane,type,start,end,time,label,tags,id,source,origin"));
    let _ = std::fs::remove_file(&out_path);
}

/// `tdsl export-csv` accepts a `.json` IR file (build → export round-trip via IR).
#[test]
fn export_csv_accepts_json_ir_input() {
    let ir_path = unique_temp("ir.json");
    let built = tdsl_bin()
        .args(["build", "--offline", "--output"])
        .arg(&ir_path)
        .arg(repo_path("examples/fictional_empire.tdsl"))
        .output()
        .expect("failed to run tdsl build");
    assert!(built.status.success(), "build exited non-zero");

    let out = tdsl_bin()
        .arg("export-csv")
        .arg(&ir_path)
        .output()
        .expect("failed to run tdsl export-csv");
    assert!(out.status.success(), "export-csv (json) exited non-zero");
    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(stdout.starts_with("lane,type,start,end,time,label,tags,id,source,origin"));
    let _ = std::fs::remove_file(&ir_path);
}

/// `export-csv` → `import-csv` round-trips item lines (8 import columns are stable).
#[test]
fn export_csv_then_import_csv_round_trips() {
    let csv_path = unique_temp("roundtrip.csv");
    let exported = tdsl_bin()
        .arg("export-csv")
        .arg(repo_path("examples/fictional_empire.tdsl"))
        .arg("--offline")
        .arg("--output")
        .arg(&csv_path)
        .output()
        .expect("failed to run tdsl export-csv");
    assert!(exported.status.success(), "export-csv exited non-zero");

    let reimported = tdsl_bin()
        .arg("import-csv")
        .arg(&csv_path)
        .output()
        .expect("failed to run tdsl import-csv");
    assert!(
        reimported.status.success(),
        "import-csv exited non-zero: {}",
        String::from_utf8_lossy(&reimported.stderr)
    );
    let snippet = String::from_utf8(reimported.stdout).expect("non-UTF-8 stdout");
    assert!(
        snippet.contains("span kingdom 1001..1180"),
        "round-trip must preserve the span item: {snippet}"
    );
    assert!(
        snippet.contains("event_range incidents 1175..1180"),
        "round-trip must preserve the event_range item: {snippet}"
    );
    let _ = std::fs::remove_file(&csv_path);
}

/// #608: `export-csv` → `import-csv` → `build` の往復で `source`/`origin`（wd:Q… / wikidata）
/// が保持され、再度 IR 化できることを検証する。ネットワーク依存を避けるため Wikidata 連携は
/// 使わず、静的 `.tdsl` に手動で `source`/`origin` を付与した fixture を使う（AGENTS.md §5、
/// implementation-strict.md §5）。
#[test]
fn export_csv_then_import_csv_preserves_provenance() {
    let tdsl_path = unique_temp("provenance_source.tdsl");
    std::fs::write(
        &tdsl_path,
        r#"timeline "Provenance Test" {
    title "Provenance Test";
    unit year;
    range 1900..2000;
    calendar proleptic_gregorian;
}

lane "Missions" as missions { kind custom; order 10; }

event missions 1969 "Apollo 11 landing" {
    id "event:apollo";
    source wd:Q43653;
    origin wikidata;
};

event missions 1901 "Hand-written record" {
    id "event:manual";
};
"#,
    )
    .unwrap();

    let csv_path = unique_temp("provenance.csv");
    let exported = tdsl_bin()
        .arg("export-csv")
        .arg(&tdsl_path)
        .arg("--offline")
        .arg("--output")
        .arg(&csv_path)
        .output()
        .expect("failed to run tdsl export-csv");
    assert!(
        exported.status.success(),
        "export-csv exited non-zero: {}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let csv = std::fs::read_to_string(&csv_path).expect("csv output should exist");
    assert!(
        csv.contains("wd:Q43653") && csv.contains("wikidata"),
        "exported CSV must carry provenance: {csv}"
    );

    let reimported = tdsl_bin()
        .arg("import-csv")
        .arg(&csv_path)
        .output()
        .expect("failed to run tdsl import-csv");
    assert!(
        reimported.status.success(),
        "import-csv exited non-zero: {}",
        String::from_utf8_lossy(&reimported.stderr)
    );
    let snippet = String::from_utf8(reimported.stdout).expect("non-UTF-8 stdout");
    assert!(
        snippet.contains("source wd:Q43653;"),
        "round-trip must preserve source: {snippet}"
    );
    assert!(
        snippet.contains("origin wikidata;"),
        "round-trip must preserve origin: {snippet}"
    );

    // 再度ビルドして意味的に妥当な DSL/IR であることを確認する。
    let snippet_path = unique_temp("provenance_reimported.tdsl");
    std::fs::write(&snippet_path, format!(
        "timeline \"T\" {{\n    title \"T\";\n    unit year;\n    range 1900..2000;\n    calendar proleptic_gregorian;\n}}\n\nlane \"Missions\" as missions {{ kind custom; order 10; }}\n\n{snippet}"
    ))
    .unwrap();
    let rebuilt = tdsl_bin()
        .args(["build", "--offline"])
        .arg(&snippet_path)
        .output()
        .expect("failed to run tdsl build");
    assert!(
        rebuilt.status.success(),
        "rebuilding the re-imported snippet must succeed: {}",
        String::from_utf8_lossy(&rebuilt.stderr)
    );

    let _ = std::fs::remove_file(&tdsl_path);
    let _ = std::fs::remove_file(&csv_path);
    let _ = std::fs::remove_file(&snippet_path);
}

/// #608: 不正な provenance（`origin=wikidata` なのに `source` が `wd:Q<id>` 形式でない）を
/// 含む CSV は `import-csv` が非ゼロ終了・エラーメッセージで拒否する（silent に破棄しない）。
#[test]
fn import_csv_rejects_malformed_provenance() {
    let csv_path = unique_temp("malformed_provenance.csv");
    std::fs::write(
        &csv_path,
        "lane,type,start,end,time,label,tags,id,source,origin\n\
missions,event,,,1969,Apollo 11,,event:apollo,,wikidata\n",
    )
    .unwrap();

    let out = tdsl_bin()
        .arg("import-csv")
        .arg(&csv_path)
        .output()
        .expect("failed to run tdsl import-csv");
    assert!(
        !out.status.success(),
        "import-csv must fail on malformed provenance"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("origin=wikidata requires"),
        "stderr must explain the provenance error: {stderr}"
    );
    let _ = std::fs::remove_file(&csv_path);
}
