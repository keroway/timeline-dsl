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

/// #612〜#616（ADR 0003）: 秒精度・UTCオフセット(`Z`)構文を使った新規サンプル。
/// このスナップショットは秒/offsetフィールド（`*_second` / `*_offset_minutes`）を
/// 含むIR構造が意図せず変化しないことを保証する。
#[test]
fn snapshot_iss_docking_second_precision_ir() {
    let src = read_example("iss_docking_second_precision.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

/// #612〜#616（ADR 0003）: `+HH:MM` / `-HH:MM` / `Z` の複数オフセット表記を使った
/// 新規サンプル。offset付き値同士がUTC正規化されて比較されること（D2）を含め、
/// パース・loweringが壊れないことを保証する。
#[test]
fn snapshot_global_conference_timezones_ir() {
    let src = read_example("global_conference_timezones.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir);
}

/// #663: `note` / `link` / `color` / open-ended `now` を実演する新規サンプル。
/// `now` はビルド時点の現在年（UTC）に解決されるため（#550）、該当フィールドのみ
/// insta redaction で正規化し、年をまたいでもスナップショットが安定するようにする。
#[test]
fn snapshot_feature_showcase_ir() {
    let src = read_example("feature_showcase.tdsl");
    let file = tdsl_parser::parse(&src).unwrap();
    let ir = lower::lower_static(&file).unwrap();
    insta::assert_json_snapshot!(ir, {
        ".items[4].end" => "[resolved-now-year]",
    });
}

/// 既存の minute-level（秒・offsetなし）サンプルが、秒/offset対応実装後も
/// 引き続き変更なくパース・lowerできることを保証する回帰テスト（#616受け入れ条件）。
/// 対象は分精度の時刻構文を使う既存サンプル全件。
#[test]
fn existing_minute_level_examples_still_parse_and_lower_unchanged() {
    let minute_level_examples = [
        "apollo_11.tdsl",
        "apollo_11_hourly.tdsl",
        "china_dynasties.tdsl",
        "japanese_history.tdsl",
        "world_wars.tdsl",
        "sci_tech_timeline.tdsl",
        "fictional_empire.tdsl",
        "internet_history.tdsl",
        "grouped_dynasties.tdsl",
    ];
    // 注: template_apply_example.tdsl は `import wikidata` を含むため lower_static では
    // 実行できない（Wikidata連携が必要）。本テストは静的のみのサンプルを対象とする。
    for name in minute_level_examples {
        let src = read_example(name);
        let file = tdsl_parser::parse(&src)
            .unwrap_or_else(|e| panic!("{name} must still parse after second/offset support: {e}"));
        lower::lower_static(&file).unwrap_or_else(|e| {
            panic!("{name} must still lower after second/offset support: {e:?}")
        });
    }
}

/// `examples/*.tdsl` を列挙し、全件が parse でき、offline lowering（`lower_static`）で
/// error にならないことを保証する（#881）。手書きリストではなくディスク上の列挙なので、
/// 新規サンプルは既定で検査対象になり、追加時の抜け漏れが起きない。
/// `import wikidata` を含むサンプルも `lower_static` は import ブロックを警告として
/// 無視して通るため、除外リストは持たない（Wikidata 取得結果の検証は対象外）。
#[test]
fn all_examples_parse_and_lower_offline() {
    let dir = "../../examples";
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {dir}: {e}"))
        .map(|entry| entry.unwrap_or_else(|e| panic!("failed to read entry in {dir}: {e}")))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tdsl"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no .tdsl examples found in {dir}");

    for name in &names {
        let src = read_example(name);
        let file = tdsl_parser::parse(&src).unwrap_or_else(|e| panic!("{name} must parse: {e}"));
        lower::lower_static(&file).unwrap_or_else(|e| panic!("{name} must lower offline: {e:?}"));
    }
}
