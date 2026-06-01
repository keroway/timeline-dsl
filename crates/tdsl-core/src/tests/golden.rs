/// Golden snapshot tests for examples/*.tdsl → IR JSON.
///
/// These tests ensure that the lowered IR for static example files does not
/// change unexpectedly. When an intentional change is made, update the
/// snapshots with:
///
///   INSTA_UPDATE=new cargo test -p tdsl-core golden
///   cargo insta review
///
/// For CI, snapshots are expected to match exactly; any diff causes a failure.
use crate::lower;

/// Read an example file relative to the workspace root.
fn read_example(name: &str) -> String {
    // Tests run from the crate directory; examples/ is two levels up.
    let path = format!("../../examples/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

#[test]
fn snapshot_china_dynasties_ir() {
    let src = read_example("china_dynasties.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_japanese_history_ir() {
    let src = read_example("japanese_history.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_world_wars_ir() {
    let src = read_example("world_wars.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_sci_tech_timeline_ir() {
    let src = read_example("sci_tech_timeline.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_fictional_empire_ir() {
    let src = read_example("fictional_empire.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_apollo_11_ir() {
    let src = read_example("apollo_11.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

#[test]
fn snapshot_internet_history_ir() {
    let src = read_example("internet_history.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}
