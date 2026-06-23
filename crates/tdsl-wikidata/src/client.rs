use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::entity::WikidataEntity;
use crate::error::WikidataError;

/// Parsed Wikipedia page reference suitable for Wikidata sitelink resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikipediaPageRef {
    pub site: String,
    pub title: String,
}

/// Trait for fetching data from Wikidata. Implementations can be swapped for testing.
#[async_trait]
pub trait WikidataClient: Send + Sync {
    /// Fetch a single entity by QID.
    async fn get_entity(&self, qid: &str, langs: &[&str]) -> Result<WikidataEntity, WikidataError>;

    /// Fetch a single entity by sitelink (e.g. site=`jawiki`, title=`漢`).
    async fn get_entity_by_sitelink(
        &self,
        site: &str,
        title: &str,
        langs: &[&str],
    ) -> Result<WikidataEntity, WikidataError>;

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

/// Default maximum number of retry attempts for transient errors.
pub const DEFAULT_MAX_RETRIES: u32 = 5;

/// HTTP-based Wikidata client using the public API.
pub struct HttpWikidataClient {
    http: reqwest::Client,
    max_retries: u32,
}

impl HttpWikidataClient {
    pub fn new() -> Self {
        Self::with_timeout(std::time::Duration::from_secs(30))
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self::with_options(timeout, DEFAULT_MAX_RETRIES)
    }

    pub fn with_options(timeout: std::time::Duration, max_retries: u32) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "tdsl/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/keroway/timeline-dsl)"
            ))
            .timeout(timeout)
            .build()
            .expect("failed to create HTTP client");
        Self { http, max_retries }
    }
}

impl Default for HttpWikidataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpWikidataClient {
    /// Send an HTTP GET request with exponential backoff retry on 429 and 5xx errors.
    async fn send_with_retry(
        &self,
        make_req: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, WikidataError> {
        let mut attempt = 0u32;
        loop {
            match make_req().send().await {
                Err(e) => {
                    if e.is_timeout() {
                        return Err(WikidataError::Timeout);
                    }
                    if attempt < self.max_retries && e.is_connect() {
                        tokio::time::sleep(Duration::from_secs(1u64 << attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(WikidataError::Http(e));
                }
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        return Ok(resp);
                    }
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        if attempt >= self.max_retries {
                            return Err(WikidataError::RateLimit);
                        }
                        let wait = resp
                            .headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(1u64 << attempt);
                        tokio::time::sleep(Duration::from_secs(wait)).await;
                        attempt += 1;
                        continue;
                    }
                    if status.is_server_error() && attempt < self.max_retries {
                        tokio::time::sleep(Duration::from_secs(1u64 << attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return match resp.error_for_status() {
                        Err(e) => Err(WikidataError::Http(e)),
                        Ok(_) => unreachable!("non-success status should be an error"),
                    };
                }
            }
        }
    }
}

/// Parse a Wikipedia article URL into Wikidata sitelink parameters.
///
/// Supported examples:
/// - `https://ja.wikipedia.org/wiki/漢`
/// - `https://en.wikipedia.org/wiki/Han_dynasty`
/// - `https://ja.wikipedia.org/w/index.php?title=漢`
pub fn parse_wikipedia_url(input: &str) -> Result<WikipediaPageRef, WikidataError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(WikidataError::InvalidInput(
            "wikipedia URL must not be empty".to_string(),
        ));
    }

    let url = Url::parse(raw)
        .map_err(|e| WikidataError::InvalidInput(format!("invalid URL: {raw} ({e})")))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(WikidataError::InvalidInput(format!(
                "unsupported URL scheme: {other}"
            )));
        }
    }

    let host = url
        .host_str()
        .ok_or_else(|| WikidataError::InvalidInput("missing host".to_string()))?
        .to_ascii_lowercase();
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() < 3 || parts[parts.len() - 2] != "wikipedia" || parts[parts.len() - 1] != "org" {
        return Err(WikidataError::InvalidInput(format!(
            "unsupported host (expected *.wikipedia.org): {host}"
        )));
    }

    let lang = parts
        .first()
        .copied()
        .filter(|x| !x.is_empty())
        .ok_or_else(|| WikidataError::InvalidInput(format!("invalid wikipedia host: {host}")))?;
    let site = format!("{lang}wiki");

    let mut title = if let Some(rest) = url.path().strip_prefix("/wiki/") {
        decode_url_component(rest)
    } else if url.path() == "/w/index.php" {
        url.query_pairs()
            .find(|(k, _)| k == "title")
            .map(|(_, v)| decode_url_component(&v))
            .unwrap_or_default()
    } else {
        String::new()
    };

    title = title.trim().to_string();
    if title.is_empty() {
        return Err(WikidataError::InvalidInput(
            "could not extract article title from URL".to_string(),
        ));
    }
    title = title.replace(' ', "_");

    Ok(WikipediaPageRef { site, title })
}

fn decode_url_component(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let fake_query = format!("v={raw}");
    url::form_urlencoded::parse(fake_query.as_bytes())
        .find(|(k, _)| k == "v")
        .map(|(_, v)| v.into_owned())
        .unwrap_or_else(|| raw.to_string())
}

fn first_entity(
    mut resp: WbGetEntitiesResponse,
    key_hint: &str,
) -> Result<WikidataEntity, WikidataError> {
    resp.entities
        .drain()
        .map(|(_, entity)| entity)
        .find(|entity| entity.id.starts_with('Q'))
        .ok_or_else(|| WikidataError::NotFound(key_hint.to_string()))
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
        let resp: WbGetEntitiesResponse = self
            .send_with_retry(|| {
                self.http.get("https://www.wikidata.org/w/api.php").query(&[
                    ("action", "wbgetentities"),
                    ("format", "json"),
                    ("ids", qid),
                    ("languages", &languages),
                ])
            })
            .await?
            .json()
            .await?;
        first_entity(resp, qid)
    }

    async fn get_entity_by_sitelink(
        &self,
        site: &str,
        title: &str,
        langs: &[&str],
    ) -> Result<WikidataEntity, WikidataError> {
        let languages = langs.join("|");
        let resp: WbGetEntitiesResponse = self
            .send_with_retry(|| {
                self.http.get("https://www.wikidata.org/w/api.php").query(&[
                    ("action", "wbgetentities"),
                    ("format", "json"),
                    ("sites", site),
                    ("titles", title),
                    ("languages", &languages),
                ])
            })
            .await?
            .json()
            .await?;

        first_entity(resp, &format!("{site}:{title}"))
    }

    async fn search_entities(
        &self,
        query: &str,
        lang: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, WikidataError> {
        let max_limit = limit.clamp(1, 50);
        let limit_str = max_limit.to_string();
        let resp: WbSearchResponse = self
            .send_with_retry(|| {
                self.http.get("https://www.wikidata.org/w/api.php").query(&[
                    ("action", "wbsearchentities"),
                    ("format", "json"),
                    ("type", "item"),
                    ("language", lang),
                    ("search", query),
                    ("limit", &limit_str),
                ])
            })
            .await?
            .json()
            .await?;

        Ok(resp.search)
    }

    async fn sparql_query(&self, query: &str) -> Result<Vec<String>, WikidataError> {
        let resp: SparqlResponse = self
            .send_with_retry(|| {
                self.http
                    .get("https://query.wikidata.org/sparql")
                    .query(&[("query", query), ("format", "json")])
            })
            .await?
            .json()
            .await?;

        let qids = resp
            .results
            .bindings
            .into_iter()
            .filter_map(|mut row| {
                // Prefer the `item` key (Wikidata SPARQL convention: SELECT ?item WHERE ...),
                // then fall back to any URI-valued binding that looks like a Wikidata entity URL.
                let value = if let Some(v) = row.remove("item") {
                    Some(v.value)
                } else {
                    row.into_values()
                        .find(|v| v.value.starts_with("http://www.wikidata.org/entity/Q"))
                        .map(|v| v.value)
                }?;
                value
                    .rsplit('/')
                    .next()
                    .filter(|s| s.starts_with('Q'))
                    .map(String::from)
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

    #[test]
    fn parse_wikipedia_url_from_wiki_path() {
        let page = parse_wikipedia_url("https://ja.wikipedia.org/wiki/%E6%BC%A2").unwrap();
        assert_eq!(
            page,
            WikipediaPageRef {
                site: "jawiki".to_string(),
                title: "漢".to_string(),
            }
        );
    }

    #[test]
    fn parse_wikipedia_url_from_index_php_title() {
        let page =
            parse_wikipedia_url("https://en.wikipedia.org/w/index.php?title=Han_dynasty").unwrap();
        assert_eq!(
            page,
            WikipediaPageRef {
                site: "enwiki".to_string(),
                title: "Han_dynasty".to_string(),
            }
        );
    }

    #[test]
    fn parse_wikipedia_url_rejects_non_wikipedia_host() {
        let err = parse_wikipedia_url("https://example.com/wiki/Han").unwrap_err();
        assert!(matches!(err, WikidataError::InvalidInput(_)));
    }

    #[test]
    fn parse_wikipedia_url_http_scheme_accepted() {
        let page = parse_wikipedia_url("http://ja.wikipedia.org/wiki/%E6%BC%A2").unwrap();
        assert_eq!(
            page,
            WikipediaPageRef {
                site: "jawiki".to_string(),
                title: "漢".to_string(),
            }
        );
    }

    #[test]
    fn parse_wikipedia_url_rejects_empty_string() {
        let err = parse_wikipedia_url("").unwrap_err();
        assert!(matches!(err, WikidataError::InvalidInput(_)));
    }

    #[test]
    fn parse_wikipedia_url_rejects_unsupported_scheme() {
        let err = parse_wikipedia_url("ftp://ja.wikipedia.org/wiki/漢").unwrap_err();
        assert!(matches!(err, WikidataError::InvalidInput(_)));
    }

    #[test]
    fn parse_wikipedia_url_with_underscored_title() {
        let page = parse_wikipedia_url("https://en.wikipedia.org/wiki/Han_dynasty").unwrap();
        assert_eq!(
            page,
            WikipediaPageRef {
                site: "enwiki".to_string(),
                title: "Han_dynasty".to_string(),
            }
        );
    }

    /// Helper: parse a SPARQL JSON response and extract QIDs using the same
    /// logic as `HttpWikidataClient::sparql_query`.
    fn extract_qids_from_sparql_json(payload: &str) -> Vec<String> {
        let resp: SparqlResponse = serde_json::from_str(payload).unwrap();
        resp.results
            .bindings
            .into_iter()
            .filter_map(|mut row| {
                let value = if let Some(v) = row.remove("item") {
                    Some(v.value)
                } else {
                    row.into_values()
                        .find(|v| v.value.starts_with("http://www.wikidata.org/entity/Q"))
                        .map(|v| v.value)
                }?;
                value
                    .rsplit('/')
                    .next()
                    .filter(|s| s.starts_with('Q'))
                    .map(String::from)
            })
            .collect()
    }

    #[test]
    fn parse_sparql_response_prefers_item_key() {
        // Response has both `item` and `name` columns; `item` should be used
        let payload = r#"
        {
          "results": {
            "bindings": [
              {
                "name": { "value": "http://www.wikidata.org/entity/Q9999" },
                "item": { "value": "http://www.wikidata.org/entity/Q42" }
              },
              {
                "name": { "value": "http://www.wikidata.org/entity/Q8888" },
                "item": { "value": "http://www.wikidata.org/entity/Q7209" }
              }
            ]
          }
        }"#;

        let qids = extract_qids_from_sparql_json(payload);
        assert_eq!(qids, vec!["Q42", "Q7209"]);
    }

    #[test]
    fn parse_sparql_response_fallback_to_uri_binding() {
        // Response has no `item` key; should fall back to URI-valued binding
        let payload = r#"
        {
          "results": {
            "bindings": [
              {
                "entity": { "value": "http://www.wikidata.org/entity/Q7183" }
              },
              {
                "entity": { "value": "http://www.wikidata.org/entity/Q7209" }
              }
            ]
          }
        }"#;

        let qids = extract_qids_from_sparql_json(payload);
        assert_eq!(qids, vec!["Q7183", "Q7209"]);
    }

    #[tokio::test]
    async fn retry_on_429_then_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // First call returns 429, second returns 200 with valid JSON.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(r#"{"search":[]}"#, "application/json"),
            )
            .mount(&server)
            .await;

        let http = reqwest::Client::builder()
            .user_agent("tdsl-test")
            .build()
            .unwrap();
        let client = HttpWikidataClient {
            http,
            max_retries: DEFAULT_MAX_RETRIES,
        };
        let base = format!("{}/w/api.php", server.uri());

        let result = client.send_with_retry(|| client.http.get(&base)).await;

        assert!(
            result.is_ok(),
            "expected success after retry, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn returns_rate_limit_error_after_max_retries() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // Always returns 429.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let http = reqwest::Client::builder()
            .user_agent("tdsl-test")
            .build()
            .unwrap();
        let client = HttpWikidataClient {
            http,
            max_retries: DEFAULT_MAX_RETRIES,
        };
        let base = format!("{}/w/api.php", server.uri());

        let result = client.send_with_retry(|| client.http.get(&base)).await;

        assert!(
            matches!(result, Err(WikidataError::RateLimit)),
            "expected RateLimit error, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn retry_on_429_respects_retry_after_header() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // First call returns 429 with Retry-After: 1
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(r#"{"search":[]}"#, "application/json"),
            )
            .mount(&server)
            .await;

        let http = reqwest::Client::builder()
            .user_agent("tdsl-test")
            .build()
            .unwrap();
        let client = HttpWikidataClient {
            http,
            max_retries: DEFAULT_MAX_RETRIES,
        };
        let base = format!("{}/w/api.php", server.uri());

        let result = client.send_with_retry(|| client.http.get(&base)).await;

        assert!(
            result.is_ok(),
            "expected success after 429+Retry-After retry, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn returns_server_error_after_max_retries() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        // Always returns 503.
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let http = reqwest::Client::builder()
            .user_agent("tdsl-test")
            .build()
            .unwrap();
        // max_retries=0 so the first 5xx triggers an immediate error.
        let client = HttpWikidataClient {
            http,
            max_retries: 0,
        };
        let base = format!("{}/w/api.php", server.uri());

        let result = client.send_with_retry(|| client.http.get(&base)).await;

        assert!(
            matches!(result, Err(WikidataError::Http(_))),
            "expected Http error after exhausting retries, got: {result:?}"
        );
    }

    #[tokio::test]
    async fn retry_on_500_then_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(r#"{"search":[]}"#, "application/json"),
            )
            .mount(&server)
            .await;

        let http = reqwest::Client::builder()
            .user_agent("tdsl-test")
            .build()
            .unwrap();
        let client = HttpWikidataClient {
            http,
            max_retries: DEFAULT_MAX_RETRIES,
        };
        let base = format!("{}/w/api.php", server.uri());

        let result = client.send_with_retry(|| client.http.get(&base)).await;

        assert!(
            result.is_ok(),
            "expected success after 500 retry, got: {result:?}"
        );
    }
}
