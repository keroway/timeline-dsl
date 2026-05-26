use std::fmt::Write;

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

pub fn lint_issues(file: &tdsl_parser::ast::File, source: &str) -> Vec<LintIssue> {
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
                if s.start.to_sortable() > s.end.to_sortable() {
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
                if er.start.to_sortable() > er.end.to_sortable() {
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
                if s.start.to_sortable() > s.end.to_sortable() {
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
                if er.start.to_sortable() > er.end.to_sortable() {
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

/// DSL 文字列リテラル中でエスケープが必要な文字をエスケープする。
pub fn escape_tdsl_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

/// AST `File` を `.tdsl` テキストに再シリアライズする。
///
/// `apply_lint_fixes` 後のファイルをテキストとして書き戻す用途に使う。
/// `color_map` フィールドは現バージョンでは出力しない（`timeline` ブロック内で
/// サポートされているが、lint fix の対象外のため省略）。
pub fn render_tdsl_file(file: &tdsl_parser::ast::File) -> String {
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
                    match policy {
                        tdsl_parser::ast::ReimportPolicy::MergeBySource => {
                            writeln!(out, "    policy merge_by_source;").unwrap();
                        }
                        tdsl_parser::ast::ReimportPolicy::OverwriteImported => {
                            writeln!(out, "    policy overwrite_imported;").unwrap();
                        }
                        tdsl_parser::ast::ReimportPolicy::KeepManual => {
                            writeln!(out, "    policy keep_manual;").unwrap();
                        }
                        tdsl_parser::ast::ReimportPolicy::FieldPriority(config) => {
                            use tdsl_parser::ast::FieldStrategy;
                            let label = match config.label {
                                FieldStrategy::Manual => "manual",
                                FieldStrategy::Wikidata => "wikidata",
                                FieldStrategy::Merge => "merge",
                            };
                            let time = match config.time {
                                FieldStrategy::Manual => "manual",
                                FieldStrategy::Wikidata => "wikidata",
                                FieldStrategy::Merge => "merge",
                            };
                            let tags = match config.tags {
                                FieldStrategy::Manual => "manual",
                                FieldStrategy::Wikidata => "wikidata",
                                FieldStrategy::Merge => "merge",
                            };
                            writeln!(out, "    policy field_priority {{").unwrap();
                            writeln!(out, "        label: {label};").unwrap();
                            writeln!(out, "        time: {time};").unwrap();
                            writeln!(out, "        tags: {tags};").unwrap();
                            writeln!(out, "    }}").unwrap();
                        }
                    }
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
                    render_map_prop(&mut out, prop);
                }
                write!(out, "}}").unwrap();
            }
            Statement::Template(t) => {
                let target = match t.target_type {
                    tdsl_parser::ast::MapTargetType::Span => "span",
                    tdsl_parser::ast::MapTargetType::Event => "event",
                    tdsl_parser::ast::MapTargetType::EventRange => "event_range",
                };
                write!(out, r#"template "{}""#, escape_tdsl_string(&t.name)).unwrap();
                if let Some(alias) = &t.alias {
                    write!(out, " as {alias}").unwrap();
                }
                writeln!(out, "\n    to {target} {{").unwrap();
                for prop in &t.props {
                    render_map_prop(&mut out, prop);
                }
                write!(out, "}}").unwrap();
            }
            Statement::Apply(a) => {
                write!(out, "apply {} to {} {{", a.template_alias, a.import_alias).unwrap();
                if a.overrides.is_empty() {
                    write!(out, "}}").unwrap();
                } else {
                    writeln!(out).unwrap();
                    for prop in &a.overrides {
                        render_map_prop(&mut out, prop);
                    }
                    write!(out, "}}").unwrap();
                }
            }
        }
    }
    out.push('\n');
    out
}

fn render_map_prop(out: &mut String, prop: &tdsl_parser::ast::MapProp) {
    match prop {
        tdsl_parser::ast::MapProp::Lane(id) => {
            writeln!(out, "    lane {id};").unwrap();
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
        tdsl_parser::ast::MapProp::Filter(expr) => {
            writeln!(out, "    filter {};", render_filter_expr(expr)).unwrap();
        }
    }
}

fn render_filter_expr(expr: &tdsl_parser::ast::FilterExpr) -> String {
    render_filter_or(expr)
}

fn render_filter_or(expr: &tdsl_parser::ast::FilterExpr) -> String {
    match expr {
        tdsl_parser::ast::FilterExpr::Or(a, b) => {
            format!("{} || {}", render_filter_or(a), render_filter_and(b))
        }
        _ => render_filter_and(expr),
    }
}

fn render_filter_and(expr: &tdsl_parser::ast::FilterExpr) -> String {
    match expr {
        tdsl_parser::ast::FilterExpr::And(a, b) => {
            format!("{} && {}", render_filter_and(a), render_filter_unary(b))
        }
        _ => render_filter_unary(expr),
    }
}

fn render_filter_unary(expr: &tdsl_parser::ast::FilterExpr) -> String {
    match expr {
        tdsl_parser::ast::FilterExpr::Not(inner) => format!("!{}", render_filter_atom(inner)),
        _ => render_filter_atom(expr),
    }
}

fn render_filter_atom(expr: &tdsl_parser::ast::FilterExpr) -> String {
    match expr {
        tdsl_parser::ast::FilterExpr::Compare { lhs, op, rhs } => {
            format!(
                "{} {} {}",
                render_filter_operand(lhs),
                render_compare_op(*op),
                render_filter_operand(rhs)
            )
        }
        other => format!("({})", render_filter_expr(other)),
    }
}

fn render_filter_operand(op: &tdsl_parser::ast::FilterOperand) -> String {
    match op {
        tdsl_parser::ast::FilterOperand::Claim(c) => render_claim_expr(c),
        tdsl_parser::ast::FilterOperand::Int(n) => n.to_string(),
        tdsl_parser::ast::FilterOperand::Null => "null".to_string(),
    }
}

fn render_compare_op(op: tdsl_parser::ast::CompareOp) -> &'static str {
    match op {
        tdsl_parser::ast::CompareOp::Eq => "==",
        tdsl_parser::ast::CompareOp::NotEq => "!=",
        tdsl_parser::ast::CompareOp::Lt => "<",
        tdsl_parser::ast::CompareOp::Le => "<=",
        tdsl_parser::ast::CompareOp::Gt => ">",
        tdsl_parser::ast::CompareOp::Ge => ">=",
    }
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
    expr.fallbacks
        .iter()
        .map(render_claim_expr)
        .collect::<Vec<_>>()
        .join(" ?? ")
}

fn render_claim_expr(expr: &tdsl_parser::ast::ClaimExpr) -> String {
    let base = if let Some(accessor) = &expr.accessor {
        format!("claim({}).{}", expr.claim.property, accessor)
    } else {
        format!("claim({})", expr.claim.property)
    };
    match expr.offset {
        Some(off) if off >= 0 => format!("{base} +{off}"),
        Some(off) => format!("{base} {off}"),
        None => base,
    }
}

fn render_label_expr(expr: &tdsl_parser::ast::LabelExpr) -> String {
    expr.fallbacks
        .iter()
        .map(|l| format!("label@{}", l.lang))
        .collect::<Vec<_>>()
        .join(" ?? ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── 検出ケース ──────────────────────────────────────────────────────────

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
}
