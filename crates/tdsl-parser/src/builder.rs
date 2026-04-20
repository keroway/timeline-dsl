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
    Ok(File { statements })
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
                let start = parse_int(&nums.next().unwrap())?;
                let end = parse_int(&nums.next().unwrap())?;
                block.range = Some(RangeExpr { start, end });
            }
            Rule::calendar_prop => {
                block.calendar = Some(prop.into_inner().next().unwrap().as_str().to_string());
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

// ─── Span ───────────────────────────────────────────────────

fn build_span(pair: Pair<'_, Rule>) -> Result<SpanDecl> {
    let mut inner = pair.into_inner();
    let lane_ref = inner.next().unwrap().as_str().to_string();
    let start = parse_int(&inner.next().unwrap())?;
    let end = parse_int(&inner.next().unwrap())?;
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
    let time = parse_int(&inner.next().unwrap())?;
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
    let start = parse_int(&inner.next().unwrap())?;
    let end = parse_int(&inner.next().unwrap())?;
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
        _ => {}
    }
    Ok(())
}

fn build_map_expr(pair: Pair<'_, Rule>) -> Result<MapExpr> {
    let mut inner = pair.into_inner();
    let claim_pair = inner.next().unwrap();
    let property = claim_pair.into_inner().next().unwrap().as_str().to_string();
    let accessor = inner.next().map(|p| p.as_str().to_string());

    Ok(MapExpr {
        claim: ClaimCall { property },
        accessor,
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
    // string_literal = ${ "\"" ~ string_inner ~ "\"" }
    let inner = pair.clone().into_inner().next().unwrap();
    inner.as_str().to_string()
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

fn pair_span(pair: &Pair<'_, Rule>) -> Span {
    let s = pair.as_span();
    Span {
        start: s.start(),
        end: s.end(),
    }
}
