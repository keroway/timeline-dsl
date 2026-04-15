use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Represents a Wikidata entity (simplified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikidataEntity {
    pub id: String,
    #[serde(default)]
    pub labels: HashMap<String, LabelValue>,
    #[serde(default)]
    pub claims: HashMap<String, Vec<Statement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelValue {
    pub language: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub mainsnak: Snak,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub qualifiers: HashMap<String, Vec<Snak>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snak {
    pub snaktype: String,
    pub property: String,
    #[serde(default)]
    pub datavalue: Option<DataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DataValue {
    #[serde(rename = "time")]
    Time { value: TimeValue },
    #[serde(rename = "wikibase-entityid")]
    WikibaseEntityId { value: EntityIdValue },
    #[serde(rename = "string")]
    String { value: String },
    #[serde(rename = "monolingualtext")]
    MonolingualText { value: MonolingualTextValue },
    #[serde(rename = "quantity")]
    Quantity { value: serde_json::Value },
    #[serde(rename = "globecoordinate")]
    GlobeCoordinate { value: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonolingualTextValue {
    pub text: String,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeValue {
    pub time: String,
    pub precision: u8,
    #[serde(default)]
    pub calendarmodel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityIdValue {
    pub id: String,
    #[serde(default, rename = "numeric-id")]
    pub numeric_id: u64,
}

impl WikidataEntity {
    /// Get the first claim value for the given property.
    pub fn claim(&self, property: &str) -> Option<&DataValue> {
        self.claims
            .get(property)?
            .iter()
            .find(|s| s.rank != "deprecated")
            .and_then(|s| s.mainsnak.datavalue.as_ref())
    }

    /// Get label with language fallback.
    pub fn label_with_fallback(&self, langs: &[&str]) -> Option<&str> {
        for lang in langs {
            if let Some(lv) = self.labels.get(*lang) {
                return Some(&lv.value);
            }
        }
        None
    }
}

/// Parse a Wikidata time string (e.g. `"+1868-01-01T00:00:00Z"` or `"-0206-01-01T00:00:00Z"`)
/// into a year integer.
pub fn time_value_to_year(tv: &TimeValue) -> Result<i64, crate::WikidataError> {
    let s = &tv.time;
    // Format: +/-YYYY-MM-DDThh:mm:ssZ
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('+') {
        (1i64, stripped)
    } else if let Some(stripped) = s.strip_prefix('-') {
        (-1i64, stripped)
    } else {
        (1i64, s.as_str())
    };

    let year_str = rest.split('-').next().unwrap_or("");
    let year: i64 = year_str
        .parse()
        .map_err(|_| crate::WikidataError::TimeParseError(s.clone()))?;

    Ok(sign * year)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_time(time_str: &str) -> TimeValue {
        TimeValue {
            time: time_str.to_string(),
            precision: 9,
            calendarmodel: String::new(),
        }
    }

    #[test]
    fn parse_positive_year() {
        let tv = make_time("+1868-01-01T00:00:00Z");
        assert_eq!(time_value_to_year(&tv).unwrap(), 1868);
    }

    #[test]
    fn parse_negative_year() {
        let tv = make_time("-0206-01-01T00:00:00Z");
        assert_eq!(time_value_to_year(&tv).unwrap(), -206);
    }

    #[test]
    fn parse_large_negative_year() {
        let tv = make_time("-13798000000-01-01T00:00:00Z");
        assert_eq!(time_value_to_year(&tv).unwrap(), -13798000000);
    }

    #[test]
    fn parse_year_zero() {
        let tv = make_time("+0000-01-01T00:00:00Z");
        assert_eq!(time_value_to_year(&tv).unwrap(), 0);
    }

    #[test]
    fn parse_year_one_bce() {
        let tv = make_time("-0001-01-01T00:00:00Z");
        assert_eq!(time_value_to_year(&tv).unwrap(), -1);
    }

    #[test]
    fn label_fallback() {
        let mut labels = HashMap::new();
        labels.insert("ja".to_string(), LabelValue {
            language: "ja".to_string(),
            value: "漢".to_string(),
        });
        labels.insert("en".to_string(), LabelValue {
            language: "en".to_string(),
            value: "Han dynasty".to_string(),
        });
        let entity = WikidataEntity {
            id: "Q7209".to_string(),
            labels,
            claims: HashMap::new(),
        };
        assert_eq!(entity.label_with_fallback(&["ja", "en"]), Some("漢"));
        assert_eq!(entity.label_with_fallback(&["zh", "en"]), Some("Han dynasty"));
        assert_eq!(entity.label_with_fallback(&["fr"]), None);
    }
}
