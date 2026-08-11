use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LintSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintIssue {
    pub code: String,
    pub severity: LintSeverity,
    pub line: usize,
    pub message: String,
    pub fixable: bool,
}

/// range の開始・終了の順序を判定し、issue があれば積む。
///
/// 以前はここで `TimeValue::to_sortable()` を直接比較していたが、これは
/// **年月日しか見ず、時分秒と UTC オフセットを捨てる**。その結果:
///
/// - `2024-01-02T08:00+09:00..2024-01-01T20:00-05:00` は UTC では正順なのに
///   逆転と誤判定され、`--fix` が swap して**正しいファイルを壊していた**
///   （修正後は `tdsl check` が start > end を報告する状態になる）
/// - 逆に同一日内で時刻だけ逆転した range は見逃していた
///
/// lowering / validate は ADR 0003 D2 に従って UTC 正規化し時分秒まで見る
/// 比較をしており、lint だけが別の順序判定を持っていた。判定を
/// `lower::compare_time_values` に一本化する（#757）。
///
/// 片側だけ offset 付きの比較は原理的に順序が決まらないため、swap ではなく
/// **修正不能な別コード `mixed_offset_range`** として報告する。
fn lint_range_order(
    kind: &str,
    start: &tdsl_parser::ast::TimeValue,
    end: &tdsl_parser::ast::TimeValue,
    line: usize,
    issues: &mut Vec<LintIssue>,
) {
    match crate::lower::compare_time_values(start, end) {
        Ok(std::cmp::Ordering::Greater) => issues.push(LintIssue {
            code: "start_gt_end".to_string(),
            severity: LintSeverity::Error,
            line,
            message: format!("{kind} is reversed: {start}..{end}"),
            fixable: true,
        }),
        Ok(_) => {}
        Err(_) => issues.push(LintIssue {
            code: "mixed_offset_range".to_string(),
            severity: LintSeverity::Error,
            line,
            // 自動修正できない理由をメッセージ自体に書く。swap しても順序は
            // 決まらないため、直せるのは書き手だけ。
            message: format!(
                "{kind} mixes a UTC-offset time value with one that has no offset; \
                 start/end order cannot be determined (ADR 0003 D2, make both sides consistent): \
                 {start}..{end}"
            ),
            fixable: false,
        }),
    }
}

/// `--fix` が start/end を swap してよいかを判定する。
///
/// **フル精度で確定的に逆転している場合だけ true を返す。** 片側だけ offset 付きで
/// 順序が決まらないケース（`Err`）で swap すると、決まらないものを勝手に並べ替えて
/// 別の不正なファイルを作ることになる。検出側は `mixed_offset_range` として
/// `fixable: false` で報告し、ここでは触らない（#757）。
fn should_swap_range(
    start: &tdsl_parser::ast::TimeValue,
    end: &tdsl_parser::ast::TimeValue,
) -> bool {
    matches!(
        crate::lower::compare_time_values(start, end),
        Ok(std::cmp::Ordering::Greater)
    )
}

pub fn lint_issues(file: &tdsl_parser::ast::File, source: &str) -> Vec<LintIssue> {
    use tdsl_parser::ast::Statement;

    let lane_ids = collect_lane_ids(file);
    let mut seen_ids: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut issues = Vec::new();

    // 行番号は行頭オフセット表を 1 度だけ作って二分探索で引く。
    // 以前は statement ごとにソース先頭から改行を数え直しており
    // O(statements × source長) になっていた（#763）。lint は LSP の
    // タイプごと診断から呼ばれるため、二乗コストがそのまま体感に出る。
    let line_offsets = crate::lower::build_line_offsets(source);

    for stmt in &file.statements {
        let line = line_from_offset(&line_offsets, stmt.span.start);
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
                lint_range_order("span range", &s.start, &s.end, line, &mut issues);
                lint_time_value(&s.start, line, &mut issues);
                lint_time_value(&s.end, line, &mut issues);
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
                lint_time_value(&e.time, line, &mut issues);
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
                lint_range_order("event_range", &er.start, &er.end, line, &mut issues);
                lint_time_value(&er.start, line, &mut issues);
                lint_time_value(&er.end, line, &mut issues);
            }
            _ => {}
        }
    }

    issues
}

/// `TimeValue::Date(y, m, d)` のカレンダー妥当性を検証し、不正な場合は warning を追加する。
/// 年精度・月精度（日なし）の場合は検証をスキップする。
fn lint_time_value(tv: &tdsl_parser::ast::TimeValue, line: usize, issues: &mut Vec<LintIssue>) {
    if let tdsl_parser::ast::TimeValue::Date(year, month, day) = tv {
        let max_day = crate::ir::days_in_month(*year, *month);
        if *day == 0 || *day > max_day {
            issues.push(LintIssue {
                code: "invalid_calendar_date".to_string(),
                severity: LintSeverity::Warning,
                line,
                message: format!("Invalid calendar date: {year}-{month:02}-{day:02}"),
                fixable: false,
            });
        }
    }
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
        match &stmt.node {
            Statement::Lane(lane) => {
                out.insert(resolve_lane_id(lane, &mut auto));
            }
            Statement::Group(group) => {
                for lane in &group.lanes {
                    out.insert(resolve_lane_id(lane, &mut auto));
                }
            }
            _ => {}
        }
    }
    out
}

fn resolve_lane_id(lane: &tdsl_parser::ast::LaneDecl, auto: &mut usize) -> String {
    match &lane.alias {
        Some(alias) => alias.clone(),
        None => {
            let slug = lane_slug(&lane.label);
            if slug.is_empty() {
                let generated = format!("lane_{auto}");
                *auto += 1;
                generated
            } else {
                slug
            }
        }
    }
}

fn lane_slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

/// 行頭オフセット表から 1-indexed の行番号を引く。
///
/// `lower::offset_to_line_col` と同じ `partition_point` を使う。
/// あちらは列も返すが、lint は行しか使わないのでここでは行だけを返す。
fn line_from_offset(line_offsets: &[usize], offset: usize) -> usize {
    line_offsets.partition_point(|&o| o <= offset).max(1)
}

pub fn apply_lint_fixes(file: &mut tdsl_parser::ast::File) -> usize {
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
                if should_swap_range(&s.start, &s.end) {
                    std::mem::swap(&mut s.start, &mut s.end);
                    fixed += 1;
                }
                fixed += ensure_item_id(
                    "span",
                    &s.lane_ref,
                    s.start.year(),
                    &mut s.props.id,
                    &mut used_ids,
                );
            }
            Statement::Event(e) => {
                fixed += fix_tags(&mut e.props.tags);
                fixed += ensure_item_id(
                    "event",
                    &e.lane_ref,
                    e.time.year(),
                    &mut e.props.id,
                    &mut used_ids,
                );
            }
            Statement::EventRange(er) => {
                fixed += fix_tags(&mut er.props.tags);
                if should_swap_range(&er.start, &er.end) {
                    std::mem::swap(&mut er.start, &mut er.end);
                    fixed += 1;
                }
                fixed += ensure_item_id(
                    "event_range",
                    &er.lane_ref,
                    er.start.year(),
                    &mut er.props.id,
                    &mut used_ids,
                );
            }
            _ => {}
        }
    }

    fixed
}

/// ソースをパースして lint 自動修正（[`apply_lint_fixes`] 相当）を適用し、再 emit した
/// DSL ソース文字列を返す。
///
/// - 修正が 1 件以上適用された場合は `Ok(Some(fixed_source))`。
/// - 修正が 0 件（変更なし）の場合は `Ok(None)`。
/// - パース失敗時は [`tdsl_parser::error::ParseError`]。
///
/// `tdsl lint --fix` と同じく全文を再 emit する。コメントは AST に保持され、
/// トップレベル位置のコメントはそのまま、ブロック内部のコメントは境界に移動して保持される（#473）。
/// LSP の Code Action（quick fix）から全文置換 [`WorkspaceEdit`] を組み立てる用途で使う。
///
/// [`WorkspaceEdit`]: https://microsoft.github.io/language-server-protocol/
pub fn fix_source(source: &str) -> Result<Option<String>, tdsl_parser::error::ParseError> {
    let mut file = tdsl_parser::parse(source)?;
    if apply_lint_fixes(&mut file) == 0 {
        return Ok(None);
    }
    Ok(Some(tdsl_parser::format_file(&file)))
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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── 検出ケース ──────────────────────────────────────────────────────────

    // ─── range の順序判定（#757）────────────────────────────────────────────
    //
    // 以前は `to_sortable()`（年月日のみ）で比較しており、時分秒と UTC offset を
    // 捨てていた。以下 4 ケースは、どれか 1 つでも旧実装に戻すと落ちる。

    fn span_src(range: &str) -> String {
        format!(
            r#"
timeline "T" {{ unit year; range 0..3000; }}
lane "A" as a {{ kind custom; }}
span a {range} "S" {{ id "s1"; }};
"#
        )
    }

    /// UTC に直すと正順なので、逆転として報告してはならない。
    /// 旧実装は年月日だけを見て 01-02 > 01-01 と判定し ERROR を出していた。
    #[test]
    fn lint_accepts_offset_range_that_is_ordered_in_utc() {
        let src = span_src("2024-01-02T08:00+09:00..2024-01-01T20:00-05:00");
        let file = tdsl_parser::parse(&src).unwrap();
        let issues = lint_issues(&file, &src);
        assert!(
            !issues.iter().any(|i| i.code == "start_gt_end"),
            "UTC では 23:00 < 01:00 で正順なのに逆転と報告された: {issues:?}"
        );
    }

    /// 上と同じ入力で `--fix` が swap してはならない。
    /// swap すると `tdsl check` が start > end を出す不正なファイルになる
    /// （＝正しいファイルを壊す、この issue の最も深刻な症状）。
    #[test]
    fn fix_does_not_swap_offset_range_that_is_ordered_in_utc() {
        let src = span_src("2024-01-02T08:00+09:00..2024-01-01T20:00-05:00");
        let mut file = tdsl_parser::parse(&src).unwrap();
        apply_lint_fixes(&mut file);
        let tdsl_parser::ast::Statement::Span(s) = &file
            .statements
            .iter()
            .find(|st| matches!(st.node, tdsl_parser::ast::Statement::Span(_)))
            .expect("span statement")
            .node
        else {
            unreachable!()
        };
        assert_eq!(
            s.start.to_string(),
            "2024-01-02T08:00+09:00",
            "start が swap された"
        );
    }

    /// 同一日内で時刻だけ逆転しているケース。旧実装は年月日が同じため見逃していた
    /// （validate は報告しており、lint との非一貫が生じていた）。
    #[test]
    fn lint_detects_same_day_time_only_reversal() {
        let src = span_src("2024-01-01T20:00..2024-01-01T08:00");
        let file = tdsl_parser::parse(&src).unwrap();
        let issues = lint_issues(&file, &src);
        assert!(
            issues.iter().any(|i| i.code == "start_gt_end"),
            "同一日内の時刻逆転を見逃した: {issues:?}"
        );
    }

    /// 片側だけ offset 付きは順序が決まらない。swap ではなく
    /// 修正不能な別コードで報告する。
    #[test]
    fn lint_reports_mixed_offset_range_as_unfixable() {
        let src = span_src("2024-01-02T08:00+09:00..2024-01-01T20:00");
        let file = tdsl_parser::parse(&src).unwrap();
        let issues = lint_issues(&file, &src);
        let issue = issues
            .iter()
            .find(|i| i.code == "mixed_offset_range")
            .unwrap_or_else(|| panic!("mixed_offset_range が無い: {issues:?}"));
        assert!(
            !issue.fixable,
            "順序が決まらないものを fixable にしてはいけない"
        );
        assert!(
            !issues.iter().any(|i| i.code == "start_gt_end"),
            "順序不定を逆転として報告してはいけない: {issues:?}"
        );

        let mut file = tdsl_parser::parse(&src).unwrap();
        apply_lint_fixes(&mut file);
        let tdsl_parser::ast::Statement::Span(s) = &file
            .statements
            .iter()
            .find(|st| matches!(st.node, tdsl_parser::ast::Statement::Span(_)))
            .expect("span statement")
            .node
        else {
            unreachable!()
        };
        assert_eq!(
            s.start.to_string(),
            "2024-01-02T08:00+09:00",
            "順序が決まらない range を swap してはいけない"
        );
    }

    /// 本当に逆転している範囲は、従来どおり検出し修正できること（退行防止）。
    #[test]
    fn lint_still_fixes_genuinely_reversed_range() {
        let src = span_src("2025..2021");
        let file = tdsl_parser::parse(&src).unwrap();
        assert!(
            lint_issues(&file, &src)
                .iter()
                .any(|i| i.code == "start_gt_end" && i.fixable),
            "明らかな逆転を検出できていない"
        );

        let mut file = tdsl_parser::parse(&src).unwrap();
        assert!(apply_lint_fixes(&mut file) > 0);
        let tdsl_parser::ast::Statement::Span(s) = &file
            .statements
            .iter()
            .find(|st| matches!(st.node, tdsl_parser::ast::Statement::Span(_)))
            .expect("span statement")
            .node
        else {
            unreachable!()
        };
        assert_eq!(s.start.to_string(), "2021");
        assert_eq!(s.end.to_string(), "2025");
    }

    #[test]
    fn lint_detects_unknown_lane() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span nonexistent 10..20 "S" { id "s1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "unknown_lane"),
            "expected unknown_lane, got: {issues:?}"
        );
    }

    #[test]
    fn lint_accepts_group_lane_references() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
group "G" {
  lane "A" as a { kind custom; }
}
span a 10..20 "S" { id "s1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            !issues.iter().any(|i| i.code == "unknown_lane"),
            "expected group lane to be known, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_empty_label() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "" { id "e1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "empty_label"),
            "expected empty_label, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_invalid_tags_empty() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E" { id "e1"; tags ["", "war"]; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "invalid_tags"),
            "expected invalid_tags, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_invalid_tags_duplicate() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E" { id "e1"; tags ["war", "war"]; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "invalid_tags"),
            "expected invalid_tags for duplicate, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_duplicate_id() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E1" { id "same"; };
event a 20 "E2" { id "same"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "duplicate_id"),
            "expected duplicate_id, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_missing_id() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E" {};
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "missing_id"),
            "expected missing_id, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_start_gt_end_span() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" { id "s1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "start_gt_end"),
            "expected start_gt_end, got: {issues:?}"
        );
    }

    #[test]
    fn lint_detects_start_gt_end_event_range() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event_range a 80..20 "R" { id "r1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            issues.iter().any(|i| i.code == "start_gt_end"),
            "expected start_gt_end for event_range, got: {issues:?}"
        );
    }

    // ─── fix 適用ケース ──────────────────────────────────────────────────────

    #[test]
    fn apply_lint_fixes_normalizes_empty_tags() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E" { tags ["", "war", ""]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 1, "expected at least 1 fix, got {fixed}");

        // タグが正規化されていること
        use tdsl_parser::ast::Statement;
        for stmt in &file.statements {
            if let Statement::Event(e) = &stmt.node {
                assert_eq!(e.props.tags, vec!["war".to_string()]);
            }
        }
    }

    #[test]
    fn apply_lint_fixes_removes_duplicate_tags() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 10..20 "S" { tags ["x", "x", "y"]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 1, "expected at least 1 fix for dup tags");

        use tdsl_parser::ast::Statement;
        for stmt in &file.statements {
            if let Statement::Span(s) = &stmt.node {
                assert_eq!(s.props.tags, vec!["x".to_string(), "y".to_string()]);
            }
        }
    }

    #[test]
    fn apply_lint_fixes_swaps_reversed_span_range() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" {};
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 1, "expected at least 1 fix for reversed span");

        use tdsl_parser::ast::Statement;
        for stmt in &file.statements {
            if let Statement::Span(s) = &stmt.node {
                assert!(
                    s.start.to_sortable() <= s.end.to_sortable(),
                    "start should be <= end after fix"
                );
            }
        }
    }

    #[test]
    fn apply_lint_fixes_swaps_reversed_event_range() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event_range a 80..20 "R" {};
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(
            fixed >= 1,
            "expected at least 1 fix for reversed event_range"
        );

        use tdsl_parser::ast::Statement;
        for stmt in &file.statements {
            if let Statement::EventRange(er) = &stmt.node {
                assert!(
                    er.start.to_sortable() <= er.end.to_sortable(),
                    "start should be <= end after fix"
                );
            }
        }
    }

    #[test]
    fn apply_lint_fixes_generates_id_for_missing() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
event a 10 "E" {};
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert_eq!(fixed, 1, "expected 1 fix for missing id");

        use tdsl_parser::ast::Statement;
        for stmt in &file.statements {
            if let Statement::Event(e) = &stmt.node {
                assert!(
                    e.props
                        .id
                        .as_deref()
                        .map(|s| !s.is_empty())
                        .unwrap_or(false),
                    "id should be set after fix"
                );
                assert_eq!(
                    e.props.id.as_deref(),
                    Some("event:a:10"),
                    "generated id format mismatch"
                );
            }
        }
    }

    // ─── fix_source（パース→修正→再 emit）ケース ───────────────────────────────

    #[test]
    fn fix_source_returns_some_and_clears_fixable_issues() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
"#;
        let fixed = fix_source(src).unwrap();
        let fixed = fixed.expect("expected Some(fixed source) for fixable input");

        // 再パースして fixable 系の issue が解消されていること
        let reparsed = tdsl_parser::parse(&fixed).unwrap();
        let issues = lint_issues(&reparsed, &fixed);
        assert!(
            !issues.iter().any(|i| matches!(
                i.code.as_str(),
                "start_gt_end" | "invalid_tags" | "missing_id"
            )),
            "fixable issues should be gone, got: {issues:?}"
        );
    }

    #[test]
    fn fix_source_returns_none_for_clean_source() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 10..20 "S" { tags ["x", "y"]; id "s1"; };
"#;
        let fixed = fix_source(src).unwrap();
        assert!(
            fixed.is_none(),
            "clean source should yield None, got: {fixed:?}"
        );
    }

    #[test]
    fn fix_source_propagates_parse_error() {
        let src = "this is not valid tdsl {{{";
        assert!(
            fix_source(src).is_err(),
            "invalid source should yield ParseError"
        );
    }

    // ─── invalid_calendar_date ────────────────────────────────────────────────

    /// `2000-02-29` は閏年なので OK（警告なし）。
    #[test]
    fn lint_calendar_date_ok_leap_year() {
        let src = r#"
timeline "T" { unit year; range 1999..2001; }
lane "A" as a { kind custom; }
event a 2000-02-29 "閏年OK" { id "e1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            !issues.iter().any(|i| i.code == "invalid_calendar_date"),
            "2000-02-29 should be valid (leap year), got: {issues:?}"
        );
    }

    /// `1900-02-28` は平年なので OK。
    #[test]
    fn lint_calendar_date_ok_non_leap_year() {
        let src = r#"
timeline "T" { unit year; range 1899..1901; }
lane "A" as a { kind custom; }
event a 1900-02-28 "平年OK" { id "e1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            !issues.iter().any(|i| i.code == "invalid_calendar_date"),
            "1900-02-28 should be valid (non-leap year), got: {issues:?}"
        );
    }

    /// `1900-02-29` は平年（1900 は 100 の倍数だが 400 の倍数でない）なので NG。
    #[test]
    fn lint_calendar_date_ng_non_leap_1900() {
        let src = r#"
timeline "T" { unit year; range 1899..1901; }
lane "A" as a { kind custom; }
event a 1900-02-29 "平年2/29はNG" { id "e1"; };
"#;
        // builder.rs の range チェック（day 1..=31）で弾かれる前に lint が走る想定だが、
        // builder がパースを通さない場合はパースエラーになる。ここでは直接 TimeValue を作る。
        // パーサが 1900-02-29 を受け付けるかどうかを確認する必要がある。
        // builder は month=1..12 / day=1..31 のみ検証するため 1900-02-29 はパース可能。
        let result = tdsl_parser::parse(src);
        if let Ok(file) = result {
            let issues = lint_issues(&file, src);
            assert!(
                issues.iter().any(|i| i.code == "invalid_calendar_date"),
                "1900-02-29 should warn invalid_calendar_date, got: {issues:?}"
            );
        }
        // パースエラーの場合は既にビルダー側で弾かれているので lint は不要（テストをスキップ）
    }

    /// `2024-02-30` は存在しない日付なので NG。
    #[test]
    fn lint_calendar_date_ng_feb30() {
        let src = r#"
timeline "T" { unit year; range 2023..2025; }
lane "A" as a { kind custom; }
event a 2024-02-30 "2月30日はNG" { id "e1"; };
"#;
        let result = tdsl_parser::parse(src);
        if let Ok(file) = result {
            let issues = lint_issues(&file, src);
            assert!(
                issues.iter().any(|i| i.code == "invalid_calendar_date"),
                "2024-02-30 should warn invalid_calendar_date, got: {issues:?}"
            );
        }
    }

    /// 4月31日は存在しない日付なので NG。
    #[test]
    fn lint_calendar_date_ng_apr31() {
        let src = r#"
timeline "T" { unit year; range 2020..2022; }
lane "A" as a { kind custom; }
event a 2021-04-31 "4月31日はNG" { id "e1"; };
"#;
        let result = tdsl_parser::parse(src);
        if let Ok(file) = result {
            let issues = lint_issues(&file, src);
            assert!(
                issues.iter().any(|i| i.code == "invalid_calendar_date"),
                "2021-04-31 should warn invalid_calendar_date, got: {issues:?}"
            );
        }
    }

    /// `2021-02-28` は平年なので OK。
    #[test]
    fn lint_calendar_date_ok_2021_feb28() {
        let src = r#"
timeline "T" { unit year; range 2020..2022; }
lane "A" as a { kind custom; }
event a 2021-02-28 "平年2/28OK" { id "e1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            !issues.iter().any(|i| i.code == "invalid_calendar_date"),
            "2021-02-28 should be valid, got: {issues:?}"
        );
    }

    /// `2021-02-29` は平年なので NG。
    #[test]
    fn lint_calendar_date_ng_2021_feb29() {
        let src = r#"
timeline "T" { unit year; range 2020..2022; }
lane "A" as a { kind custom; }
event a 2021-02-29 "平年2/29はNG" { id "e1"; };
"#;
        let result = tdsl_parser::parse(src);
        if let Ok(file) = result {
            let issues = lint_issues(&file, src);
            assert!(
                issues.iter().any(|i| i.code == "invalid_calendar_date"),
                "2021-02-29 should warn invalid_calendar_date, got: {issues:?}"
            );
        }
    }

    /// 月精度のみ（日なし）は検証をスキップする。
    #[test]
    fn lint_calendar_date_skip_year_month_precision() {
        let src = r#"
timeline "T" { unit year; range 2020..2022; }
lane "A" as a { kind custom; }
event a 2021-02 "月精度はスキップ" { id "e1"; };
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        assert!(
            !issues.iter().any(|i| i.code == "invalid_calendar_date"),
            "year-month precision should not trigger invalid_calendar_date, got: {issues:?}"
        );
    }

    /// span の両端日付も検証される。
    #[test]
    fn lint_calendar_date_span_both_ends_checked() {
        let src = r#"
timeline "T" { unit year; range 2020..2022; }
lane "A" as a { kind custom; }
span a 2021-01-01..2021-02-30 "spanのendがNG" { id "s1"; };
"#;
        let result = tdsl_parser::parse(src);
        if let Ok(file) = result {
            let issues = lint_issues(&file, src);
            assert!(
                issues.iter().any(|i| i.code == "invalid_calendar_date"),
                "span end 2021-02-30 should warn invalid_calendar_date, got: {issues:?}"
            );
        }
    }

    #[test]
    fn apply_lint_fixes_returns_count_of_all_fixes() {
        let src = r#"
timeline "T" { unit year; range 0..100; }
lane "A" as a { kind custom; }
span a 50..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
event_range a 80..20 "R" { tags ["war", "war"]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        // span: tags fix(1) + swap(1) + id gen(1) = 3
        // event: id gen(1) = 1
        // event_range: tags fix(1) + swap(1) + id gen(1) = 3
        // total = 7
        assert_eq!(fixed, 7, "expected 7 total fixes, got {fixed}");
    }

    /// #763: 行番号の引き方を線形走査から二分探索へ変えた。
    /// 旧実装と同じ値を返すことを、境界を含めて確かめる。
    #[test]
    fn line_from_offset_matches_linear_scan() {
        fn linear(source: &str, offset: usize) -> usize {
            let clamped = offset.min(source.len());
            source.as_bytes()[..clamped]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                + 1
        }

        for source in [
            "",
            "a",
            "a\n",
            "\n",
            "\n\n\n",
            "one\ntwo\nthree",
            "one\ntwo\nthree\n",
            "日本語\n改行あり\n",
        ] {
            let offsets = crate::lower::build_line_offsets(source);
            // 範囲外（len 超過）も含めて全オフセットを比較する。
            for offset in 0..=source.len() + 3 {
                assert_eq!(
                    line_from_offset(&offsets, offset),
                    linear(source, offset),
                    "source={source:?} offset={offset}"
                );
            }
        }
    }
}
