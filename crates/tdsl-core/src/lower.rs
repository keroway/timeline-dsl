use std::collections::HashMap;

use tdsl_parser::ast::{self, MapTargetType};
use tdsl_wikidata::WikidataClient;
use tdsl_wikidata::entity::{DataValue, WikidataEntity, time_value_to_year};

use crate::error::LoweringError;
use crate::ir::*;

/// Lower a parsed AST into the canonical IR (static items only, no Wikidata).
pub fn lower_static(file: &ast::File) -> Result<TimelineIr, Vec<LoweringError>> {
    let mut ctx = LoweringContext::new();
    ctx.pass1_declarations(file);
    ctx.pass2_static_items(file);
    ctx.finish()
}

/// Lower a parsed AST into the canonical IR with Wikidata resolution.
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
    item_ids: HashMap<String, bool>,
    errors: Vec<LoweringError>,
    lane_auto_id: usize,

    // Import resolution state
    // import_alias -> { entity_alias -> WikidataEntity }
    import_entities: HashMap<String, HashMap<String, WikidataEntity>>,
    // import_alias -> source_type
    import_sources: HashMap<String, String>,
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
            item_ids: HashMap::new(),
            errors: Vec::new(),
            lane_auto_id: 0,
            import_entities: HashMap::new(),
            import_sources: HashMap::new(),
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
                    self.meta = Some(Meta {
                        title: t.title.clone().unwrap_or_else(|| t.name.clone()),
                        unit: t.unit.clone().unwrap_or_else(|| "year".to_string()),
                        range: t.range.as_ref().map_or((0, 2000), |r| (r.start, r.end)),
                        calendar: t
                            .calendar
                            .clone()
                            .unwrap_or_else(|| "proleptic_gregorian".to_string()),
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
                        self.errors
                            .push(LoweringError::UnknownLane(s.lane_ref.clone()));
                        continue;
                    }
                    let id = s
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("span:{}:{}", s.lane_ref, s.start));
                    self.check_dup_id(&id);
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
                        self.errors
                            .push(LoweringError::UnknownLane(e.lane_ref.clone()));
                        continue;
                    }
                    let id = e
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("event:{}:{}", e.lane_ref, e.time));
                    self.check_dup_id(&id);
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
                        self.errors
                            .push(LoweringError::UnknownLane(er.lane_ref.clone()));
                        continue;
                    }
                    let id = er
                        .props
                        .id
                        .clone()
                        .unwrap_or_else(|| format!("event_range:{}:{}", er.lane_ref, er.start));
                    self.check_dup_id(&id);
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
    ///
    /// NOTE: `import.policy` (merge_by_source / overwrite_imported / keep_manual)
    /// is parsed and stored in the AST but not yet implemented in lowering.
    /// The field is retained for forward-compatibility; behaviour is currently
    /// "insert all" regardless of the policy value.
    async fn pass3_resolve_imports(&mut self, file: &ast::File, client: &dyn WikidataClient) {
        for stmt in &file.statements {
            if let ast::Statement::Import(imp) = &stmt.node {
                let import_alias = imp.alias.clone().unwrap_or_else(|| imp.source_type.clone());
                self.import_sources
                    .insert(import_alias.clone(), imp.source_type.clone());

                let mut entities: HashMap<String, WikidataEntity> = HashMap::new();

                for item in &imp.items {
                    let ast::ImportItem::Entity { qid, alias } = item;
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

                self.import_entities.insert(import_alias, entities);
            }
        }
    }

    /// Pass 4: Apply map blocks to generate items from imported entities.
    fn pass4_apply_maps(&mut self, file: &ast::File) {
        for stmt in &file.statements {
            if let ast::Statement::Map(m) = &stmt.node {
                // Parse source_ref: "wd.han_dynasty" -> import_alias="wd", entity_key="han_dynasty"
                let parts: Vec<&str> = m.source_ref.splitn(2, '.').collect();
                if parts.len() != 2 {
                    self.errors
                        .push(LoweringError::UnresolvedImport(m.source_ref.clone()));
                    continue;
                }
                let (import_alias, entity_key) = (parts[0], parts[1]);

                let entities = match self.import_entities.get(import_alias) {
                    Some(entities) => entities,
                    None => {
                        self.errors
                            .push(LoweringError::UnresolvedImport(import_alias.to_string()));
                        continue;
                    }
                };

                match entities.get(entity_key) {
                    Some(entity) => {
                        self.apply_map_to_entity(m, &entity.clone());
                    }
                    None => {
                        self.errors.push(LoweringError::UnresolvedEntity(format!(
                            "{}.{}",
                            import_alias, entity_key
                        )));
                        continue;
                    }
                }
            }
        }
    }

    fn apply_map_to_entity(&mut self, map: &ast::MapBlock, entity: &WikidataEntity) {
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
            self.errors.push(LoweringError::UnknownMappedLane(lane_ref));
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
                    self.check_dup_id(&id);
                    self.imports.push(ImportRecord {
                        source: "wikidata".to_string(),
                        qid: entity.id.clone(),
                        mapped_to: id.clone(),
                    });
                    self.items.push(Item::Span {
                        id,
                        lane: lane_ref,
                        start: s,
                        end: e,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    });
                }
            }
            MapTargetType::Event => {
                if let Some(t) = time {
                    let id = format!("event:{}:{}", entity.id.to_lowercase(), t);
                    self.check_dup_id(&id);
                    self.imports.push(ImportRecord {
                        source: "wikidata".to_string(),
                        qid: entity.id.clone(),
                        mapped_to: id.clone(),
                    });
                    self.items.push(Item::Event {
                        id,
                        lane: lane_ref,
                        time: t,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    });
                }
            }
            MapTargetType::EventRange => {
                if let (Some(s), Some(e)) = (start, end) {
                    let id = format!("event_range:{}:{}", entity.id.to_lowercase(), s);
                    self.check_dup_id(&id);
                    self.imports.push(ImportRecord {
                        source: "wikidata".to_string(),
                        qid: entity.id.clone(),
                        mapped_to: id.clone(),
                    });
                    self.items.push(Item::EventRange {
                        id,
                        lane: lane_ref,
                        start: s,
                        end: e,
                        label,
                        tags,
                        source: item_source,
                        origin,
                    });
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

    fn check_dup_id(&mut self, id: &str) {
        if self.item_ids.contains_key(id) {
            self.errors
                .push(LoweringError::DuplicateItemId(id.to_string()));
        } else {
            self.item_ids.insert(id.to_string(), true);
        }
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
fn eval_map_expr(expr: &ast::MapExpr, entity: &WikidataEntity) -> Option<i64> {
    let dv = entity.claim(&expr.claim.property)?;
    match dv {
        DataValue::Time { value } => {
            if expr.accessor.as_deref() == Some("year") {
                time_value_to_year(value).ok()
            } else {
                time_value_to_year(value).ok()
            }
        }
        _ => None,
    }
}

/// Evaluate a label expression with fallback (e.g. `label@ja ?? label@en`).
fn eval_label_expr(expr: &ast::LabelExpr, entity: &WikidataEntity) -> Option<String> {
    let langs: Vec<&str> = expr.fallbacks.iter().map(|lr| lr.lang.as_str()).collect();
    entity.label_with_fallback(&langs).map(|s| s.to_string())
}

// ─── Helpers ────────────────────────────────────────────────

fn source_str(sr: &Option<ast::SourceRef>) -> Option<String> {
    sr.as_ref().map(|s| format!("{}:{}", s.prefix, s.qid))
}

fn slug(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase()
}
