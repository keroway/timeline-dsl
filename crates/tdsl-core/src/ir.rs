use serde::{Deserialize, Serialize};

/// The canonical intermediate representation, serializable to JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIr {
    pub meta: Meta,
    pub lanes: Vec<Lane>,
    pub items: Vec<Item>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ImportRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub title: String,
    pub unit: String,
    pub range: (i64, i64),
    pub calendar: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lane {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Item {
    Span {
        id: String,
        lane: String,
        start: i64,
        end: i64,
        label: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    Event {
        id: String,
        lane: String,
        time: i64,
        label: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    EventRange {
        id: String,
        lane: String,
        start: i64,
        end: i64,
        label: String,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tags: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRecord {
    pub source: String,
    pub qid: String,
    pub mapped_to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub provider: String,
    pub license: String,
}
