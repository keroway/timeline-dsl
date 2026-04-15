use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Syntax error: {0}")]
    Syntax(#[from] pest::error::Error<crate::Rule>),

    #[error("Invalid integer at {location}: {value}")]
    InvalidInt {
        value: String,
        location: String,
    },

    #[error("Unknown re-import policy: {0}")]
    UnknownPolicy(String),

    #[error("Unexpected rule {rule} at {location}")]
    UnexpectedRule {
        rule: String,
        location: String,
    },
}
