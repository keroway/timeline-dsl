use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Syntax error: {0}")]
    Syntax(#[from] pest::error::Error<crate::Rule>),

    #[error("Invalid integer at {location}: {value}")]
    InvalidInt { value: String, location: String },

    #[error("Unknown re-import policy: {0}")]
    UnknownPolicy(String),

    #[error("Unknown map target type: {0} (expected span, event, or event_range)")]
    UnknownTargetType(String),

    #[error("Unexpected rule {rule} at {location}")]
    UnexpectedRule { rule: String, location: String },

    #[error("Invalid month at {location}: {value} (expected 1-12)")]
    InvalidMonth { value: u32, location: String },

    #[error("Invalid day at {location}: {value} (expected 1-31)")]
    InvalidDay { value: u32, location: String },
}
