use std::collections::HashMap;

use async_trait::async_trait;
use tdsl_wikidata::entity::{DataValue, LabelValue, Snak, Statement, TimeValue};
use tdsl_wikidata::{SearchResult, WikidataClient, WikidataEntity, WikidataError};

pub(super) struct MockWikidataClient {
    pub(super) entities: HashMap<String, WikidataEntity>,
    pub(super) query_results: Vec<String>,
}

#[async_trait]
impl WikidataClient for MockWikidataClient {
    async fn get_entity(
        &self,
        qid: &str,
        _langs: &[&str],
    ) -> Result<WikidataEntity, WikidataError> {
        self.entities
            .get(qid)
            .cloned()
            .ok_or_else(|| WikidataError::NotFound(qid.to_string()))
    }

    async fn get_entity_by_sitelink(
        &self,
        _site: &str,
        title: &str,
        _langs: &[&str],
    ) -> Result<WikidataEntity, WikidataError> {
        let qid = self
            .entities
            .iter()
            .find_map(|(qid, entity)| {
                if entity
                    .labels
                    .values()
                    .any(|label| label.value == title || label.value.replace(' ', "_") == title)
                {
                    Some(qid.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| WikidataError::NotFound(title.to_string()))?;
        self.entities
            .get(&qid)
            .cloned()
            .ok_or(WikidataError::NotFound(title.to_string()))
    }

    async fn search_entities(
        &self,
        _query: &str,
        _lang: &str,
        _limit: usize,
    ) -> Result<Vec<SearchResult>, WikidataError> {
        Ok(Vec::new())
    }

    async fn sparql_query(&self, _query: &str) -> Result<Vec<String>, WikidataError> {
        Ok(self.query_results.clone())
    }
}

pub(super) fn make_time(year: i64) -> TimeValue {
    TimeValue {
        time: format!("{year:+05}-01-01T00:00:00Z"),
        precision: 9,
        calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
    }
}

pub(super) fn make_time_statement(property: &str, year: i64) -> Statement {
    Statement {
        mainsnak: Snak {
            snaktype: "value".to_string(),
            property: property.to_string(),
            datavalue: Some(DataValue::Time {
                value: make_time(year),
            }),
        },
        rank: "normal".to_string(),
        qualifiers: HashMap::new(),
    }
}

pub(super) fn make_entity(id: &str, ja_label: &str, start: i64, end: i64) -> WikidataEntity {
    let mut labels = HashMap::new();
    labels.insert(
        "ja".to_string(),
        LabelValue {
            language: "ja".to_string(),
            value: ja_label.to_string(),
        },
    );

    let mut claims = HashMap::new();
    claims.insert("P571".to_string(), vec![make_time_statement("P571", start)]);
    claims.insert("P576".to_string(), vec![make_time_statement("P576", end)]);

    WikidataEntity {
        id: id.to_string(),
        labels,
        claims,
    }
}
