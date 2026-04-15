use thiserror::Error;

#[derive(Debug, Error)]
pub enum WikidataError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Failed to parse time value: {0}")]
    TimeParseError(String),

    #[error("Missing claim {property} on entity {entity}")]
    MissingClaim { entity: String, property: String },
}
