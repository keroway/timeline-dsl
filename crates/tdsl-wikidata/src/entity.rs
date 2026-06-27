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

/// Wikidata の言語付きラベル値。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelValue {
    pub language: String,
    pub value: String,
}

/// Wikidata のステートメント（主張）。プロパティ値と rank を持つ。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub mainsnak: Snak,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub qualifiers: HashMap<String, Vec<Snak>>,
}

/// Wikidata の Snak（値の主張単位）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snak {
    pub snaktype: String,
    pub property: String,
    #[serde(default)]
    pub datavalue: Option<DataValue>,
}

/// Wikidata のデータ値型（time / entity / string など）。
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

/// 単言語テキスト値（language + text）。
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

    /// Get the first qualifier value for `qualifier_property` from the first non-deprecated
    /// statement of `main_property`.
    pub fn qualifier_claim(
        &self,
        main_property: &str,
        qualifier_property: &str,
    ) -> Option<&DataValue> {
        let stmt = self
            .claims
            .get(main_property)?
            .iter()
            .find(|s| s.rank != "deprecated")?;
        stmt.qualifiers
            .get(qualifier_property)?
            .first()
            .and_then(|s| s.datavalue.as_ref())
    }

    /// Return all statements for `property` as a slice.
    /// Returns an empty slice when the property is absent.
    pub fn statements(&self, property: &str) -> &[Statement] {
        self.claims
            .get(property)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
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

/// Parsed time value with optional month/day/hour/minute precision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimePoint {
    pub year: i64,
    /// `Some` when precision >= 10 (month).
    pub month: Option<u8>,
    /// `Some` when precision >= 11 (day).
    pub day: Option<u8>,
    /// `Some` when precision >= 12 (hour).
    pub hour: Option<u8>,
    /// `Some` when precision >= 13 (minute).
    pub minute: Option<u8>,
    pub precision: u8,
}

/// Parse a Wikidata time string (e.g. `"+1868-01-01T00:00:00Z"` or `"-0206-01-01T00:00:00Z"`)
/// into a year integer.
pub fn time_value_to_year(tv: &TimeValue) -> Result<i64, crate::WikidataError> {
    Ok(time_value_to_timepoint(tv)?.year)
}

/// Parse a Wikidata time value into a [`TimePoint`], preserving month/day when precision allows.
pub fn time_value_to_timepoint(tv: &TimeValue) -> Result<TimePoint, crate::WikidataError> {
    let s = &tv.time;
    // Format: +/-YYYY-MM-DDThh:mm:ssZ
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('+') {
        (1i64, stripped)
    } else if let Some(stripped) = s.strip_prefix('-') {
        (-1i64, stripped)
    } else {
        (1i64, s.as_str())
    };

    let mut parts = rest.splitn(3, '-');
    let year_str = parts.next().unwrap_or("");
    let year: i64 = year_str
        .parse()
        .map_err(|_| crate::WikidataError::TimeParseError(s.clone()))?;

    let month = if tv.precision >= 10 {
        parts
            .next()
            .and_then(|m| m.parse::<u8>().ok())
            .filter(|&m| (1..=12).contains(&m))
    } else {
        None
    };

    let time_part;
    let day = if tv.precision >= 11 {
        let raw_day = parts.next();
        time_part = raw_day.and_then(|d| d.split_once('T').map(|(_, t)| t));
        raw_day
            .and_then(|d| d.split('T').next())
            .and_then(|d| d.parse::<u8>().ok())
            .filter(|&d| (1..=31).contains(&d))
    } else {
        time_part = None;
        None
    };

    let mut clock = time_part.unwrap_or("").trim_end_matches('Z').split(':');
    let hour = if tv.precision >= 12 {
        clock
            .next()
            .and_then(|h| h.parse::<u8>().ok())
            .filter(|&h| h <= 23)
    } else {
        None
    };
    let minute = if tv.precision >= 13 {
        clock
            .next()
            .and_then(|m| m.parse::<u8>().ok())
            .filter(|&m| m <= 59)
    } else {
        None
    };

    Ok(TimePoint {
        year: sign * year,
        month,
        day,
        hour,
        minute,
        precision: tv.precision,
    })
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
    fn parse_time_precision_century() {
        // precision=7 (century) — year extraction still works
        let tv = TimeValue {
            time: "+1900-01-01T00:00:00Z".to_string(),
            precision: 7,
            calendarmodel: String::new(),
        };
        assert_eq!(time_value_to_year(&tv).unwrap(), 1900);
    }

    #[test]
    fn parse_time_precision_decade() {
        // precision=8 (decade)
        let tv = TimeValue {
            time: "+1940-01-01T00:00:00Z".to_string(),
            precision: 8,
            calendarmodel: String::new(),
        };
        assert_eq!(time_value_to_year(&tv).unwrap(), 1940);
    }

    #[test]
    fn parse_time_precision_millennium() {
        // precision=6 (millennium)
        let tv = TimeValue {
            time: "+1000-01-01T00:00:00Z".to_string(),
            precision: 6,
            calendarmodel: String::new(),
        };
        assert_eq!(time_value_to_year(&tv).unwrap(), 1000);
    }

    #[test]
    fn parse_time_precision_day() {
        // precision=11 (day)
        let tv = TimeValue {
            time: "+1868-01-03T00:00:00Z".to_string(),
            precision: 11,
            calendarmodel: String::new(),
        };
        assert_eq!(time_value_to_year(&tv).unwrap(), 1868);
    }

    #[test]
    fn parse_time_precision_minute_and_bce_date() {
        let tv = TimeValue {
            time: "+1969-07-20T20:17:00Z".to_string(),
            precision: 13,
            calendarmodel: String::new(),
        };
        let tp = time_value_to_timepoint(&tv).unwrap();
        assert_eq!(tp.hour, Some(20));
        assert_eq!(tp.minute, Some(17));

        let bce = TimeValue {
            time: "-0206-01-15T00:00:00Z".to_string(),
            precision: 11,
            calendarmodel: String::new(),
        };
        let tp = time_value_to_timepoint(&bce).unwrap();
        assert_eq!(tp.year, -206);
        assert_eq!(tp.month, Some(1));
        assert_eq!(tp.day, Some(15));
    }

    #[test]
    fn parse_time_without_sign_prefix() {
        // 符号なし文字列は正の年として扱う
        let tv = TimeValue {
            time: "1868-01-01T00:00:00Z".to_string(),
            precision: 9,
            calendarmodel: String::new(),
        };
        assert_eq!(time_value_to_year(&tv).unwrap(), 1868);
    }

    #[test]
    fn parse_time_invalid_returns_error() {
        let tv = TimeValue {
            time: "+not-a-year-01-01T00:00:00Z".to_string(),
            precision: 9,
            calendarmodel: String::new(),
        };
        assert!(time_value_to_year(&tv).is_err());
    }

    #[test]
    fn label_fallback() {
        let mut labels = HashMap::new();
        labels.insert(
            "ja".to_string(),
            LabelValue {
                language: "ja".to_string(),
                value: "漢".to_string(),
            },
        );
        labels.insert(
            "en".to_string(),
            LabelValue {
                language: "en".to_string(),
                value: "Han dynasty".to_string(),
            },
        );
        let entity = WikidataEntity {
            id: "Q7209".to_string(),
            labels,
            claims: HashMap::new(),
        };
        assert_eq!(entity.label_with_fallback(&["ja", "en"]), Some("漢"));
        assert_eq!(
            entity.label_with_fallback(&["zh", "en"]),
            Some("Han dynasty")
        );
        assert_eq!(entity.label_with_fallback(&["fr"]), None);
    }
}
