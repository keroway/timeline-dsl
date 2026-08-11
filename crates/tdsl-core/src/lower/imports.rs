#[cfg(feature = "wikidata")]
use std::collections::HashMap;

#[cfg(feature = "wikidata")]
use tdsl_parser::ast;
#[cfg(feature = "wikidata")]
use tdsl_wikidata::WikidataClient;

#[cfg(feature = "wikidata")]
use crate::error::LoweringError;

#[cfg(feature = "wikidata")]
use super::context::LoweringContext;

#[cfg(feature = "wikidata")]
const MAX_IMPORT_QUERY_RESULTS: usize = 50;

#[cfg(feature = "wikidata")]
impl LoweringContext {
    /// Pass 3: Resolve import blocks by fetching entities from Wikidata.
    pub(crate) async fn pass3_resolve_imports(
        &mut self,
        file: &ast::File,
        client: &dyn WikidataClient,
    ) {
        for stmt in &file.statements {
            // エラーに添える位置。push_error() がこれを読む（#760）。
            self.current_span = Some(stmt.span);
            if let ast::Statement::Import(imp) = &stmt.node {
                let import_alias = imp.alias.clone().unwrap_or_else(|| imp.source_type.clone());
                self.import_sources
                    .insert(import_alias.clone(), imp.source_type.clone());
                self.import_policies.insert(
                    import_alias.clone(),
                    imp.policy.unwrap_or(ast::ReimportPolicy::MergeBySource),
                );

                let mut entities = HashMap::new();
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
                                    self.push_error(LoweringError::Wikidata(e));
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
                                                self.push_error(LoweringError::Wikidata(e));
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
                                    self.push_error(LoweringError::Wikidata(e));
                                }
                            }
                        }
                    }
                }

                self.import_entities.insert(import_alias.clone(), entities);
                self.import_groups.insert(import_alias, groups);
            }
        }
        // ループを抜けたら位置を捨てる。以降のエラー（NoTimeline 等、
        // ファイル全体に対するもの）に直前 statement の位置を
        // 添えてしまわないため（#760）。
        self.current_span = None;
    }
}
