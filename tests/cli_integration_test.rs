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
