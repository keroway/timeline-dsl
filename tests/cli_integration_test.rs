/// CLI integration tests for the `tdsl` binary.
///
/// These tests invoke the compiled binary via `std::process::Command` and verify
/// stdout/stderr/exit-code behaviour.  They must be run after `cargo build` so
/// that the binary exists at `target/debug/tdsl` (or the path returned by
/// `env!("CARGO_BIN_EXE_tdsl")`).
use std::process::Command;

fn tdsl_bin() -> Command {
    // CARGO_BIN_EXE_tdsl is set by Cargo when running integration tests for a
    // workspace member that declares a [[bin]] named "tdsl".
    Command::new(env!("CARGO_BIN_EXE_tdsl"))
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

/// `tdsl render --help` includes the `--watch` flag in its output.
#[test]
fn render_help_includes_watch_flag() {
    let out = tdsl_bin()
        .args(["render", "--help"])
        .output()
        .expect("failed to run tdsl");

    assert!(
        out.status.success(),
        "tdsl render --help exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).expect("non-UTF-8 stdout");
    assert!(
        stdout.contains("--watch"),
        "--watch flag should appear in render --help output"
    );
}

/// `tdsl render --watch` without `--output` exits non-zero with a helpful message.
#[test]
fn render_watch_without_output_fails() {
    let out = tdsl_bin()
        .args(["render", "examples/china_dynasties.tdsl", "--watch"])
        .output()
        .expect("failed to run tdsl");

    assert!(
        !out.status.success(),
        "tdsl render --watch without --output should exit non-zero"
    );

    let stderr = String::from_utf8(out.stderr).expect("non-UTF-8 stderr");
    assert!(
        stderr.contains("--output") || stderr.contains("--watch"),
        "error message should mention --output or --watch, got: {stderr}"
    );
}

/// `tdsl render --watch --format png` exits non-zero (PNG not supported in watch mode).
#[test]
fn render_watch_png_format_fails() {
    let out = tdsl_bin()
        .args([
            "render",
            "examples/china_dynasties.tdsl",
            "--watch",
            "--output",
            "/tmp/test_watch_out.png",
            "--format",
            "png",
        ])
        .output()
        .expect("failed to run tdsl");

    assert!(
        !out.status.success(),
        "tdsl render --watch --format png should exit non-zero"
    );

    let stderr = String::from_utf8(out.stderr).expect("non-UTF-8 stderr");
    assert!(
        stderr.contains("html") || stderr.contains("svg") || stderr.contains("watch"),
        "error message should mention html/svg or watch, got: {stderr}"
    );
}
