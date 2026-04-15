use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;

use crate::entity::WikidataEntity;
use crate::error::WikidataError;

/// Trait for fetching data from Wikidata. Implementations can be swapped for testing.
#[async_trait]
pub trait WikidataClient: Send + Sync {
    /// Fetch a single entity by QID.
    async fn get_entity(&self, qid: &str, langs: &[&str]) -> Result<WikidataEntity, WikidataError>;

    /// Execute a SPARQL query and return entity QIDs from the result.
    async fn sparql_query(&self, query: &str) -> Result<Vec<String>, WikidataError>;
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
