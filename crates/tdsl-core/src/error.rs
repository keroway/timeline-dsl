use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoweringError {
    #[error("Parse error: {0}")]
    Parse(#[from] tdsl_parser::error::ParseError),

    #[error("Unknown lane reference: {0}")]
    UnknownLane(String),

    #[error("Duplicate lane alias: {0}")]
    DuplicateLane(String),

    #[error("Duplicate item id: {0}")]
    DuplicateItemId(String),

    #[error("No timeline block found")]
    NoTimeline,

    #[error("Multiple timeline blocks found")]
    MultipleTimelines,

    #[error("Unknown timeline unit: {value} (expected one of: {expected})")]
    UnknownTimelineUnit { value: String, expected: String },

    #[cfg(feature = "wikidata")]
    #[error("Wikidata error: {0}")]
    Wikidata(#[from] tdsl_wikidata::WikidataError),

    #[error("Unresolved import reference: {0}")]
    UnresolvedImport(String),

    #[error("Unresolved entity key: {0}")]
    UnresolvedEntity(String),

    #[error("Map references unknown lane: {0}")]
    UnknownMappedLane(String),

    #[error("Duplicate template alias: {0}")]
    DuplicateTemplate(String),

    #[error("Unknown template reference: {0}")]
    UnknownTemplate(String),

    #[error("Invalid item link URL: {0} (expected http:// or https:// URL)")]
    InvalidItemLink(String),

    #[error("Invalid item color value: {0}")]
    InvalidItemColor(String),

    /// offset付きtime valueとoffsetなしtime valueの比較は曖昧なので明示エラーとする
    /// （ADR 0003 D2: 「オフセットなしは暗黙にUTCとはみなさない」/ AGENTS.md §4.1 no silent fallback）。
    #[error(
        "Cannot compare a UTC-offset time value with a value that has no offset (author must make both sides consistent): {0} vs {1}"
    )]
    MixedOffsetComparison(String, String),
}
