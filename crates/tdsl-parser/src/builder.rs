use pest::iterators::{Pair, Pairs};

use crate::Rule;
use crate::ast::*;
use crate::error::ParseError;

type Result<T> = std::result::Result<T, ParseError>;

/// Build an AST [`File`] from the top-level pest parse result.
pub fn build_file(pairs: Pairs<'_, Rule>) -> Result<File> {
    let mut statements = Vec::new();
    // The top-level `file` rule wraps all statements
    let file_pair = pairs.into_iter().next().unwrap();
    for pair in file_pair.into_inner() {
        match pair.as_rule() {
            Rule::timeline_block => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Timeline(build_timeline(pair)?),
                    span,
                });
            }
            Rule::lane_decl => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Lane(build_lane(pair)?),
                    span,
                });
            }
            Rule::group_decl => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Group(build_group(pair)?),
                    span,
                });
            }
            Rule::span_decl => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Span(build_span(pair)?),
                    span,
                });
            }
            Rule::event_decl => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Event(build_event(pair)?),
                    span,
                });
            }
            Rule::event_range_decl => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::EventRange(build_event_range(pair)?),
                    span,
                });
            }
            Rule::import_block => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Import(build_import(pair)?),
                    span,
                });
            }
            Rule::map_block => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Map(build_map(pair)?),
                    span,
                });
            }
            Rule::template_block => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Template(build_template(pair)?),
                    span,
                });
            }
            Rule::apply_block => {
                let span = pair_span(&pair);
                statements.push(Spanned {
                    node: Statement::Apply(build_apply(pair)?),
                    span,
                });
            }
            Rule::EOI => {}
            _ => {}
        }
    }
    // コメントは build 段階では空。[`crate::parse`] が別パス（`comments::scan_comments`）で埋める。
    Ok(File {
        statements,
        comments: Vec::new(),
    })
}

// ─── Timeline ───────────────────────────────────────────────

fn build_timeline(pair: Pair<'_, Rule>) -> Result<TimelineBlock> {
    let mut inner = pair.into_inner();
    let name = extract_string_literal(&inner.next().unwrap());

    let mut block = TimelineBlock {
        name,
        title: None,
        unit: None,
        range: None,
        calendar: None,
        color_map: Vec::new(),
    };

    for prop in inner {
        match prop.as_rule() {
            Rule::title_prop => {
                block.title = Some(extract_string_literal(&prop.into_inner().next().unwrap()));
            }
            Rule::unit_prop => {
                block.unit = Some(prop.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::range_prop => {
                let mut nums = prop.into_inner();
                let start = parse_time_value(nums.next().unwrap())?;
                let end = parse_time_value(nums.next().unwrap())?;
                block.range = Some(RangeExpr { start, end });
            }
            Rule::calendar_prop => {
                block.calendar = Some(prop.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::color_map_block => {
                for entry in prop.into_inner() {
                    let mut ei = entry.into_inner();
                    let tag = ei.next().unwrap().as_str().to_string();
                    let color = extract_string_literal(&ei.next().unwrap());
                    block.color_map.push((tag, color));
                }
            }
            _ => {}
        }
    }
    Ok(block)
}

// ─── Lane ───────────────────────────────────────────────────

fn build_lane(pair: Pair<'_, Rule>) -> Result<LaneDecl> {
    let mut inner = pair.into_inner();
    let label = extract_string_literal(&inner.next().unwrap());

    let mut decl = LaneDecl {
        label,
        alias: None,
        kind: None,
        order: None,
    };

    for item in inner {
        match item.as_rule() {
            Rule::ident => {
                decl.alias = Some(item.as_str().to_string());
            }
            Rule::kind_prop => {
                decl.kind = Some(item.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::order_prop => {
                decl.order = Some(parse_int(&item.into_inner().next().unwrap())?);
            }
            _ => {}
        }
    }
    Ok(decl)
}

// ─── Group ──────────────────────────────────────────────────

fn build_group(pair: Pair<'_, Rule>) -> Result<GroupDecl> {
    let mut inner = pair.into_inner();
    let label = extract_string_literal(&inner.next().unwrap());
    let mut lanes = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::lane_decl {
            lanes.push(build_lane(item)?);
        }
    }
    Ok(GroupDecl { label, lanes })
}

// ─── Span ───────────────────────────────────────────────────

fn build_span(pair: Pair<'_, Rule>) -> Result<SpanDecl> {
    let mut inner = pair.into_inner();
    let lane_ref = inner.next().unwrap().as_str().to_string();
    let start = parse_time_value(inner.next().unwrap())?;
    let end = parse_time_value(inner.next().unwrap())?;
    let label = extract_string_literal(&inner.next().unwrap());
    let props = build_block_options(inner.next().unwrap())?;

    Ok(SpanDecl {
        lane_ref,
        start,
        end,
        label,
        props,
    })
}

// ─── Event ──────────────────────────────────────────────────

fn build_event(pair: Pair<'_, Rule>) -> Result<EventDecl> {
    let mut inner = pair.into_inner();
    let lane_ref = inner.next().unwrap().as_str().to_string();
    let time = parse_time_value(inner.next().unwrap())?;
    let label = extract_string_literal(&inner.next().unwrap());
    let props = build_block_options(inner.next().unwrap())?;

    Ok(EventDecl {
        lane_ref,
        time,
        label,
        props,
    })
}

// ─── Event Range ────────────────────────────────────────────

fn build_event_range(pair: Pair<'_, Rule>) -> Result<EventRangeDecl> {
    let mut inner = pair.into_inner();
    let lane_ref = inner.next().unwrap().as_str().to_string();
    let start = parse_time_value(inner.next().unwrap())?;
    let end = parse_time_value(inner.next().unwrap())?;
    let label = extract_string_literal(&inner.next().unwrap());
    let props = build_block_options(inner.next().unwrap())?;

    Ok(EventRangeDecl {
        lane_ref,
        start,
        end,
        label,
        props,
    })
}

// ─── Block Options ──────────────────────────────────────────

fn build_block_options(pair: Pair<'_, Rule>) -> Result<ItemProps> {
    let mut props = ItemProps::default();
    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::tags_option => {
                props.tags = build_string_list(item.into_inner().next().unwrap());
            }
            Rule::source_option => {
                props.source = Some(build_source_ref(item.into_inner().next().unwrap()));
            }
            Rule::id_option => {
                props.id = Some(extract_string_literal(&item.into_inner().next().unwrap()));
            }
            Rule::origin_option => {
                props.origin = Some(item.into_inner().next().unwrap().as_str().to_string());
            }
            _ => {}
        }
    }
    Ok(props)
}

// ─── Import ─────────────────────────────────────────────────

fn build_import(pair: Pair<'_, Rule>) -> Result<ImportBlock> {
    let mut inner = pair.into_inner();

    let source_type = inner
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .to_string();

    let mut alias = None;
    let mut items = Vec::new();
    let mut policy = None;

    for item in inner {
        match item.as_rule() {
            Rule::ident => {
                alias = Some(item.as_str().to_string());
            }
            Rule::entity_import => {
                let mut ei = item.into_inner();
                let qid = ei.next().unwrap().as_str().to_string();
                let entity_alias = ei.next().map(|p| p.as_str().to_string());
                items.push(ImportItem::Entity {
                    qid,
                    alias: entity_alias,
                });
            }
            Rule::query_import => {
                let mut qi = item.into_inner();
                let query = extract_string_literal(&qi.next().unwrap());
                let query_alias = qi.next().map(|p| p.as_str().to_string());
                items.push(ImportItem::Query {
                    query,
                    alias: query_alias,
                });
            }
            Rule::policy_import => {
                let name = item.into_inner().next().unwrap().as_str();
                policy = Some(match name {
                    "merge_by_source" => ReimportPolicy::MergeBySource,
                    "overwrite_imported" => ReimportPolicy::OverwriteImported,
                    "keep_manual" => ReimportPolicy::KeepManual,
                    other => return Err(ParseError::UnknownPolicy(other.to_string())),
                });
            }
            Rule::field_priority_policy => {
                let mut config = FieldPriorityConfig::default();
                for decl in item.into_inner() {
                    let rule = decl.as_rule();
                    let strategy_str = decl.into_inner().next().unwrap().as_str();
                    let strategy = match strategy_str {
                        "manual" => FieldStrategy::Manual,
                        "wikidata" => FieldStrategy::Wikidata,
                        "merge" => FieldStrategy::Merge,
                        _ => unreachable!("grammar enforces valid values"),
                    };
                    match rule {
                        Rule::label_strategy => config.label = strategy,
                        Rule::time_strategy => config.time = strategy,
                        Rule::tags_strategy => config.tags = strategy,
                        _ => {}
                    }
                }
                policy = Some(ReimportPolicy::FieldPriority(config));
            }
            _ => {}
        }
    }

    Ok(ImportBlock {
        source_type,
        alias,
        items,
        policy,
    })
}

// ─── Template ───────────────────────────────────────────────

fn build_template(pair: Pair<'_, Rule>) -> Result<TemplateBlock> {
    let mut inner = pair.into_inner();
    let name = extract_string_literal(&inner.next().unwrap());

    // Optional alias: if the next token is an ident before target_type
    let mut alias = None;
    let mut peeked = inner.next().unwrap();
    if peeked.as_rule() == Rule::ident {
        alias = Some(peeked.as_str().to_string());
        peeked = inner.next().unwrap();
    }

    // peeked is now target_type
    let target_type = parse_target_type(peeked.into_inner().next().unwrap())?;

    let mut props = Vec::new();
    for item in inner {
        build_map_prop(item, &mut props)?;
    }

    Ok(TemplateBlock {
        name,
        alias,
        target_type,
        props,
    })
}

// ─── Apply ──────────────────────────────────────────────────

fn build_apply(pair: Pair<'_, Rule>) -> Result<ApplyBlock> {
    let mut inner = pair.into_inner();
    let template_alias = inner.next().unwrap().as_str().to_string();
    let import_alias = inner.next().unwrap().as_str().to_string();

    let mut overrides = Vec::new();
    for item in inner {
        build_map_prop(item, &mut overrides)?;
    }

    Ok(ApplyBlock {
        template_alias,
        import_alias,
        overrides,
    })
}

// ─── Map ────────────────────────────────────────────────────

fn build_map(pair: Pair<'_, Rule>) -> Result<MapBlock> {
    let mut inner = pair.into_inner();
    let source_ref = inner.next().unwrap().as_str().to_string();
    let target_type = parse_target_type(inner.next().unwrap().into_inner().next().unwrap())?;

    let mut props = Vec::new();
    for item in inner {
        build_map_prop(item, &mut props)?;
    }

    Ok(MapBlock {
        source_ref,
        target_type,
        props,
    })
}

fn build_map_prop(item: Pair<'_, Rule>, props: &mut Vec<MapProp>) -> Result<()> {
    match item.as_rule() {
        Rule::map_lane => {
            props.push(MapProp::Lane(
                item.into_inner().next().unwrap().as_str().to_string(),
            ));
        }
        Rule::map_start => {
            props.push(MapProp::Start(build_map_expr(
                item.into_inner().next().unwrap(),
            )?));
        }
        Rule::map_end => {
            props.push(MapProp::End(build_map_expr(
                item.into_inner().next().unwrap(),
            )?));
        }
        Rule::map_time => {
            props.push(MapProp::Time(build_map_expr(
                item.into_inner().next().unwrap(),
            )?));
        }
        Rule::map_label => {
            props.push(MapProp::Label(build_label_expr(
                item.into_inner().next().unwrap(),
            )));
        }
        Rule::map_tags => {
            props.push(MapProp::Tags(build_string_list(
                item.into_inner().next().unwrap(),
            )));
        }
        Rule::map_filter => {
            props.push(MapProp::Filter(build_filter_expr(
                item.into_inner().next().unwrap(),
            )?));
        }
        Rule::map_expand => {
            // map_expand = { "expand" ~ claim_call ~ ";" }
            let claim_pair = item.into_inner().next().unwrap();
            let property = claim_pair.into_inner().next().unwrap().as_str().to_string();
            props.push(MapProp::Expand(ClaimCall { property }));
        }
        _ => {}
    }
    Ok(())
}

// ─── Filter expressions ─────────────────────────────────────

fn build_filter_expr(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    // filter_expr wraps filter_or
    let inner = pair.into_inner().next().unwrap();
    build_filter_or(inner)
}

fn build_filter_or(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let mut iter = pair.into_inner();
    let first = build_filter_and(iter.next().unwrap())?;
    iter.try_fold(first, |acc, p| {
        let rhs = build_filter_and(p)?;
        Ok(FilterExpr::Or(Box::new(acc), Box::new(rhs)))
    })
}

fn build_filter_and(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let mut iter = pair.into_inner();
    let first = build_filter_not(iter.next().unwrap())?;
    iter.try_fold(first, |acc, p| {
        let rhs = build_filter_not(p)?;
        Ok(FilterExpr::And(Box::new(acc), Box::new(rhs)))
    })
}

fn build_filter_not(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let s = pair.as_str();
    let inner = pair.into_inner().next().unwrap();
    let atom = build_filter_atom(inner)?;
    if s.trim_start().starts_with('!') {
        Ok(FilterExpr::Not(Box::new(atom)))
    } else {
        Ok(atom)
    }
}

fn build_filter_atom(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let location = pair_location_str(&pair);
    match pair.as_rule() {
        Rule::filter_paren => build_filter_expr(pair.into_inner().next().unwrap()),
        Rule::filter_compare => build_filter_compare(pair),
        Rule::filter_string_op => build_filter_string_op(pair),
        other => Err(ParseError::UnexpectedRule {
            rule: format!("filter_atom: {other:?}"),
            location,
        }),
    }
}

fn build_filter_string_op(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let mut iter = pair.into_inner();
    let label_ref_pair = iter.next().unwrap();
    let lang = label_ref_pair
        .as_str()
        .strip_prefix("label@")
        .unwrap_or(label_ref_pair.as_str())
        .to_string();
    let op_pair = iter.next().unwrap();
    let location = pair_location_str(&op_pair);
    let op = match op_pair.as_str() {
        "contains" => StringMatchOp::Contains,
        "startswith" => StringMatchOp::StartsWith,
        other => {
            return Err(ParseError::UnexpectedRule {
                rule: format!("string_match_op: {other}"),
                location,
            });
        }
    };
    let rhs_pair = iter.next().unwrap();
    let rhs = extract_string_literal(&rhs_pair);
    Ok(FilterExpr::StringMatch {
        lhs: LabelRef { lang },
        op,
        rhs,
    })
}

fn build_filter_compare(pair: Pair<'_, Rule>) -> Result<FilterExpr> {
    let mut iter = pair.into_inner();
    let lhs = build_filter_operand(iter.next().unwrap())?;
    let op_pair = iter.next().unwrap();
    let op_location = pair_location_str(&op_pair);
    let op = parse_compare_op(op_pair.as_str(), op_location)?;
    let rhs = build_filter_operand(iter.next().unwrap())?;
    Ok(FilterExpr::Compare { lhs, op, rhs })
}

fn build_filter_operand(pair: Pair<'_, Rule>) -> Result<FilterOperand> {
    let location = pair_location_str(&pair);
    match pair.as_rule() {
        Rule::null_literal => Ok(FilterOperand::Null),
        Rule::claim_expr => Ok(FilterOperand::Claim(build_claim_expr(pair)?)),
        Rule::integer => {
            let raw = pair.as_str().to_string();
            let n: i64 = raw.parse().map_err(|_| ParseError::InvalidInt {
                value: raw,
                location,
            })?;
            Ok(FilterOperand::Int(n))
        }
        other => Err(ParseError::UnexpectedRule {
            rule: format!("filter_operand: {other:?}"),
            location,
        }),
    }
}

fn parse_compare_op(s: &str, location: String) -> Result<CompareOp> {
    match s {
        "==" => Ok(CompareOp::Eq),
        "!=" => Ok(CompareOp::NotEq),
        "<" => Ok(CompareOp::Lt),
        "<=" => Ok(CompareOp::Le),
        ">" => Ok(CompareOp::Gt),
        ">=" => Ok(CompareOp::Ge),
        other => Err(ParseError::UnexpectedRule {
            rule: format!("compare_op: {other}"),
            location,
        }),
    }
}

fn build_map_expr(pair: Pair<'_, Rule>) -> Result<MapExpr> {
    let mut fallbacks = Vec::new();
    for child in pair.into_inner() {
        let fb = match child.as_rule() {
            Rule::claim_expr => MapFallback::Claim(build_claim_expr(child)?),
            Rule::integer => {
                let span = child.as_span();
                let n: i64 = child.as_str().parse().map_err(|_| ParseError::InvalidInt {
                    value: child.as_str().to_string(),
                    location: format!("{}:{}", span.start(), span.end()),
                })?;
                MapFallback::Literal(n)
            }
            _ => {
                let span = child.as_span();
                return Err(ParseError::UnexpectedRule {
                    rule: format!("{:?}", child.as_rule()),
                    location: format!("{}:{}", span.start(), span.end()),
                });
            }
        };
        fallbacks.push(fb);
    }
    Ok(MapExpr { fallbacks })
}

fn build_claim_expr(pair: Pair<'_, Rule>) -> Result<ClaimExpr> {
    let mut inner = pair.into_inner();
    let claim_pair = inner.next().unwrap();
    let property = claim_pair.into_inner().next().unwrap().as_str().to_string();

    let mut qualifier = None;
    let mut accessor = None;
    let mut offset = None;
    for child in inner {
        match child.as_rule() {
            Rule::claim_qualifier => {
                // claim_qualifier = { "." ~ "qualifier" ~ "(" ~ property_id ~ ")" }
                qualifier = Some(child.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::claim_accessor => {
                // claim_accessor = { "." ~ ident }
                accessor = Some(child.into_inner().next().unwrap().as_str().to_string());
            }
            Rule::claim_offset => {
                // as_str() gives "+1" or "-30"; parse directly as i32
                offset = child.as_str().parse::<i32>().ok();
            }
            _ => {}
        }
    }

    Ok(ClaimExpr {
        claim: ClaimCall { property },
        qualifier,
        accessor,
        offset,
    })
}

fn build_label_expr(pair: Pair<'_, Rule>) -> LabelExpr {
    let fallbacks = pair
        .into_inner()
        .map(|lr| {
            // label_ref is compound: "label" "@" ident — the whole thing is captured
            let s = lr.as_str();
            let lang = s.strip_prefix("label@").unwrap_or(s).to_string();
            LabelRef { lang }
        })
        .collect();
    LabelExpr { fallbacks }
}

fn parse_target_type(pair: Pair<'_, Rule>) -> Result<MapTargetType> {
    match pair.as_str() {
        "span" => Ok(MapTargetType::Span),
        "event" => Ok(MapTargetType::Event),
        "event_range" => Ok(MapTargetType::EventRange),
        other => Err(ParseError::UnknownTargetType(other.to_string())),
    }
}

// ─── Helpers ────────────────────────────────────────────────

fn extract_string_literal(pair: &Pair<'_, Rule>) -> String {
    let inner = pair.clone().into_inner().next().unwrap();
    let s = inner.as_str();
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(c) => {
                    out.push('\\');
                    out.push(c);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn build_source_ref(pair: Pair<'_, Rule>) -> SourceRef {
    let s = pair.as_str();
    let (prefix, qid) = s.split_once(':').unwrap();
    SourceRef {
        prefix: prefix.to_string(),
        qid: qid.to_string(),
    }
}

fn build_string_list(pair: Pair<'_, Rule>) -> Vec<String> {
    pair.into_inner()
        .map(|p| extract_string_literal(&p))
        .collect()
}

fn parse_int(pair: &Pair<'_, Rule>) -> Result<i64> {
    pair.as_str()
        .parse::<i64>()
        .map_err(|_| ParseError::InvalidInt {
            value: pair.as_str().to_string(),
            location: format!("{}:{}", pair.as_span().start(), pair.as_span().end()),
        })
}

/// `time_value` ノードを [`TimeValue`] に変換する。
///
/// PEG ルールは `date_time_lit | date_lit | year_month_lit | year_lit` の順に試行され、
/// 月は 1〜12、日は 1〜31、時は 0〜23、分は 0〜59 の範囲を builder 側で検証する。
/// カレンダー妥当性（2月30日など）は lowering 側の責務。
pub(crate) fn parse_time_value(pair: Pair<'_, Rule>) -> Result<TimeValue> {
    let location = pair_location_str(&pair);
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::UnexpectedRule {
            rule: "time_value: empty inner".to_string(),
            location: location.clone(),
        })?;
    match inner.as_rule() {
        Rule::year_lit => {
            let year = inner
                .as_str()
                .parse::<i64>()
                .map_err(|_| ParseError::InvalidInt {
                    value: inner.as_str().to_string(),
                    location: location.clone(),
                })?;
            Ok(TimeValue::Year(year))
        }
        Rule::year_month_lit => {
            let s = inner.as_str();
            let (year, month, _) = parse_date_parts(s, &location)?;
            check_month(month, &location)?;
            Ok(TimeValue::YearMonth(year, month as u8))
        }
        Rule::date_lit => {
            let s = inner.as_str();
            let (year, month, day) = parse_date_parts(s, &location)?;
            let day = day.ok_or_else(|| ParseError::UnexpectedRule {
                rule: format!("date_lit malformed: {s}"),
                location: location.clone(),
            })?;
            check_month(month, &location)?;
            check_day(day, &location)?;
            Ok(TimeValue::Date(year, month as u8, day as u8))
        }
        Rule::date_time_lit => {
            let s = inner.as_str();
            let (date, clock) = s
                .split_once('T')
                .ok_or_else(|| ParseError::UnexpectedRule {
                    rule: format!("date_time_lit malformed: {s}"),
                    location: location.clone(),
                })?;
            let (year, month, day) = parse_date_parts(date, &location)?;
            let day = day.ok_or_else(|| ParseError::UnexpectedRule {
                rule: format!("date_time_lit malformed: {s}"),
                location: location.clone(),
            })?;
            let (hour, minute) =
                clock
                    .split_once(':')
                    .ok_or_else(|| ParseError::UnexpectedRule {
                        rule: format!("date_time_lit malformed: {s}"),
                        location: location.clone(),
                    })?;
            let hour: u32 = hour.parse().map_err(|_| ParseError::InvalidInt {
                value: hour.to_string(),
                location: location.clone(),
            })?;
            let minute: u32 = minute.parse().map_err(|_| ParseError::InvalidInt {
                value: minute.to_string(),
                location: location.clone(),
            })?;
            check_month(month, &location)?;
            check_day(day, &location)?;
            check_hour(hour, &location)?;
            check_minute(minute, &location)?;
            Ok(TimeValue::DateTime(
                year,
                month as u8,
                day as u8,
                hour as u8,
                minute as u8,
            ))
        }
        other => Err(ParseError::UnexpectedRule {
            rule: format!("time_value: {other:?}"),
            location,
        }),
    }
}

fn parse_date_parts(s: &str, location: &str) -> Result<(i64, u32, Option<u32>)> {
    let rest = s.strip_prefix('-').unwrap_or(s);
    let mut parts = rest.splitn(3, '-');
    let y = parts.next().ok_or_else(|| ParseError::UnexpectedRule {
        rule: format!("time literal malformed: {s}"),
        location: location.to_string(),
    })?;
    let m = parts.next().ok_or_else(|| ParseError::UnexpectedRule {
        rule: format!("time literal malformed: {s}"),
        location: location.to_string(),
    })?;
    let d = parts.next();
    let year = y.parse::<i64>().map_err(|_| ParseError::InvalidInt {
        value: y.to_string(),
        location: location.to_string(),
    })?;
    let month: u32 = m.parse().map_err(|_| ParseError::InvalidInt {
        value: m.to_string(),
        location: location.to_string(),
    })?;
    let day = d
        .map(|d| {
            d.parse().map_err(|_| ParseError::InvalidInt {
                value: d.to_string(),
                location: location.to_string(),
            })
        })
        .transpose()?;
    Ok((if s.starts_with('-') { -year } else { year }, month, day))
}

fn check_month(month: u32, location: &str) -> Result<()> {
    if (1..=12).contains(&month) {
        Ok(())
    } else {
        Err(ParseError::InvalidMonth {
            value: month,
            location: location.to_string(),
        })
    }
}

fn check_day(day: u32, location: &str) -> Result<()> {
    if (1..=31).contains(&day) {
        Ok(())
    } else {
        Err(ParseError::InvalidDay {
            value: day,
            location: location.to_string(),
        })
    }
}

fn check_hour(hour: u32, location: &str) -> Result<()> {
    if hour <= 23 {
        Ok(())
    } else {
        Err(ParseError::InvalidInt {
            value: hour.to_string(),
            location: location.to_string(),
        })
    }
}

fn check_minute(minute: u32, location: &str) -> Result<()> {
    if minute <= 59 {
        Ok(())
    } else {
        Err(ParseError::InvalidInt {
            value: minute.to_string(),
            location: location.to_string(),
        })
    }
}

fn pair_span(pair: &Pair<'_, Rule>) -> Span {
    let s = pair.as_span();
    Span {
        start: s.start(),
        end: s.end(),
    }
}

fn pair_location_str(pair: &Pair<'_, Rule>) -> String {
    let s = pair.as_span();
    format!("{}:{}", s.start(), s.end())
}
