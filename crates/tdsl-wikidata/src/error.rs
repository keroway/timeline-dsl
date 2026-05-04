use thiserror::Error;

#[derive(Debug, Error)]
pub enum WikidataError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Failed to parse time value: {0}")]
    TimeParseError(String),

    #[error("Missing claim {property} on entity {entity}")]
    MissingClaim { entity: String, property: String },

    #[error("Wikidata API request timed out. Try running with the --offline flag.")]
    Timeout,

    #[error("Wikidata API rate limit exceeded (HTTP 429). Please wait a moment and retry.")]
    RateLimit,
}
