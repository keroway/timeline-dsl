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

    /// `Retry-After` が上限を超える待機を指示した。
    ///
    /// 指示に従うと CLI が進捗表示なしで長時間ブロックするため、待たずに返す（#768）。
    #[error(
        "Wikidata API asked to wait {requested_secs}s (Retry-After), which exceeds the {max_secs}s cap. Not waiting; retry later or use --offline."
    )]
    RateLimitRetryAfterTooLong {
        /// サーバが指示した待機秒数。
        requested_secs: u64,
        /// ライブラリ側の上限秒数。
        max_secs: u64,
    },
}
