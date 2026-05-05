use std::collections::HashMap;

use tdsl_parser::ast;
#[cfg(feature = "wikidata")]
use tdsl_parser::ast::MapTargetType;
#[cfg(feature = "wikidata")]
use tdsl_wikidata::WikidataClient;
#[cfg(feature = "wikidata")]
use tdsl_wikidata::entity::{DataValue, WikidataEntity, time_value_to_year};

use crate::error::LoweringError;
use crate::ir::*;

#[cfg(feature = "wikidata")]
const MAX_IMPORT_QUERY_RESULTS: usize = 50;

/// Lower a parsed AST into the canonical IR (static items only, no Wikidata).
pub fn lower_static(file: &ast::File) -> Result<TimelineIr, Vec<LoweringError>> {
    let mut ctx = LoweringContext::new();
    ctx.pass1_declarations(file);
    ctx.pass2_static_items(file);
    ctx.finish()
}

/// Lower a parsed AST into the canonical IR with Wikidata resolution.
#[cfg(feature = "wikidata")]
pub async fn lower_with_wikidata(
    file: &ast::File,
    client: &dyn WikidataClient,
) -> Result<TimelineIr, Vec<LoweringError>> {
    let mut ctx = LoweringContext::new();
    ctx.pass1_declarations(file);
    ctx.pass2_static_items(file);
    ctx.pass3_resolve_imports(file, client).await;
    ctx.pass4_apply_maps(file);
    ctx.finish()
}

struct LoweringContext {
    meta: Option<Meta>,
    lanes_map: HashMap<String, Lane>,
    lane_order: Vec<String>,
    items: Vec<Item>,
    imports: Vec<ImportRecord>,
    sources: Vec<SourceRecord>,
    item_index_by_id: HashMap<String, usize>,
    #[cfg(feature = "wikidata")]
    item_imported_by_id: HashMap<String, bool>,
    #[cfg(feature = "wikidata")]
    import_record_index_by_item_id: HashMap<String, usize>,
    errors: Vec<LoweringError>,
    lane_auto_id: usize,

    // Import resolution state
    // import_alias -> { entity_alias -> WikidataEntity }
    #[cfg(feature = "wikidata")]
    import_entities: HashMap<String, HashMap<String, WikidataEntity>>,
    // import_alias -> { query_alias -> [entity_alias] }
    #[cfg(feature = "wikidata")]
    import_groups: HashMap<String, HashMap<String, Vec<String>>>,
    // import_alias -> source_type
    #[cfg(feature = "wikidata")]
    import_sources: HashMap<String, String>,
    // import_alias -> import policy
    #[cfg(feature = "wikidata")]
    import_policies: HashMap<String, ast::ReimportPolicy>,

    // Template registry
    // template_alias -> TemplateBlock
    templates: HashMap<String, ast::TemplateBlock>,
}

impl LoweringContext {
    fn new() -> Self {
        Self {
            meta: None,
            lanes_map: HashMap::new(),
            lane_order: Vec::new(),
            items: Vec::new(),
            imports: Vec::new(),
            sources: Vec::new(),
            item_index_by_id: HashMap::new(),
            #[cfg(feature = "wikidata")]
            item_imported_by_id: HashMap::new(),
            #[cfg(feature = "wikidata")]
            import_record_index_by_item_id: HashMap::new(),
            errors: Vec::new(),
            lane_auto_id: 0,
            #[cfg(feature = "wikidata")]
            import_entities: HashMap::new(),
            #[cfg(feature = "wikidata")]
            import_groups: HashMap::new(),
            #[cfg(feature = "wikidata")]
            import_sources: HashMap::new(),
            #[cfg(feature = "wikidata")]
            import_policies: HashMap::new(),
            templates: HashMap::new(),
        }
    }

    /// Pass 1: Collect timeline meta and lane declarations.
    fn pass1_declarations(&mut self, file: &ast::File) {
        for stmt in &file.statements {
            match &stmt.node {
                ast::Statement::Timeline(t) => {
                    if self.meta.is_some() {
                        self.errors.push(LoweringError::MultipleTimelines);
                        continue;
                    }
                    let color_map = t
                        .color_map
                        .iter()
                        .cloned()
                        .collect::<std::collections::HashMap<_, _>>();
                    self.meta = Some(Meta {
                        title: t.title.clone().unwrap_or_else(|| t.name.clone()),
                        unit: t.unit.clone().unwrap_or_else(|| "year".to_string()),
                        range: t.range.as_ref().map_or((0, 2000), |r| (r.start, r.end)),
                        calendar: t
                            .calendar
                            .clone()
                            .unwrap_or_else(|| "proleptic_gregorian".to_string()),
                        color_map,
                    });
                }
                ast::Statement::Lane(l) => {
                    let id = l.alias.clone().unwrap_or_else(|| {
                        let s = slug(&l.label);
                        if s.is_empty() {
                            let auto = format!("lane_{}", self.lane_auto_id);
                            self.lane_auto_id += 1;
                            auto
                        } else {
                            s
                        }
                    });
                    if self.lanes_map.contains_key(&id) {
                        self.errors.push(LoweringError::DuplicateLane(id.clone()));
                        continue;
                    }
                    let lane = Lane {
                        id: id.clone(),
                        label: l.label.clone(),
                        kind: l.kind.clone().unwrap_or_else(|| "custom".to_string()),
                        order: l.order.unwrap_or(0),
                    };
                    self.lane_order.push(id.clone());
                    self.lanes_map.insert(id, lane);
                }
                ast::Statement::Template(t) => {
                    let key = t.alias.clone().unwrap_or_else(|| t.name.clone());
                    if self.templates.contains_key(&key) {
                        self.errors
                            .push(LoweringError::DuplicateTemplate(key.clone()));
                        continue;
                    }
                    self.templates.insert(key, t.clone());
                }
                _ => {}
            }
        }
        if self.meta.is_none() {
            self.errors.push(LoweringError::NoTimeline);
        }
    }

    /// Pass 2: Lower static items (span, event, event_range).
    fn pass2_static_items(&mut self, file: &ast::File) {
        for stmt in &file.statements {
            match &stmt.node {
                ast::Statement::Span(s) => {
                    if !self.lanes_map.contains_key(&s.lane_ref) {
                        let err = self.make_unknown_lane_error(&s.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = s
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("span:{}:{}", s.lane_ref, s.start));
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&s.props.source);
                    self.items.push(Item::Span {
                        id,
                        lane: s.lane_ref.clone(),
                        start: s.start,
                        end: s.end,
                        label: s.label.clone(),
                        tags: s.props.tags.clone(),
                        source: source_str(&s.props.source),
                        origin: s.props.origin.clone(),
                    });
                }
                ast::Statement::Event(e) => {
                    if !self.lanes_map.contains_key(&e.lane_ref) {
                        let err = self.make_unknown_lane_error(&e.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = e
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("event:{}:{}", e.lane_ref, e.time));
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&e.props.source);
                    self.items.push(Item::Event {
                        id,
                        lane: e.lane_ref.clone(),
                        time: e.time,
                        label: e.label.clone(),
                        tags: e.props.tags.clone(),
                        source: source_str(&e.props.source),
                        origin: e.props.origin.clone(),
                    });
                }
                ast::Statement::EventRange(er) => {
                    if !self.lanes_map.contains_key(&er.lane_ref) {
                        let err = self.make_unknown_lane_error(&er.lane_ref);
                        self.errors.push(err);
                        continue;
                    }
                    let id = er
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("event_range:{}:{}", er.lane_ref, er.start));
                    if !self.register_static_id(&id) {
                        continue;
                    }
                    self.add_source_from_ref(&er.props.source);
                    self.items.push(Item::EventRange {
                        id,
                        lane: er.lane_ref.clone(),
                        start: er.start,
                        end: er.end,
                        label: er.label.clone(),
                        tags: er.props.tags.clone(),
                        source: source_str(&er.props.source),
                        origin: er.props.origin.clone(),
                    });
                }
                _ => {}
            }
        }
    }

    /// Pass 3: Resolve import blocks by fetching entities from Wikidata.
    #[cfg(feature = "wikidata")]
    async fn pass3_resolve_imports(&mut self, file: &ast::File, client: &dyn WikidataClient) {
        for stmt in &file.statements {
            if let ast::Statement::Import(imp) = &stmt.node {
                let import_alias = imp.alias.clone().unwrap_or_else(|| imp.source_type.clone());
                self.import_sources
                    .insert(import_alias.clone(), imp.source_type.clone());
                self.import_policies.insert(
                    import_alias.clone(),
                    imp.policy.unwrap_or(ast::ReimportPolicy::MergeBySource),
                );

                let mut entities: HashMap<String, WikidataEntity> = HashMap::new();
                let mut groups: HashMap<String, Vec<String>> = HashMap::new();
                let mut unnamed_query_index = 0usize;

                for item in &imp.items {
                    match item {
                        ast::ImportItem::Entity { qid, alias } => {
                            match client.get_entity(qid, &["ja", "en"]).await {
                                Ok(entity) => {
                                    let key = alias.clone().unwrap_or_else(|| qid.to_lowercase());
                                    entities.insert(key, entity);
                                }
                                Err(e) => {
                                    self.errors.push(LoweringError::Wikidata(e));
                                }
                            }
                        }
                        ast::ImportItem::Query { query, alias } => {
                            match client.sparql_query(query).await {
                                Ok(mut qids) => {
                                    qids.sort();
                                    qids.dedup();
                                    if qids.len() > MAX_IMPORT_QUERY_RESULTS {
                                        qids.truncate(MAX_IMPORT_QUERY_RESULTS);
                                    }

                                    let mut group_keys = Vec::new();
                                    for qid in qids {
                                        match client.get_entity(&qid, &["ja", "en"]).await {
                                            Ok(entity) => {
                                                let key = qid.to_lowercase();
                                                group_keys.push(key.clone());
                                                entities.insert(key, entity);
                                            }
                                            Err(e) => {
                                                self.errors.push(LoweringError::Wikidata(e));
                                            }
                                        }
                                    }

                                    if let Some(group_alias) = alias.clone().or_else(|| {
                                        let name = format!("query_{unnamed_query_index}");
                                        unnamed_query_index += 1;
                                        Some(name)
                                    }) {
                                        groups.insert(group_alias, group_keys);
                                    }
                                }
                                Err(e) => {
                                    self.errors.push(LoweringError::Wikidata(e));
                                }
                            }
                        }
                    }
                }

                self.import_entities.insert(import_alias.clone(), entities);
                self.import_groups.insert(import_alias, groups);
            }
        }
    }

    /// Pass 4: Apply map blocks and apply blocks to generate items from imported entities.
    #[cfg(feature = "wikidata")]
    fn pass4_apply_maps(&mut self, file: &ast::File) {
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

                if let Some(entity_groups) = self.import_groups.get(import_alias) {
                    if let Some(keys) = entity_groups.get(entity_key) {
                        let mapped_entities: Vec<WikidataEntity> = keys
                            .iter()
                            .filter_map(|k| entities.get(k).cloned())
                            .collect();
                        for entity in &mapped_entities {
                            self.apply_map_to_entity(m, entity, policy);
                        }
                        continue;
                    }
                }

                self.errors.push(LoweringError::UnresolvedEntity(format!(
                    "{}.{}",
                    import_alias, entity_key
                )));
            }
        }
    }

    #[cfg(feature = "wikidata")]
    fn process_apply_block(&mut self, apply: &ast::ApplyBlock) {
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

        // Apply to every entity in the import
        for entity in entities.values() {
            self.apply_map_to_entity(&synthetic_map, entity, policy);
        }

        // Also apply to query groups (apply all entities in each group)
        if let Some(groups) = self.import_groups.get(&apply.import_alias).cloned() {
            let all_entities = self.import_entities[&apply.import_alias].clone();
            for keys in groups.values() {
                for key in keys {
                    if let Some(entity) = all_entities.get(key) {
                        self.apply_map_to_entity(&synthetic_map, entity, policy);
                    }
                }
            }
        }
    }

    #[cfg(feature = "wikidata")]
    fn apply_map_to_entity(
        &mut self,
        map: &ast::MapBlock,
        entity: &WikidataEntity,
        policy: ast::ReimportPolicy,
    ) {
        let mut lane_ref = String::new();
        let mut start: Option<i64> = None;
        let mut end: Option<i64> = None;
        let mut time: Option<i64> = None;
        let mut label = String::new();
        let mut tags = Vec::new();

        for prop in &map.props {
            match prop {
                ast::MapProp::Lane(l) => lane_ref = l.clone(),
                ast::MapProp::Start(expr) => start = eval_map_expr(expr, entity),
                ast::MapProp::End(expr) => end = eval_map_expr(expr, entity),
                ast::MapProp::Time(expr) => time = eval_map_expr(expr, entity),
                ast::MapProp::Label(lexpr) => {
                    label = eval_label_expr(lexpr, entity).unwrap_or_default();
                }
                ast::MapProp::Tags(t) => tags = t.clone(),
            }
        }

        // Validate lane existence
        if !lane_ref.is_empty() && !self.lanes_map.contains_key(&lane_ref) {
            let err = self.make_unknown_mapped_lane_error(&lane_ref);
            self.errors.push(err);
            return;
        }

        if lane_ref.is_empty() || label.is_empty() {
            return;
        }

        let source_id = format!("wd:{}", entity.id);
        self.sources.push(SourceRecord {
            id: source_id.clone(),
            provider: "wikidata".to_string(),
            license: "CC0".to_string(),
        });

        let item_source = Some(source_id.clone());
        let origin = Some("wikidata".to_string());

        match map.target_type {
            MapTargetType::Span => {
                if let (Some(s), Some(e)) = (start, end) {
                    let id = format!("span:{}:{}", entity.id.to_lowercase(), s);
                    let item = Item::Span {
                        id,
                        lane: lane_ref,
                        start: s,
                        end: e,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                }
            }
            MapTargetType::Event => {
                if let Some(t) = time {
                    let id = format!("event:{}:{}", entity.id.to_lowercase(), t);
                    let item = Item::Event {
                        id,
                        lane: lane_ref,
                        time: t,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                }
            }
            MapTargetType::EventRange => {
                if let (Some(s), Some(e)) = (start, end) {
                    let id = format!("event_range:{}:{}", entity.id.to_lowercase(), s);
                    let item = Item::EventRange {
                        id,
                        lane: lane_ref,
                        start: s,
                        end: e,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    };
                    self.insert_imported_item(item, &entity.id, policy);
                }
            }
        }
    }

    fn finish(mut self) -> Result<TimelineIr, Vec<LoweringError>> {
        if !self.errors.is_empty() {
            return Err(self.errors);
        }

        let lanes: Vec<Lane> = self
            .lane_order
            .iter()
            .filter_map(|id| self.lanes_map.remove(id))
            .collect();

        // Deduplicate sources
        let mut seen = std::collections::HashSet::new();
        self.sources.retain(|s| seen.insert(s.id.clone()));

        Ok(TimelineIr {
            meta: self.meta.unwrap(),
            lanes,
            items: self.items,
            imports: self.imports,
            sources: self.sources,
        })
    }

    fn register_static_id(&mut self, id: &str) -> bool {
        if self.item_index_by_id.contains_key(id) {
            self.errors
                .push(LoweringError::DuplicateItemId(id.to_string()));
            return false;
        }
        let idx = self.items.len();
        self.item_index_by_id.insert(id.to_string(), idx);
        #[cfg(feature = "wikidata")]
        self.item_imported_by_id.insert(id.to_string(), false);
        true
    }

    #[cfg(feature = "wikidata")]
    fn insert_imported_item(&mut self, item: Item, qid: &str, policy: ast::ReimportPolicy) {
        let id = item_id(&item).to_string();

        if let Some(idx) = self.item_index_by_id.get(&id).copied() {
            match policy {
                ast::ReimportPolicy::MergeBySource => {
                    self.errors.push(LoweringError::DuplicateItemId(id));
                }
                ast::ReimportPolicy::KeepManual => {
                    // Keep existing item when IDs conflict.
                }
                ast::ReimportPolicy::OverwriteImported => {
                    let existing_imported =
                        self.item_imported_by_id.get(&id).copied().unwrap_or(false);
                    if existing_imported {
                        self.items[idx] = item;
                        self.upsert_import_record(&id, qid);
                    } else {
                        self.errors.push(LoweringError::DuplicateItemId(id));
                    }
                }
                ast::ReimportPolicy::FieldPriority(config) => {
                    let existing = self.items[idx].clone();
                    let merged = merge_items_by_field_priority(existing, item, &config);
                    self.items[idx] = merged;
                    self.upsert_import_record(&id, qid);
                }
            }
        } else {
            let idx = self.items.len();
            self.items.push(item);
            self.item_index_by_id.insert(id.clone(), idx);
            self.item_imported_by_id.insert(id.clone(), true);
            self.upsert_import_record(&id, qid);
        }
    }

    #[cfg(feature = "wikidata")]
    fn upsert_import_record(&mut self, item_id: &str, qid: &str) {
        if let Some(idx) = self.import_record_index_by_item_id.get(item_id).copied() {
            self.imports[idx] = ImportRecord {
                source: "wikidata".to_string(),
                qid: qid.to_string(),
                mapped_to: item_id.to_string(),
            };
        } else {
            let idx = self.imports.len();
            self.imports.push(ImportRecord {
                source: "wikidata".to_string(),
                qid: qid.to_string(),
                mapped_to: item_id.to_string(),
            });
            self.import_record_index_by_item_id
                .insert(item_id.to_string(), idx);
        }
    }

    /// Build an UnknownLane error with suggestions from currently-declared lanes.
    fn make_unknown_lane_error(&self, lane_ref: &str) -> LoweringError {
        let available = self.lane_order.clone();
        let hint = lane_suggestion_hint(lane_ref, &available);
        LoweringError::UnknownLane(format!("'{lane_ref}' — {hint}"))
    }

    /// Build an UnknownMappedLane error with suggestions.
    #[cfg(feature = "wikidata")]
    fn make_unknown_mapped_lane_error(&self, lane_ref: &str) -> LoweringError {
        let available = self.lane_order.clone();
        let hint = lane_suggestion_hint(lane_ref, &available);
        LoweringError::UnknownMappedLane(format!("'{lane_ref}' — {hint}"))
    }

    fn add_source_from_ref(&mut self, sr: &Option<ast::SourceRef>) {
        if let Some(sr) = sr {
            self.sources.push(SourceRecord {
                id: format!("{}:{}", sr.prefix, sr.qid),
                provider: sr.prefix.clone(),
                license: if sr.prefix == "wd" {
                    "CC0".to_string()
                } else {
                    "unknown".to_string()
                },
            });
        }
    }
}

// ─── Expression Evaluation ──────────────────────────────────

/// Evaluate a map expression (e.g. `claim(P571).year`) against a Wikidata entity.
#[cfg(feature = "wikidata")]
fn eval_map_expr(expr: &ast::MapExpr, entity: &WikidataEntity) -> Option<i64> {
    let dv = entity.claim(&expr.claim.property)?;
    match dv {
        DataValue::Time { value } => match expr.accessor.as_deref() {
            Some("year") | None => time_value_to_year(value).ok(),
            _ => None,
        },
        _ => None,
    }
}

/// Evaluate a label expression with fallback (e.g. `label@ja ?? label@en`).
#[cfg(feature = "wikidata")]
fn eval_label_expr(expr: &ast::LabelExpr, entity: &WikidataEntity) -> Option<String> {
    let langs: Vec<&str> = expr.fallbacks.iter().map(|lr| lr.lang.as_str()).collect();
    entity.label_with_fallback(&langs).map(|s| s.to_string())
}

// ─── Helpers ────────────────────────────────────────────────

/// Merge template props with apply overrides.
/// Override props of the same variant replace the template props.
#[cfg(feature = "wikidata")]
fn merge_map_props(base: &[ast::MapProp], overrides: &[ast::MapProp]) -> Vec<ast::MapProp> {
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
    )
}

/// Build a human-readable hint for an unknown lane reference.
/// Shows similar-looking candidates (prefix match or substring) first, then all available.
fn lane_suggestion_hint(unknown: &str, available: &[String]) -> String {
    if available.is_empty() {
        return "定義済みのlaneがありません。先にlane宣言を追加してください".to_string();
    }

    // Find candidates that share a common prefix (>=2 chars), or contain/are contained by unknown
    let u_lower = unknown.to_lowercase();
    let similar: Vec<&str> = available
        .iter()
        .filter(|candidate| {
            let c = candidate.to_lowercase();
            let prefix_len = u_lower.len().min(2);
            c.starts_with(&u_lower[..prefix_len]) || c.contains(&u_lower) || u_lower.contains(c.as_str())
        })
        .map(|s| s.as_str())
        .collect();

    let all: Vec<&str> = available.iter().map(|s| s.as_str()).collect();

    if !similar.is_empty() && similar != all {
        format!(
            "もしかして: {} ？（利用可能なlane: {}）",
            similar.join(", "),
            all.join(", ")
        )
    } else {
        format!("利用可能なlane: {}", all.join(", "))
    }
}

fn source_str(sr: &Option<ast::SourceRef>) -> Option<String> {
    sr.as_ref().map(|s| format!("{}:{}", s.prefix, s.qid))
}

#[cfg(feature = "wikidata")]
fn item_id(item: &Item) -> &str {
    match item {
        Item::Span { id, .. } => id,
        Item::Event { id, .. } => id,
        Item::EventRange { id, .. } => id,
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

#[cfg(feature = "wikidata")]
fn merge_items_by_field_priority(
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
            },
            Item::Span {
                start: in_start,
                end: in_end,
                label: in_label,
                tags: in_tags,
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
            },
            Item::Event {
                time: in_time,
                label: in_label,
                tags: in_tags,
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
            },
            Item::EventRange {
                start: in_start,
                end: in_end,
                label: in_label,
                tags: in_tags,
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
        },
        (_, incoming) => incoming,
    }
}
