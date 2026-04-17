use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::entity::WikidataEntity;
use crate::error::WikidataError;

/// Trait for fetching data from Wikidata. Implementations can be swapped for testing.
#[async_trait]
pub trait WikidataClient: Send + Sync {
    /// Fetch a single entity by QID.
    async fn get_entity(&self, qid: &str, langs: &[&str]) -> Result<WikidataEntity, WikidataError>;

    /// Search entities by a free-text query.
    async fn search_entities(
        &self,
        query: &str,
        lang: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, WikidataError>;

    /// Execute a SPARQL query and return entity QIDs from the result.
    async fn sparql_query(&self, query: &str) -> Result<Vec<String>, WikidataError>;
}

/// One search hit from Wikidata's wbsearchentities API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// HTTP-based Wikidata client using the public API.
pub struct HttpWikidataClient {
    http: reqwest::Client,
}

impl HttpWikidataClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("tdsl/0.1.0 (https://github.com/keroway/timeline-dsl)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create HTTP client");
        Self { http }
    }
}

/// Raw response from wbgetentities API.
#[derive(Deserialize)]
struct WbGetEntitiesResponse {
    #[serde(default)]
    entities: HashMap<String, WikidataEntity>,
}

/// Raw response from wbsearchentities API.
#[derive(Deserialize)]
struct WbSearchResponse {
    #[serde(default)]
    search: Vec<SearchResult>,
}

/// SPARQL query response.
#[derive(Deserialize)]
struct SparqlResponse {
    results: SparqlBindings,
}

#[derive(Deserialize)]
struct SparqlBindings {
    bindings: Vec<HashMap<String, SparqlValue>>,
}

#[derive(Deserialize)]
struct SparqlValue {
    value: String,
}

#[async_trait]
impl WikidataClient for HttpWikidataClient {
    async fn get_entity(&self, qid: &str, langs: &[&str]) -> Result<WikidataEntity, WikidataError> {
        let languages = langs.join("|");
        let url = format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&ids={qid}&languages={languages}&format=json"
        );

        let resp: WbGetEntitiesResponse = self
            .http
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        resp.entities
            .into_values()
            .next()
            .ok_or_else(|| WikidataError::NotFound(qid.to_string()))
    }

    async fn search_entities(
        &self,
        query: &str,
        lang: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, WikidataError> {
        let max_limit = limit.clamp(1, 50);
        let resp: WbSearchResponse = self
            .http
            .get("https://www.wikidata.org/w/api.php")
            .query(&[
                ("action", "wbsearchentities"),
                ("format", "json"),
                ("type", "item"),
                ("language", lang),
                ("search", query),
                ("limit", &max_limit.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(resp.search)
    }

    async fn sparql_query(&self, query: &str) -> Result<Vec<String>, WikidataError> {
        let resp: SparqlResponse = self
            .http
            .get("https://query.wikidata.org/sparql")
            .query(&[("query", query), ("format", "json")])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let qids = resp
            .results
            .bindings
            .into_iter()
            .filter_map(|row| {
                // Extract QID from the first URI-typed binding (e.g. "http://www.wikidata.org/entity/Q42")
                row.into_values().next().and_then(|v| {
                    v.value
                        .rsplit('/')
                        .next()
                        .filter(|s| s.starts_with('Q'))
                        .map(String::from)
                })
            })
            .collect();

        Ok(qids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_search_response() {
        let payload = r#"
        {
          "search": [
            {
              "id": "Q7209",
              "label": "漢",
              "description": "中国の王朝",
              "aliases": ["漢王朝", "前漢後漢"]
            },
            {
              "id": "Q7183",
              "label": "秦",
              "description": "中国の王朝"
            }
          ]
        }"#;
        let parsed: WbSearchResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.search.len(), 2);
        assert_eq!(parsed.search[0].id, "Q7209");
        assert_eq!(parsed.search[0].aliases.len(), 2);
        assert_eq!(parsed.search[1].id, "Q7183");
        assert!(parsed.search[1].aliases.is_empty());
    }
}
