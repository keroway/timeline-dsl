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

    /// 秒精度・オフセット付き時刻はまだ IR に反映できない（#613 で対応予定）。
    /// silent に分精度へ切り捨てることはせず、明示的エラーとして拒否する
    /// （AGENTS.md §4.1 no silent fallback / ADR 0003）。
    #[error(
        "Second precision and UTC offset are parsed but not yet supported by the IR (tracked in #613): {0}"
    )]
    SubMinutePrecisionNotYetSupported(String),
}
