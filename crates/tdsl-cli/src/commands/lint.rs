use std::fmt::Write;

use serde::Serialize;
use tdsl_core::lint::{LintIssue, LintSeverity, apply_lint_fixes, lint_issues};

use crate::LintOutputFormat;

#[derive(Debug, Clone, Serialize)]
struct LintReportOutput {
    file: String,
    fix_applied: usize,
    issue_count: usize,
    ok: bool,
    issues: Vec<LintIssue>,
}

pub(crate) fn cmd_lint(
    input: &std::path::Path,
    fix: bool,
    format: LintOutputFormat,
) -> Result<(), String> {
    let source = super::read_source(input)?;
    let mut file = tdsl_parser::parse(&source).map_err(|e| e.to_string())?;

    let mut fix_applied = 0usize;
    let mut lint_source = source.clone();
    if fix {
        fix_applied = apply_lint_fixes(&mut file);
        let rewritten = render_tdsl_file(&file);
        if rewritten != source {
            std::fs::write(input, &rewritten)
                .map_err(|e| format!("Failed to write {}: {e}", input.display()))?;
            lint_source = rewritten;
        }
    }

    let issues = lint_issues(&file, &lint_source);
    match format {
        LintOutputFormat::Text => {
            if fix {
                println!("Applied {fix_applied} fix(es) to {}", input.display());
            }
            if issues.is_empty() {
                println!("OK: no lint issues");
                return Ok(());
            }
            println!("Found {} issue(s):", issues.len());
            for issue in &issues {
                println!(
                    "- {severity} [{code}] line {line}: {message}{fixable}",
                    severity = match issue.severity {
                        LintSeverity::Error => "ERROR",
                        LintSeverity::Warning => "WARN",
                    },
                    code = issue.code,
                    line = issue.line,
                    message = issue.message,
                    fixable = if issue.fixable { " (fixable)" } else { "" }
                );
            }
        }
        LintOutputFormat::Json => {
            let report = LintReportOutput {
                file: input.display().to_string(),
                fix_applied,
                issue_count: issues.len(),
                ok: issues.is_empty(),
                issues,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
            );
        }
    }

    Ok(())
}

pub(crate) fn render_tdsl_file(file: &tdsl_parser::ast::File) -> String {
    use tdsl_parser::ast::Statement;

    let mut out = String::new();
    for (idx, stmt) in file.statements.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
            out.push('\n');
        }
        match &stmt.node {
            Statement::Timeline(t) => {
                writeln!(
                    out,
                    r#"timeline "{}" {{"#,
                    super::escape_tdsl_string(&t.name)
                )
                .unwrap();
                if let Some(title) = &t.title {
                    writeln!(out, r#"    title "{}";"#, super::escape_tdsl_string(title)).unwrap();
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
                write!(out, r#"lane "{}""#, super::escape_tdsl_string(&l.label)).unwrap();
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
                    super::escape_tdsl_string(&s.label),
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
                    super::escape_tdsl_string(&e.label),
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
                    super::escape_tdsl_string(&er.label),
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
                            write!(out, r#"    query "{}""#, super::escape_tdsl_string(query))
                                .unwrap();
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
                    match prop {
                        tdsl_parser::ast::MapProp::Lane(lane) => {
                            writeln!(out, "    lane {lane};").unwrap();
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
                                .map(|t| format!(r#""{}""#, super::escape_tdsl_string(t)))
                                .collect::<Vec<_>>()
                                .join(", ");
                            writeln!(out, "    tags [{joined}];").unwrap();
                        }
                        tdsl_parser::ast::MapProp::Filter(expr) => {
                            writeln!(out, "    filter {};", render_filter_expr(expr)).unwrap();
                        }
                    }
                }
                write!(out, "}}").unwrap();
            }
            Statement::Template(t) => {
                let target = match t.target_type {
                    tdsl_parser::ast::MapTargetType::Span => "span",
                    tdsl_parser::ast::MapTargetType::Event => "event",
                    tdsl_parser::ast::MapTargetType::EventRange => "event_range",
                };
                write!(out, r#"template "{}""#, super::escape_tdsl_string(&t.name)).unwrap();
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
    use std::fmt::Write as _;
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
                .map(|t| format!(r#""{}""#, super::escape_tdsl_string(t)))
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
            .map(|t| format!(r#""{}""#, super::escape_tdsl_string(t)))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("tags [{joined}];"));
    }
    if let Some(source) = &props.source {
        parts.push(format!("source {}:{};", source.prefix, source.qid));
    }
    if let Some(id) = &props.id {
        parts.push(format!(r#"id "{}";"#, super::escape_tdsl_string(id)));
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

    #[test]
    fn lint_issues_detects_initial_rule_set() {
        let src = r#"
timeline "Lint" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span b 20..10 "" { tags ["x", "", "x"]; id "dup"; };
event a 30 "E" { id "dup"; };
event a 40 "No ID" {};
"#;
        let file = tdsl_parser::parse(src).unwrap();
        let issues = lint_issues(&file, src);
        let codes: std::collections::HashSet<String> =
            issues.iter().map(|i| i.code.clone()).collect();
        assert!(codes.contains("unknown_lane"));
        assert!(codes.contains("duplicate_id"));
        assert!(codes.contains("start_gt_end"));
        assert!(codes.contains("empty_label"));
        assert!(codes.contains("invalid_tags"));
        assert!(codes.contains("missing_id"));
    }

    #[test]
    fn apply_lint_fixes_normalizes_tags_swaps_ranges_and_generates_ids() {
        let src = r#"
timeline "Fix" { unit year; range 0..100; }
lane "A" as a { kind custom; order 10; }
span a 20..10 "S" { tags ["x", "", "x"]; };
event a 30 "E" {};
event_range a 50..40 "R" { tags ["war", "war"]; };
"#;
        let mut file = tdsl_parser::parse(src).unwrap();
        let fixed = apply_lint_fixes(&mut file);
        assert!(fixed >= 5);

        let rendered = render_tdsl_file(&file);
        let reparsed = tdsl_parser::parse(&rendered).unwrap();
        let issues = lint_issues(&reparsed, &rendered);
        assert!(!issues.iter().any(|i| i.code == "start_gt_end"
            || i.code == "invalid_tags"
            || i.code == "missing_id"));

        let ir = tdsl_core::lower::lower_static(&reparsed).unwrap();
        assert_eq!(ir.items.len(), 3);
    }
}
