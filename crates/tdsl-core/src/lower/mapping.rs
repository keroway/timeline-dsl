#[cfg(feature = "wikidata")]
use tdsl_parser::ast::{self, MapTargetType};
#[cfg(feature = "wikidata")]
use tdsl_wikidata::entity::{DataValue, TimePoint, WikidataEntity, time_value_to_timepoint};

#[cfg(feature = "wikidata")]
use crate::error::LoweringError;
#[cfg(feature = "wikidata")]
use crate::ir::Item;

#[cfg(feature = "wikidata")]
use super::context::LoweringContext;

#[cfg(feature = "wikidata")]
impl LoweringContext {
    /// Pass 4: Apply map blocks and apply blocks to generate items from imported entities.
    pub(crate) fn pass4_apply_maps(&mut self, file: &ast::File) {
        for stmt in &file.statements {
            if let ast::Statement::Apply(apply) = &stmt.node {
                self.process_apply_block(apply);
                continue;
            }
            if let ast::Statement::Map(m) = &stmt.node {
                // Parse source_ref: "wd.han_dynasty" -> import_alias="wd", entity_key="han_dynasty"
                let parts: Vec<&str> = m.source_ref.splitn(2, '.').collect();
                if parts.len() != 2 {
                    self.errors
                        .push(LoweringError::UnresolvedImport(m.source_ref.clone()));
                    continue;
                }
                let (import_alias, entity_key) = (parts[0], parts[1]);
                let policy = *self
                    .import_policies
                    .get(import_alias)
                    .unwrap_or(&ast::ReimportPolicy::MergeBySource);

                let entities = match self.import_entities.get(import_alias) {
                    Some(entities) => entities,
                    None => {
                        self.errors
                            .push(LoweringError::UnresolvedImport(import_alias.to_string()));
                        continue;
                    }
                };

                if let Some(entity) = entities.get(entity_key) {
                    self.apply_map_to_entity(m, &entity.clone(), policy);
                    continue;
                }

                if let Some(entity_groups) = self.import_groups.get(import_alias)
                    && let Some(keys) = entity_groups.get(entity_key)
                {
                    let mapped_entities: Vec<WikidataEntity> = keys
                        .iter()
                        .filter_map(|k| entities.get(k).cloned())
                        .collect();
                    for entity in &mapped_entities {
                        self.apply_map_to_entity(m, entity, policy);
                    }
                    continue;
                }

                self.errors.push(LoweringError::UnresolvedEntity(format!(
                    "{}.{}",
                    import_alias, entity_key
                )));
            }
        }
    }

    pub(crate) fn process_apply_block(&mut self, apply: &ast::ApplyBlock) {
        let template = match self.templates.get(&apply.template_alias).cloned() {
            Some(t) => t,
            None => {
                self.errors
                    .push(LoweringError::UnknownTemplate(apply.template_alias.clone()));
                return;
            }
        };

        let entities = match self.import_entities.get(&apply.import_alias).cloned() {
            Some(e) => e,
            None => {
                self.errors
                    .push(LoweringError::UnresolvedImport(apply.import_alias.clone()));
                return;
            }
        };

        let policy = *self
            .import_policies
            .get(&apply.import_alias)
            .unwrap_or(&ast::ReimportPolicy::MergeBySource);

        // Merge template props with apply overrides (overrides win)
        let merged_props = merge_map_props(&template.props, &apply.overrides);

        // Build a synthetic MapBlock to reuse apply_map_to_entity
        let synthetic_map = ast::MapBlock {
            source_ref: apply.import_alias.clone(),
            target_type: template.target_type,
            props: merged_props,
        };

        // Apply to every entity in the import (both direct entity and query results are
        // stored in import_entities, so a single pass is sufficient).
        for entity in entities.values() {
            self.apply_map_to_entity(&synthetic_map, entity, policy);
        }
    }

    pub(crate) fn apply_map_to_entity(
        &mut self,
        map: &ast::MapBlock,
        entity: &WikidataEntity,
        policy: ast::ReimportPolicy,
    ) {
        // Check for an expand directive
        let expand_prop = map.props.iter().find_map(|p| {
            if let ast::MapProp::Expand(call) = p {
                Some(call.property.as_str())
            } else {
                None
            }
        });

        if let Some(prop) = expand_prop {
            // Collect non-deprecated statements for the expand property
            let stmts: Vec<&tdsl_wikidata::entity::Statement> = entity
                .statements(prop)
                .iter()
                .filter(|s| s.rank != "deprecated")
                .collect();

            // No statements means nothing to expand — not a silent fallback, just nothing to do
            for (idx, stmt) in stmts.iter().enumerate() {
                self.apply_map_to_entity_with_ctx(
                    map,
                    entity,
                    Some(stmt),
                    Some((prop, idx)),
                    policy,
                );
            }
        } else {
            self.apply_map_to_entity_with_ctx(map, entity, None, None, policy);
        }
    }

    /// Core mapping logic, optionally scoped to a specific Statement context for `expand`.
    ///
    /// `ctx_stmt` — the current statement being iterated in an `expand` loop.
    /// `expand_ctx` — `(expand_property, statement_index)` used to generate unique item IDs
    ///                 when multiple statements are expanded.
    pub(crate) fn apply_map_to_entity_with_ctx(
        &mut self,
        map: &ast::MapBlock,
        entity: &WikidataEntity,
        ctx_stmt: Option<&tdsl_wikidata::entity::Statement>,
        expand_ctx: Option<(&str, usize)>,
        policy: ast::ReimportPolicy,
    ) {
        let mut lane_ref = String::new();
        let mut start: Option<TimePoint> = None;
        let mut end: Option<TimePoint> = None;
        let mut time: Option<TimePoint> = None;
        let mut label = String::new();
        let mut tags = Vec::new();

        // First pass: evaluate filters; skip this entity if any filter is false.
        // Filters are evaluated without expand context (entity-level filtering).
        for prop in &map.props {
            if let ast::MapProp::Filter(expr) = prop
                && !eval_filter_expr(expr, entity)
            {
                return;
            }
        }

        for prop in &map.props {
            match prop {
                ast::MapProp::Lane(l) => lane_ref = l.clone(),
                ast::MapProp::Start(expr) => start = eval_map_expr(expr, entity, ctx_stmt),
                ast::MapProp::End(expr) => end = eval_map_expr(expr, entity, ctx_stmt),
                ast::MapProp::Time(expr) => time = eval_map_expr(expr, entity, ctx_stmt),
                ast::MapProp::Label(lexpr) => {
                    label = eval_label_expr(lexpr, entity).unwrap_or_default();
                }
                ast::MapProp::Tags(t) => tags = t.clone(),
                ast::MapProp::Filter(_) | ast::MapProp::Expand(_) => {}
            }
        }

        // Validate lane existence
        if !lane_ref.is_empty() && !self.lanes_map.contains_key(&lane_ref) {
            let err = self.make_unknown_mapped_lane_error(&lane_ref);
            self.errors.push(err);
            return;
        }

        // Describe the mapping target for diagnostics (entity + optional expand context).
        let target_desc = match expand_ctx {
            Some((prop, idx)) => format!("{} ({prop}#{idx})", entity.id),
            None => entity.id.clone(),
        };

        if lane_ref.is_empty() {
            self.warnings.push(format!(
                "Mapped entity {target_desc} produced no item: required `lane` is unresolved/empty"
            ));
            return;
        }
        if label.is_empty() {
            self.warnings.push(format!(
                "Mapped entity {target_desc} produced no item: required `label` could not be resolved"
            ));
            return;
        }

        let source_id = format!("wd:{}", entity.id);
        self.sources.push(crate::ir::SourceRecord {
            id: source_id.clone(),
            provider: "wikidata".to_string(),
            license: "CC0".to_string(),
        });

        let item_source = Some(source_id.clone());
        let origin = Some("wikidata".to_string());

        // 仕様 §4: 紀元前（year < 0）のデータは year 精度に丸める
        let strip_bc = |tp: &TimePoint| -> (Option<u8>, Option<u8>) {
            if tp.year < 0 {
                (None, None)
            } else {
                (tp.month, tp.day)
            }
        };

        // Generate item ID. When expand is active, include the property and statement index
        // to ensure uniqueness across multiple statements for the same entity.
        let make_id = |prefix: &str, year: i64| -> String {
            match expand_ctx {
                Some((expand_prop, idx)) => format!(
                    "{prefix}:{}_{}_{idx}:{year}",
                    entity.id.to_lowercase(),
                    expand_prop.to_lowercase()
                ),
                None => format!("{prefix}:{}:{year}", entity.id.to_lowercase()),
            }
        };

        match map.target_type {
            MapTargetType::Span => {
                if let (Some(s), Some(e)) = (start, end) {
                    let id = make_id("span", s.year);
                    let (s_month, s_day) = strip_bc(&s);
                    let (e_month, e_day) = strip_bc(&e);
                    let item = Item::Span {
                        id,
                        lane: lane_ref,
                        start: s.year,
                        end: e.year,
                        label,
                        tags,
                        source: item_source,
                        origin,
                        start_month: s_month,
                        start_day: s_day,
                        end_month: e_month,
                        end_day: e_day,
                        source_span: None,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                } else {
                    self.warnings.push(format!(
                        "Mapped entity {target_desc} produced no `span`: `start`/`end` could not be resolved"
                    ));
                }
            }
            MapTargetType::Event => {
                if let Some(t) = time {
                    let id = make_id("event", t.year);
                    let (t_month, t_day) = strip_bc(&t);
                    let item = Item::Event {
                        id,
                        lane: lane_ref,
                        time: t.year,
                        label,
                        tags,
                        source: item_source,
                        origin,
                        time_month: t_month,
                        time_day: t_day,
                        source_span: None,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                } else {
                    self.warnings.push(format!(
                        "Mapped entity {target_desc} produced no `event`: `time` could not be resolved"
                    ));
                }
            }
            MapTargetType::EventRange => {
                if let (Some(s), Some(e)) = (start, end) {
                    let id = make_id("event_range", s.year);
                    let (s_month, s_day) = strip_bc(&s);
                    let (e_month, e_day) = strip_bc(&e);
                    let item = Item::EventRange {
                        id,
                        lane: lane_ref,
                        start: s.year,
                        end: e.year,
                        label,
                        tags,
                        source: item_source,
                        origin,
                        start_month: s_month,
                        start_day: s_day,
                        end_month: e_month,
                        end_day: e_day,
                        source_span: None,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                } else {
                    self.warnings.push(format!(
                        "Mapped entity {target_desc} produced no `event_range`: `start`/`end` could not be resolved"
                    ));
                }
            }
        }
    }
}

// ─── Expression Evaluation ──────────────────────────────────

/// Evaluate a map expression with `??` fallback.
/// Supports claim chains (`claim(P580).year ?? claim(P571).year`) and literal fallbacks
/// (`claim(P580).year ?? 9999`). Returns the first resolved value.
///
/// `ctx_stmt` — when `Some`, provides the Statement context for an `expand` loop iteration.
/// This allows `claim(P39).qualifier(P580).year` to resolve against the iterated statement.
#[cfg(feature = "wikidata")]
pub(crate) fn eval_map_expr(
    expr: &ast::MapExpr,
    entity: &WikidataEntity,
    ctx_stmt: Option<&tdsl_wikidata::entity::Statement>,
) -> Option<TimePoint> {
    for fb in &expr.fallbacks {
        match fb {
            ast::MapFallback::Claim(ce) => {
                if let Some(tp) = eval_claim_expr(ce, entity, ctx_stmt) {
                    return Some(tp);
                }
            }
            ast::MapFallback::Literal(n) => {
                return Some(TimePoint {
                    year: *n,
                    month: None,
                    day: None,
                    precision: 9,
                });
            }
        }
    }
    None
}

/// Evaluate a single claim expression (e.g. `claim(P571).year`) against a Wikidata entity.
/// Returns a [`TimePoint`] with precision information based on the accessor:
/// - `.year` or no accessor: year only
/// - `.month`: year + month (if precision >= 10)
/// - `.day`: year + month + day (if precision >= 11)
///
/// If `expr.offset` is set, the integer offset is added to the resolved year.
///
/// When `expr.qualifier` is set and `ctx_stmt` is provided, resolves the qualifier from
/// `ctx_stmt` (the current expand iteration's Statement).  Without `ctx_stmt`, the first
/// non-deprecated statement's qualifier is used.
#[cfg(feature = "wikidata")]
pub(crate) fn eval_claim_expr(
    expr: &ast::ClaimExpr,
    entity: &WikidataEntity,
    ctx_stmt: Option<&tdsl_wikidata::entity::Statement>,
) -> Option<TimePoint> {
    let dv: &DataValue = if let Some(qual_prop) = &expr.qualifier {
        // Qualifier access
        if let Some(stmt) = ctx_stmt {
            // Inside an expand loop — use the iterated statement's qualifier
            stmt.qualifiers
                .get(qual_prop)?
                .first()
                .and_then(|s| s.datavalue.as_ref())?
        } else {
            // Outside expand — use the first non-deprecated statement's qualifier
            entity.qualifier_claim(&expr.claim.property, qual_prop)?
        }
    } else {
        entity.claim(&expr.claim.property)?
    };

    match dv {
        DataValue::Time { value } => {
            let tp = time_value_to_timepoint(value).ok()?;
            let mut result = match expr.accessor.as_deref() {
                Some("month") => tp.month.map(|_| tp)?,
                Some("day") => tp.day.map(|_| tp)?,
                Some("year") | None => TimePoint {
                    year: tp.year,
                    month: None,
                    day: None,
                    precision: tp.precision,
                },
                _ => return None,
            };
            if let Some(off) = expr.offset {
                result.year += i64::from(off);
            }
            Some(result)
        }
        _ => None,
    }
}

/// Evaluate a label expression with fallback (e.g. `label@ja ?? label@en`).
#[cfg(feature = "wikidata")]
pub(crate) fn eval_label_expr(expr: &ast::LabelExpr, entity: &WikidataEntity) -> Option<String> {
    let langs: Vec<&str> = expr.fallbacks.iter().map(|lr| lr.lang.as_str()).collect();
    entity.label_with_fallback(&langs).map(|s| s.to_string())
}

/// Evaluate a filter expression against an entity. Returns `true` if the entity passes.
#[cfg(feature = "wikidata")]
pub(crate) fn eval_filter_expr(expr: &ast::FilterExpr, entity: &WikidataEntity) -> bool {
    match expr {
        ast::FilterExpr::And(a, b) => eval_filter_expr(a, entity) && eval_filter_expr(b, entity),
        ast::FilterExpr::Or(a, b) => eval_filter_expr(a, entity) || eval_filter_expr(b, entity),
        ast::FilterExpr::Not(a) => !eval_filter_expr(a, entity),
        ast::FilterExpr::Compare { lhs, op, rhs } => eval_filter_compare(lhs, *op, rhs, entity),
        ast::FilterExpr::StringMatch { lhs, op, rhs } => {
            let label = entity
                .labels
                .get(&lhs.lang)
                .map(|lv| lv.value.as_str())
                .unwrap_or("");
            match op {
                ast::StringMatchOp::Contains => label.contains(rhs.as_str()),
                ast::StringMatchOp::StartsWith => label.starts_with(rhs.as_str()),
            }
        }
    }
}

#[cfg(feature = "wikidata")]
fn eval_filter_compare(
    lhs: &ast::FilterOperand,
    op: ast::CompareOp,
    rhs: &ast::FilterOperand,
    entity: &WikidataEntity,
) -> bool {
    let lv = resolve_filter_operand(lhs, entity);
    let rv = resolve_filter_operand(rhs, entity);
    match (lv, rv) {
        // null-vs-null
        (None, None) => matches!(op, ast::CompareOp::Eq),
        // null-vs-value: only Eq/NotEq are meaningful; order comparisons are false.
        (None, Some(_)) | (Some(_), None) => matches!(op, ast::CompareOp::NotEq),
        (Some(a), Some(b)) => match op {
            ast::CompareOp::Eq => a == b,
            ast::CompareOp::NotEq => a != b,
            ast::CompareOp::Lt => a < b,
            ast::CompareOp::Le => a <= b,
            ast::CompareOp::Gt => a > b,
            ast::CompareOp::Ge => a >= b,
        },
    }
}

/// Resolve a filter operand to an optional integer value.
/// `null` and unevaluable claim expressions both yield `None`.
/// Filter expressions are always evaluated outside an expand context, so `ctx_stmt` is `None`.
#[cfg(feature = "wikidata")]
fn resolve_filter_operand(op: &ast::FilterOperand, entity: &WikidataEntity) -> Option<i64> {
    match op {
        ast::FilterOperand::Int(n) => Some(*n),
        ast::FilterOperand::Null => None,
        ast::FilterOperand::Claim(ce) => eval_claim_expr(ce, entity, None).map(|tp| tp.year),
    }
}

// ─── Helpers ────────────────────────────────────────────────

/// Merge template props with apply overrides.
/// Override props of the same variant replace the template props.
#[cfg(feature = "wikidata")]
pub(crate) fn merge_map_props(
    base: &[ast::MapProp],
    overrides: &[ast::MapProp],
) -> Vec<ast::MapProp> {
    let mut result = base.to_vec();
    for ov in overrides {
        let replaced = result.iter_mut().find(|p| props_same_variant(p, ov));
        if let Some(slot) = replaced {
            *slot = ov.clone();
        } else {
            result.push(ov.clone());
        }
    }
    result
}

#[cfg(feature = "wikidata")]
fn props_same_variant(a: &ast::MapProp, b: &ast::MapProp) -> bool {
    matches!(
        (a, b),
        (ast::MapProp::Lane(_), ast::MapProp::Lane(_))
            | (ast::MapProp::Start(_), ast::MapProp::Start(_))
            | (ast::MapProp::End(_), ast::MapProp::End(_))
            | (ast::MapProp::Time(_), ast::MapProp::Time(_))
            | (ast::MapProp::Label(_), ast::MapProp::Label(_))
            | (ast::MapProp::Tags(_), ast::MapProp::Tags(_))
            | (ast::MapProp::Expand(_), ast::MapProp::Expand(_))
    )
}

#[cfg(feature = "wikidata")]
pub(crate) fn item_id(item: &Item) -> &str {
    match item {
        Item::Span { id, .. } => id,
        Item::Event { id, .. } => id,
        Item::EventRange { id, .. } => id,
    }
}

#[cfg(feature = "wikidata")]
pub(crate) fn merge_items_by_field_priority(
    existing: Item,
    incoming: Item,
    config: &ast::FieldPriorityConfig,
) -> Item {
    use ast::FieldStrategy;

    let pick_label = |ex: String, inc: String| match config.label {
        FieldStrategy::Manual => ex,
        FieldStrategy::Wikidata | FieldStrategy::Merge => inc,
    };
    let pick_time = |ex: i64, inc: i64| match config.time {
        FieldStrategy::Manual => ex,
        FieldStrategy::Wikidata | FieldStrategy::Merge => inc,
    };
    let merge_tags = |ex: Vec<String>, inc: Vec<String>| match config.tags {
        FieldStrategy::Manual => ex,
        FieldStrategy::Wikidata => inc,
        FieldStrategy::Merge => {
            let mut merged = ex;
            for t in inc {
                if !merged.contains(&t) {
                    merged.push(t);
                }
            }
            merged
        }
    };

    match (existing, incoming) {
        (
            Item::Span {
                id,
                lane,
                start: ex_start,
                end: ex_end,
                label: ex_label,
                tags: ex_tags,
                source,
                origin,
                start_month: ex_sm,
                start_day: ex_sd,
                end_month: ex_em,
                end_day: ex_ed,
                source_span,
            },
            Item::Span {
                start: in_start,
                end: in_end,
                label: in_label,
                tags: in_tags,
                start_month: in_sm,
                start_day: in_sd,
                end_month: in_em,
                end_day: in_ed,
                ..
            },
        ) => Item::Span {
            id,
            lane,
            start: pick_time(ex_start, in_start),
            end: pick_time(ex_end, in_end),
            label: pick_label(ex_label, in_label),
            tags: merge_tags(ex_tags, in_tags),
            source,
            origin,
            start_month: in_sm.or(ex_sm),
            start_day: in_sd.or(ex_sd),
            end_month: in_em.or(ex_em),
            end_day: in_ed.or(ex_ed),
            source_span,
        },
        (
            Item::Event {
                id,
                lane,
                time: ex_time,
                label: ex_label,
                tags: ex_tags,
                source,
                origin,
                time_month: ex_tm,
                time_day: ex_td,
                source_span,
            },
            Item::Event {
                time: in_time,
                label: in_label,
                tags: in_tags,
                time_month: in_tm,
                time_day: in_td,
                ..
            },
        ) => Item::Event {
            id,
            lane,
            time: pick_time(ex_time, in_time),
            label: pick_label(ex_label, in_label),
            tags: merge_tags(ex_tags, in_tags),
            source,
            origin,
            time_month: in_tm.or(ex_tm),
            time_day: in_td.or(ex_td),
            source_span,
        },
        (
            Item::EventRange {
                id,
                lane,
                start: ex_start,
                end: ex_end,
                label: ex_label,
                tags: ex_tags,
                source,
                origin,
                start_month: ex_sm,
                start_day: ex_sd,
                end_month: ex_em,
                end_day: ex_ed,
                source_span,
            },
            Item::EventRange {
                start: in_start,
                end: in_end,
                label: in_label,
                tags: in_tags,
                start_month: in_sm,
                start_day: in_sd,
                end_month: in_em,
                end_day: in_ed,
                ..
            },
        ) => Item::EventRange {
            id,
            lane,
            start: pick_time(ex_start, in_start),
            end: pick_time(ex_end, in_end),
            label: pick_label(ex_label, in_label),
            tags: merge_tags(ex_tags, in_tags),
            source,
            origin,
            start_month: in_sm.or(ex_sm),
            start_day: in_sd.or(ex_sd),
            end_month: in_em.or(ex_em),
            end_day: in_ed.or(ex_ed),
            source_span,
        },
        (_, incoming) => incoming,
    }
}
