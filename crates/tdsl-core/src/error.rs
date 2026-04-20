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
}
