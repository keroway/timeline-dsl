//! AST → DSL ソース再 emit（フォーマッタ）。
//!
//! `parse` で得た [`crate::ast::File`] を 2 スペースインデント・ブロック間空行 1 行で
//! 整形して文字列として返す。
//!
//! コメントは [`File::comments`] に保持されており、byte span を根拠に再 emit される（#473）。
//! - トップレベル文間・先頭・末尾の独立行コメントはその位置に leading として保持される。
//! - 文と同一行の trailing コメントはその文の行末に保持される。
//! - ブロック内部のコメントは AST には保持されるが、フォーマッタはフィールド順に再
//!   emit するため、ブロック境界（直前文の末尾または次文の直前）に移動される（内容は欠落
//!   しない）。いずれのケースも再パース・再整形で冪等（idempotent）。
//!
//! lowering は [`File::comments`] を一切参照しないため、コメント保持は IR に影響しない。
//!
//! 公開 API は [`format_source`]（ソース文字列を整形）と [`format_file`]（AST を直接整形）。

use std::fmt::Write;

use crate::ast::{
    ApplyBlock, ClaimExpr, Comment, CompareOp, EventDecl, EventRangeDecl, FieldPriorityConfig,
    FieldStrategy, File, FilterExpr, FilterOperand, GroupDecl, ImportBlock, ImportItem, ItemProps,
    LabelExpr, LaneDecl, MapBlock, MapExpr, MapFallback, MapProp, MapTargetType, ReimportPolicy,
    SourceRef, SpanDecl, Spanned, Statement, StringMatchOp, TemplateBlock, TimelineBlock,
};
use crate::error::ParseError;

const INDENT: &str = "  ";

/// DSL ソースを整形して返す。パース失敗時は [`ParseError`] を返却する。
pub fn format_source(source: &str) -> Result<String, ParseError> {
    let file = crate::parse(source)?;
    Ok(format_file(&file))
}

/// AST（[`File`]）を直接整形して DSL ソース文字列を返す。
///
/// `lint --fix` 適用後の AST など、すでにパース済みの AST を再 emit したいときに使う。
/// 整形ルールは [`format_source`] と同一（2 スペースインデント・ブロック間空行 1 行）。
pub fn format_file(file: &File) -> String {
    let mut out = String::new();

    // コメントを出現順（byte span 順）に並べて順に消費する。
    let mut comments: Vec<&Spanned<Comment>> = file.comments.iter().collect();
    comments.sort_by_key(|c| c.span.start);
    let mut ci = 0usize;

    for (i, stmt) in file.statements.iter().enumerate() {
        // この文より前に出現する未消費コメントを振り分ける。
        // own_line=true → この文の leading、own_line=false → 直前文の trailing。
        let mut leading: Vec<&Comment> = Vec::new();
        while ci < comments.len() && comments[ci].span.start < stmt.span.start {
            let c = &comments[ci].node;
            if c.own_line || i == 0 {
                leading.push(c);
            } else {
                append_trailing(&mut out, c);
            }
            ci += 1;
        }

        // 文間の空行（先頭を除く）。leading コメントがあればその上に空行が入る。
        if i > 0 {
            out.push('\n');
        }
        for c in &leading {
            push_comment_line(&mut out, c);
        }
        write_statement(&mut out, &stmt.node);
    }

    // 最後の文より後ろのコメント（末尾コメント）。
    let mut dangling_started = false;
    while ci < comments.len() {
        let c = &comments[ci].node;
        if !c.own_line && !dangling_started && !out.is_empty() {
            append_trailing(&mut out, c);
        } else {
            if !dangling_started && !file.statements.is_empty() {
                out.push('\n');
            }
            push_comment_line(&mut out, c);
            dangling_started = true;
        }
        ci += 1;
    }

    out
}

/// コメントを独立行として出力する（末尾に改行を付ける）。
fn push_comment_line(out: &mut String, c: &Comment) {
    out.push_str(&c.text);
    out.push('\n');
}

/// 直前に出力した行の末尾に trailing コメントを追記する。
fn append_trailing(out: &mut String, c: &Comment) {
    // 直前文の末尾改行を一旦除去して同一行に連結する。
    if out.ends_with('\n') {
        out.pop();
    }
    out.push(' ');
    out.push_str(&c.text);
    out.push('\n');
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn write_statement(out: &mut String, stmt: &Statement) {
    match stmt {
        Statement::Timeline(b) => write_timeline(out, b),
        Statement::Lane(b) => write_lane(out, b),
        Statement::Group(b) => write_group(out, b),
        Statement::Span(b) => write_span(out, b),
        Statement::Event(b) => write_event(out, b),
        Statement::EventRange(b) => write_event_range(out, b),
        Statement::Import(b) => write_import(out, b),
        Statement::Map(b) => write_map(out, b),
        Statement::Template(b) => write_template(out, b),
        Statement::Apply(b) => write_apply(out, b),
    }
}

fn write_group(out: &mut String, b: &GroupDecl) {
    writeln!(out, r#"group "{}" {{"#, escape_string(&b.label)).unwrap();
    for lane in &b.lanes {
        // インデントして lane を emit する
        let mut lane_out = String::new();
        write_lane(&mut lane_out, lane);
        for line in lane_out.lines() {
            writeln!(out, "{INDENT}{line}").unwrap();
        }
    }
    writeln!(out, "}}").unwrap();
}

// ─── timeline ────────────────────────────────────────────────────────────────

fn write_timeline(out: &mut String, b: &TimelineBlock) {
    writeln!(out, r#"timeline "{}" {{"#, escape_string(&b.name)).unwrap();
    if let Some(title) = &b.title {
        writeln!(out, r#"{INDENT}title "{}";"#, escape_string(title)).unwrap();
    }
    if let Some(unit) = &b.unit {
        writeln!(out, "{INDENT}unit {unit};").unwrap();
    }
    if let Some(range) = &b.range {
        writeln!(out, "{INDENT}range {}..{};", range.start, range.end).unwrap();
    }
    if let Some(cal) = &b.calendar {
        writeln!(out, "{INDENT}calendar {cal};").unwrap();
    }
    if !b.color_map.is_empty() {
        writeln!(out, "{INDENT}color_map {{").unwrap();
        for (k, v) in &b.color_map {
            writeln!(out, r#"{INDENT}{INDENT}{k}: "{}";"#, escape_string(v)).unwrap();
        }
        writeln!(out, "{INDENT}}}").unwrap();
    }
    writeln!(out, "}}").unwrap();
}

// ─── lane ────────────────────────────────────────────────────────────────────

fn write_lane(out: &mut String, b: &LaneDecl) {
    write!(out, r#"lane "{}""#, escape_string(&b.label)).unwrap();
    if let Some(alias) = &b.alias {
        write!(out, " as {alias}").unwrap();
    }
    let has_body = b.kind.is_some() || b.order.is_some();
    if !has_body {
        writeln!(out, " {{}}").unwrap();
        return;
    }
    writeln!(out, " {{").unwrap();
    if let Some(kind) = &b.kind {
        writeln!(out, "{INDENT}kind {kind};").unwrap();
    }
    if let Some(order) = b.order {
        writeln!(out, "{INDENT}order {order};").unwrap();
    }
    writeln!(out, "}}").unwrap();
}

// ─── span / event / event_range ──────────────────────────────────────────────

fn write_span(out: &mut String, b: &SpanDecl) {
    write!(
        out,
        r#"span {} {}..{} "{}" "#,
        b.lane_ref,
        b.start,
        b.end,
        escape_string(&b.label)
    )
    .unwrap();
    write_item_props(out, &b.props);
    writeln!(out, ";").unwrap();
}

fn write_event(out: &mut String, b: &EventDecl) {
    write!(
        out,
        r#"event {} {} "{}" "#,
        b.lane_ref,
        b.time,
        escape_string(&b.label)
    )
    .unwrap();
    write_item_props(out, &b.props);
    writeln!(out, ";").unwrap();
}

fn write_event_range(out: &mut String, b: &EventRangeDecl) {
    write!(
        out,
        r#"event_range {} {}..{} "{}" "#,
        b.lane_ref,
        b.start,
        b.end,
        escape_string(&b.label)
    )
    .unwrap();
    write_item_props(out, &b.props);
    writeln!(out, ";").unwrap();
}

fn item_has_body(p: &ItemProps) -> bool {
    !p.tags.is_empty() || p.source.is_some() || p.id.is_some() || p.origin.is_some()
}

fn write_item_props(out: &mut String, p: &ItemProps) {
    if !item_has_body(p) {
        out.push_str("{}");
        return;
    }
    writeln!(out, "{{").unwrap();
    if !p.tags.is_empty() {
        let joined = p
            .tags
            .iter()
            .map(|t| format!(r#""{}""#, escape_string(t)))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out, "{INDENT}tags [{joined}];").unwrap();
    }
    if let Some(s) = &p.source {
        writeln!(out, "{INDENT}source {};", format_source_ref(s)).unwrap();
    }
    if let Some(id) = &p.id {
        writeln!(out, r#"{INDENT}id "{}";"#, escape_string(id)).unwrap();
    }
    if let Some(origin) = &p.origin {
        writeln!(out, "{INDENT}origin {origin};").unwrap();
    }
    write!(out, "}}").unwrap();
}

fn format_source_ref(s: &SourceRef) -> String {
    format!("{}:{}", s.prefix, s.qid)
}

// ─── import ──────────────────────────────────────────────────────────────────

fn write_import(out: &mut String, b: &ImportBlock) {
    write!(out, "import {}", b.source_type).unwrap();
    if let Some(alias) = &b.alias {
        write!(out, " as {alias}").unwrap();
    }
    let has_body = !b.items.is_empty() || b.policy.is_some();
    if !has_body {
        writeln!(out, " {{}}").unwrap();
        return;
    }
    writeln!(out, " {{").unwrap();
    for item in &b.items {
        match item {
            ImportItem::Entity { qid, alias } => {
                write!(out, "{INDENT}entity {qid}").unwrap();
                if let Some(a) = alias {
                    write!(out, " as {a}").unwrap();
                }
                writeln!(out, ";").unwrap();
            }
            ImportItem::Query { query, alias } => {
                write!(out, r#"{INDENT}query "{}""#, escape_string(query)).unwrap();
                if let Some(a) = alias {
                    write!(out, " as {a}").unwrap();
                }
                writeln!(out, ";").unwrap();
            }
        }
    }
    if let Some(policy) = &b.policy {
        write_reimport_policy(out, policy);
    }
    writeln!(out, "}}").unwrap();
}

fn write_reimport_policy(out: &mut String, p: &ReimportPolicy) {
    match p {
        ReimportPolicy::MergeBySource => writeln!(out, "{INDENT}policy merge_by_source;").unwrap(),
        ReimportPolicy::OverwriteImported => {
            writeln!(out, "{INDENT}policy overwrite_imported;").unwrap();
        }
        ReimportPolicy::KeepManual => writeln!(out, "{INDENT}policy keep_manual;").unwrap(),
        ReimportPolicy::FieldPriority(cfg) => write_field_priority(out, cfg),
    }
}

fn write_field_priority(out: &mut String, cfg: &FieldPriorityConfig) {
    writeln!(out, "{INDENT}policy field_priority {{").unwrap();
    writeln!(
        out,
        "{INDENT}{INDENT}label: {};",
        field_strategy_str(cfg.label)
    )
    .unwrap();
    writeln!(
        out,
        "{INDENT}{INDENT}time: {};",
        field_strategy_str(cfg.time)
    )
    .unwrap();
    writeln!(
        out,
        "{INDENT}{INDENT}tags: {};",
        field_strategy_str(cfg.tags)
    )
    .unwrap();
    writeln!(out, "{INDENT}}}").unwrap();
}

fn field_strategy_str(s: FieldStrategy) -> &'static str {
    match s {
        FieldStrategy::Manual => "manual",
        FieldStrategy::Wikidata => "wikidata",
        FieldStrategy::Merge => "merge",
    }
}

// ─── map / template / apply ──────────────────────────────────────────────────

fn write_map(out: &mut String, b: &MapBlock) {
    let tt = target_type_str(b.target_type);
    if b.props.is_empty() {
        writeln!(out, "map {} to {tt} {{}}", b.source_ref).unwrap();
        return;
    }
    writeln!(out, "map {} to {tt} {{", b.source_ref).unwrap();
    write_map_props(out, &b.props);
    writeln!(out, "}}").unwrap();
}

fn write_template(out: &mut String, b: &TemplateBlock) {
    write!(out, r#"template "{}""#, escape_string(&b.name)).unwrap();
    if let Some(alias) = &b.alias {
        write!(out, " as {alias}").unwrap();
    }
    let tt = target_type_str(b.target_type);
    if b.props.is_empty() {
        writeln!(out, " to {tt} {{}}").unwrap();
        return;
    }
    writeln!(out, " to {tt} {{").unwrap();
    write_map_props(out, &b.props);
    writeln!(out, "}}").unwrap();
}

fn write_apply(out: &mut String, b: &ApplyBlock) {
    if b.overrides.is_empty() {
        writeln!(out, "apply {} to {} {{}}", b.template_alias, b.import_alias).unwrap();
        return;
    }
    writeln!(out, "apply {} to {} {{", b.template_alias, b.import_alias).unwrap();
    write_map_props(out, &b.overrides);
    writeln!(out, "}}").unwrap();
}

fn target_type_str(t: MapTargetType) -> &'static str {
    match t {
        MapTargetType::Span => "span",
        MapTargetType::Event => "event",
        MapTargetType::EventRange => "event_range",
    }
}

fn write_map_props(out: &mut String, props: &[MapProp]) {
    for p in props {
        match p {
            MapProp::Lane(id) => writeln!(out, "{INDENT}lane {id};").unwrap(),
            MapProp::Start(e) => writeln!(out, "{INDENT}start {};", format_map_expr(e)).unwrap(),
            MapProp::End(e) => writeln!(out, "{INDENT}end {};", format_map_expr(e)).unwrap(),
            MapProp::Time(e) => writeln!(out, "{INDENT}time {};", format_map_expr(e)).unwrap(),
            MapProp::Label(l) => writeln!(out, "{INDENT}label {};", format_label_expr(l)).unwrap(),
            MapProp::Tags(tags) => {
                let joined = tags
                    .iter()
                    .map(|t| format!(r#""{}""#, escape_string(t)))
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(out, "{INDENT}tags [{joined}];").unwrap();
            }
            MapProp::Filter(f) => {
                writeln!(out, "{INDENT}filter {};", format_filter_expr(f)).unwrap();
            }
            MapProp::Expand(call) => {
                writeln!(out, "{INDENT}expand claim({});", call.property).unwrap();
            }
        }
    }
}

fn format_claim_expr(c: &ClaimExpr) -> String {
    // Build base: "claim(P)" or "claim(P).qualifier(Q)"
    let base = if let Some(qual) = &c.qualifier {
        format!("claim({}).qualifier({qual})", c.claim.property)
    } else {
        format!("claim({})", c.claim.property)
    };
    // Append accessor
    let base = match &c.accessor {
        Some(acc) => format!("{base}.{acc}"),
        None => base,
    };
    // Append offset
    match c.offset {
        Some(off) if off >= 0 => format!("{base} +{off}"),
        Some(off) => format!("{base} {off}"),
        None => base,
    }
}

fn format_map_expr(e: &MapExpr) -> String {
    e.fallbacks
        .iter()
        .map(|fb| match fb {
            MapFallback::Claim(c) => format_claim_expr(c),
            MapFallback::Literal(n) => n.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ?? ")
}

fn format_label_expr(l: &LabelExpr) -> String {
    l.fallbacks
        .iter()
        .map(|r| format!("label@{}", r.lang))
        .collect::<Vec<_>>()
        .join(" ?? ")
}

// filter_expr の出力は parser の左結合パース（builder.rs L441–456）と整合させるため、
// And/Or をフラット展開する。Or の子に And が現れる場合は括弧不要、And の子に Or が
// 現れる場合のみ括弧で囲んで結合方向を保持する。
fn format_filter_expr(f: &FilterExpr) -> String {
    match f {
        FilterExpr::Or(a, b) => format!("{} || {}", format_filter_expr(a), format_filter_expr(b)),
        FilterExpr::And(a, b) => format!(
            "{} && {}",
            paren_if_or(format_filter_expr(a), a),
            paren_if_or(format_filter_expr(b), b)
        ),
        FilterExpr::Not(inner) => format!("!({})", format_filter_expr(inner)),
        FilterExpr::Compare { lhs, op, rhs } => format!(
            "{} {} {}",
            format_filter_operand(lhs),
            compare_op_str(*op),
            format_filter_operand(rhs)
        ),
        FilterExpr::StringMatch { lhs, op, rhs } => format!(
            "label@{} {} \"{}\"",
            lhs.lang,
            match op {
                StringMatchOp::Contains => "contains",
                StringMatchOp::StartsWith => "startswith",
            },
            rhs.replace('\\', "\\\\").replace('"', "\\\"")
        ),
    }
}

fn paren_if_or(rendered: String, expr: &FilterExpr) -> String {
    if matches!(expr, FilterExpr::Or(_, _)) {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn format_filter_operand(op: &FilterOperand) -> String {
    match op {
        FilterOperand::Claim(c) => format_claim_expr(c),
        FilterOperand::Int(i) => i.to_string(),
        FilterOperand::Null => "null".to_string(),
    }
}

fn compare_op_str(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "==",
        CompareOp::NotEq => "!=",
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn fmt(src: &str) -> String {
        format_source(src).expect("format succeeded")
    }

    #[test]
    fn format_basic_timeline() {
        let src =
            r#"timeline "T"{title "T";unit year;range 1900..2000;calendar proleptic_gregorian;}"#;
        let out = fmt(src);
        assert_eq!(
            out,
            "timeline \"T\" {\n  title \"T\";\n  unit year;\n  range 1900..2000;\n  calendar proleptic_gregorian;\n}\n"
        );
    }

    #[test]
    fn format_lane_with_alias_and_props() {
        let src = r#"lane "漢" as han { kind dynasty; order 10; }"#;
        let out = fmt(src);
        assert_eq!(
            out,
            "lane \"漢\" as han {\n  kind dynasty;\n  order 10;\n}\n"
        );
    }

    #[test]
    fn format_lane_empty_body() {
        let src = r#"lane "Simple" {}"#;
        let out = fmt(src);
        assert_eq!(out, "lane \"Simple\" {}\n");
    }

    #[test]
    fn format_static_items() {
        let src = r#"
            span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
            event han -209 "陳勝・呉広の乱" {};
            event_range han 184..204 "黄巾の乱" { tags ["war"]; };
        "#;
        let out = fmt(src);
        // span のフォーマット: ブロック内に tags / source / id が改行で並ぶ
        assert!(out.contains("span han -206..220 \"漢\" {\n  tags [\"dynasty\"];\n  source wd:Q7209;\n  id \"span:han\";\n};"));
        assert!(out.contains("event han -209 \"陳勝・呉広の乱\" {};"));
        assert!(out.contains("event_range han 184..204 \"黄巾の乱\" {\n  tags [\"war\"];\n};"));
    }

    #[test]
    fn format_time_value_precisions() {
        let src = r#"
            event han 1969-07-20 "moon" {};
            event han 1969-07 "month" {};
            event han -206 "year" {};
        "#;
        let out = fmt(src);
        assert!(out.contains("event han 1969-07-20 \"moon\" {};"));
        assert!(out.contains("event han 1969-07 \"month\" {};"));
        assert!(out.contains("event han -206 \"year\" {};"));
    }

    #[test]
    fn format_import_and_policy() {
        let src = r#"
            import wikidata as wd {
                entity Q7209 as han_dynasty;
                query "SELECT ?item WHERE { ?item wdt:P31 wd:Q28171280 . }" as dynasties;
                policy merge_by_source;
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("import wikidata as wd {\n"));
        assert!(out.contains("  entity Q7209 as han_dynasty;\n"));
        assert!(out.contains("  query \"SELECT ?item WHERE"));
        assert!(out.contains("  policy merge_by_source;\n"));
    }

    #[test]
    fn format_field_priority_policy() {
        let src = r#"
            import wikidata as wd {
                entity Q123 as foo;
                policy field_priority {
                    label: manual;
                    time: wikidata;
                    tags: merge;
                }
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("  policy field_priority {\n    label: manual;\n    time: wikidata;\n    tags: merge;\n  }\n"));
    }

    #[test]
    fn format_map_block_with_fallbacks() {
        let src = r#"
            map wd.han to span {
                lane han;
                start claim(P580).year ?? claim(P571).year;
                end claim(P582).year ?? claim(P576).year;
                label label@ja ?? label@en;
                tags ["dynasty", "china"];
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("map wd.han to span {\n"));
        assert!(out.contains("  lane han;\n"));
        assert!(out.contains("  start claim(P580).year ?? claim(P571).year;\n"));
        assert!(out.contains("  end claim(P582).year ?? claim(P576).year;\n"));
        assert!(out.contains("  label label@ja ?? label@en;\n"));
        assert!(out.contains("  tags [\"dynasty\", \"china\"];\n"));
    }

    #[test]
    fn format_map_block_with_literal_fallback() {
        let src = r#"
            map wd.x to span {
                lane a;
                start claim(P580).year ?? 0;
                end claim(P582).year ?? 9999;
                label label@ja;
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("  start claim(P580).year ?? 0;\n"));
        assert!(out.contains("  end claim(P582).year ?? 9999;\n"));
    }

    #[test]
    fn format_map_with_filter() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1000 && claim(P582).year < 2000;
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("  filter claim(P580).year > 1000 && claim(P582).year < 2000;\n"));
    }

    #[test]
    fn format_template_and_apply() {
        let src = r#"
            template "人物の生涯" as person_life to event_range {
                start claim(P569).year;
                end claim(P570).year;
                label label@ja ?? label@en;
            }
            apply person_life to emperors {
                lane imperial;
            }
        "#;
        let out = fmt(src);
        assert!(out.contains("template \"人物の生涯\" as person_life to event_range {\n"));
        assert!(out.contains("  start claim(P569).year;\n"));
        assert!(out.contains("apply person_life to emperors {\n  lane imperial;\n}\n"));
    }

    #[test]
    fn format_color_map_block() {
        let src = r##"timeline "T" { unit year; range 0..2000; color_map { dynasty: "#3366cc"; war: "#cc0000"; } }"##;
        let out = fmt(src);
        assert!(out.contains("  color_map {\n"));
        assert!(out.contains("    dynasty: \"#3366cc\";\n"));
        assert!(out.contains("    war: \"#cc0000\";\n"));
    }

    #[test]
    fn format_blank_line_between_statements() {
        let src = r#"
            lane "A" as a {}
            lane "B" as b {}
        "#;
        let out = fmt(src);
        // ブロック間に空行が 1 行入る
        assert!(out.contains("lane \"A\" as a {}\n\nlane \"B\" as b {}\n"));
    }

    #[test]
    fn format_is_idempotent_simple() {
        let src = r#"
            timeline "T" { title "T"; unit year; range 1900..2000; calendar proleptic_gregorian; }
            lane "A" as a { kind custom; order 1; }
            event a 1950 "X" { tags ["foo", "bar"]; id "evt:a:1950"; };
        "#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "format must be idempotent");
    }

    #[test]
    fn format_is_idempotent_full() {
        let src = r#"
            timeline "中国王朝" { title "中国王朝"; unit year; range -500..2000; calendar proleptic_gregorian; }
            lane "漢" as han { kind dynasty; order 10; }
            lane "秦" as qin { kind dynasty; order 20; }
            span han -206..220 "漢" { tags ["dynasty"]; source wd:Q7209; id "span:han"; };
            event han -209 "陳勝・呉広の乱" {};
            event_range han 184..204 "黄巾の乱" { tags ["war"]; };
            import wikidata as wd {
                entity Q7209 as han_dynasty;
                policy merge_by_source;
            }
            map wd.han_dynasty to span {
                lane han;
                start claim(P571).year ?? claim(P580).year;
                end claim(P576).year ?? claim(P582).year;
                label label@ja ?? label@en;
            }
        "#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "format must be idempotent for full example");
    }

    #[test]
    fn format_idempotent_with_filter_and_or() {
        let src = r#"
            map wd.x to span {
                lane a;
                filter claim(P580).year > 1
                    || (claim(P582).year < 0 && claim(P571).year > 2);
                start claim(P580).year;
                end claim(P582).year;
                label label@ja;
            }
        "#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "filter Or/And must round-trip");
    }

    #[test]
    fn format_output_is_parseable_full() {
        let src = r#"
            timeline "T" { title "T"; unit year; range 1900..2000; calendar proleptic_gregorian; }
            lane "A" as a { kind custom; order 1; }
            span a 1900..1950 "X" { id "x"; };
            event a 1925 "Y" {};
            event_range a 1930..1940 "Z" {};
            import wikidata as wd { entity Q1 as foo; policy keep_manual; }
            map wd.foo to span { lane a; start claim(P580).year; end claim(P582).year; label label@ja; }
        "#;
        let formatted = format_source(src).unwrap();
        let reparsed = parse(&formatted).expect("formatted output must reparse");
        assert_eq!(reparsed.statements.len(), 7);
    }

    #[test]
    fn format_returns_parse_error_on_invalid_input() {
        let result = format_source("this is not valid tdsl !!!");
        assert!(result.is_err());
    }

    #[test]
    fn format_escapes_quotes_in_strings() {
        let src = r#"lane "He said \"hello\"" as x {}"#;
        let out = fmt(src);
        assert!(out.contains(r#"lane "He said \"hello\"" as x"#));
        // 再パース可能
        parse(&out).expect("escaped output must reparse");
    }

    #[test]
    fn format_negative_year_in_range() {
        let src = r#"timeline "T" { unit year; range -500..2000; }"#;
        let out = fmt(src);
        assert!(out.contains("range -500..2000;"));
    }

    #[test]
    fn format_mixed_precision_span() {
        let src = r#"span ww 1939-09-01..1945-09-02 "WW2" {};"#;
        let out = fmt(src);
        assert!(out.contains("span ww 1939-09-01..1945-09-02 \"WW2\" {};"));
    }

    #[test]
    fn format_empty_import_block() {
        let src = r#"import wikidata as wd {}"#;
        let out = fmt(src);
        assert_eq!(out, "import wikidata as wd {}\n");
    }

    #[test]
    fn format_empty_apply_overrides() {
        let src = r#"apply t to imports {}"#;
        let out = fmt(src);
        assert_eq!(out, "apply t to imports {}\n");
    }

    // ─── comment preservation (#473) ────────────────────────────────────

    #[test]
    fn format_preserves_leading_line_comment() {
        let src = "// header\nlane \"A\" as a {}\n";
        let out = fmt(src);
        assert_eq!(out, "// header\nlane \"A\" as a {}\n");
    }

    #[test]
    fn format_preserves_comment_between_statements() {
        let src = "lane \"A\" as a {}\n// section\nlane \"B\" as b {}\n";
        let out = fmt(src);
        assert_eq!(
            out,
            "lane \"A\" as a {}\n\n// section\nlane \"B\" as b {}\n"
        );
    }

    #[test]
    fn format_preserves_trailing_same_line_comment() {
        let src = "lane \"A\" as a {} // tail\n";
        let out = fmt(src);
        assert_eq!(out, "lane \"A\" as a {} // tail\n");
    }

    #[test]
    fn format_preserves_block_comment() {
        let src = "/* doc */\nlane \"A\" as a {}\n";
        let out = fmt(src);
        assert_eq!(out, "/* doc */\nlane \"A\" as a {}\n");
    }

    #[test]
    fn format_preserves_trailing_eof_comment() {
        let src = "lane \"A\" as a {}\n// footer\n";
        let out = fmt(src);
        assert_eq!(out, "lane \"A\" as a {}\n\n// footer\n");
    }

    #[test]
    fn format_comment_preservation_is_idempotent() {
        let src = r#"// file header
lane "A" as a {} // inline
// before B
lane "B" as b { kind dynasty; order 2; }
// footer
"#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "comment preservation must be idempotent");
    }

    #[test]
    fn format_block_interior_comment_is_relocated_not_lost() {
        // ブロック内部コメントは内容が残り、再整形で冪等になる。
        let src = "lane \"A\" as a { kind dynasty; /* inline */ order 2; }\nlane \"B\" {}\n";
        let once = format_source(src).unwrap();
        assert!(once.contains("/* inline */"), "comment content preserved");
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "relocated comment must be idempotent");
    }

    #[test]
    fn format_comment_only_source() {
        let src = "// just a comment\n";
        let out = fmt(src);
        assert_eq!(out, "// just a comment\n");
    }

    #[test]
    fn format_output_with_comments_reparses() {
        let src = "// header\nlane \"A\" as a {} // tail\nlane \"B\" {}\n";
        let out = fmt(src);
        parse(&out).expect("formatted output with comments must reparse");
    }
}
