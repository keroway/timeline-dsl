use std::collections::HashMap;

use tdsl_parser::ast;
#[cfg(feature = "wikidata")]
use tdsl_wikidata::entity::WikidataEntity;

use crate::error::LoweringError;
use crate::ir::*;

use super::{lane_suggestion_hint, offset_to_line_col, slug};

pub(crate) struct LoweringContext {
    pub(crate) meta: Option<Meta>,
    pub(crate) lanes_map: HashMap<String, Lane>,
    pub(crate) lane_order: Vec<String>,
    pub(crate) items: Vec<Item>,
    pub(crate) imports: Vec<ImportRecord>,
    pub(crate) sources: Vec<SourceRecord>,
    pub(crate) item_index_by_id: HashMap<String, usize>,
    #[cfg(feature = "wikidata")]
    pub(crate) item_imported_by_id: HashMap<String, bool>,
    #[cfg(feature = "wikidata")]
    pub(crate) import_record_index_by_item_id: HashMap<String, usize>,
    pub(crate) errors: Vec<LoweringError>,
    pub(crate) lane_auto_id: usize,

    // Import resolution state
    // import_alias -> { entity_alias -> WikidataEntity }
    #[cfg(feature = "wikidata")]
    pub(crate) import_entities: HashMap<String, HashMap<String, WikidataEntity>>,
    // import_alias -> { query_alias -> [entity_alias] }
    #[cfg(feature = "wikidata")]
    pub(crate) import_groups: HashMap<String, HashMap<String, Vec<String>>>,
    // import_alias -> source_type
    #[cfg(feature = "wikidata")]
    pub(crate) import_sources: HashMap<String, String>,
    // import_alias -> import policy
    #[cfg(feature = "wikidata")]
    pub(crate) import_policies: HashMap<String, ast::ReimportPolicy>,

    // Template registry
    // template_alias -> TemplateBlock
    pub(crate) templates: HashMap<String, ast::TemplateBlock>,
}

impl LoweringContext {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn finish(mut self) -> Result<TimelineIr, Vec<LoweringError>> {
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

        let meta = self.meta.ok_or_else(|| vec![LoweringError::NoTimeline])?;

        Ok(TimelineIr {
            meta,
            lanes,
            items: self.items,
            imports: self.imports,
            sources: self.sources,
        })
    }

    pub(crate) fn register_static_id(&mut self, id: &str) -> bool {
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
    pub(crate) fn insert_imported_item(
        &mut self,
        item: Item,
        qid: &str,
        policy: ast::ReimportPolicy,
    ) {
        use super::mapping::item_id;
        use super::mapping::merge_items_by_field_priority;

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
    pub(crate) fn upsert_import_record(&mut self, item_id: &str, qid: &str) {
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

    pub(crate) fn lower_lane_decl(
        &mut self,
        l: &ast::LaneDecl,
        group: Option<&str>,
        stmt_span: &ast::Span,
        line_offsets: Option<&[usize]>,
    ) {
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
            return;
        }
        let source_span = line_offsets.map(|lo| {
            let (line, col_start) = offset_to_line_col(stmt_span.start, lo);
            let (_, col_end) = offset_to_line_col(stmt_span.end, lo);
            SourceSpan {
                line,
                col_start,
                col_end,
            }
        });
        let lane = Lane {
            id: id.clone(),
            label: l.label.clone(),
            kind: l.kind.clone().unwrap_or_else(|| "custom".to_string()),
            order: l.order.unwrap_or(0),
            group: group.map(|s| s.to_string()),
            source_span,
        };
        self.lane_order.push(id.clone());
        self.lanes_map.insert(id, lane);
    }

    /// Build an UnknownLane error with suggestions from currently-declared lanes.
    pub(crate) fn make_unknown_lane_error(&self, lane_ref: &str) -> LoweringError {
        let available = self.lane_order.clone();
        let hint = lane_suggestion_hint(lane_ref, &available);
        LoweringError::UnknownLane(format!("'{lane_ref}' — {hint}"))
    }

    /// Build an UnknownMappedLane error with suggestions.
    #[cfg(feature = "wikidata")]
    pub(crate) fn make_unknown_mapped_lane_error(&self, lane_ref: &str) -> LoweringError {
        let available = self.lane_order.clone();
        let hint = lane_suggestion_hint(lane_ref, &available);
        LoweringError::UnknownMappedLane(format!("'{lane_ref}' — {hint}"))
    }

    pub(crate) fn add_source_from_ref(&mut self, sr: &Option<ast::SourceRef>) {
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
