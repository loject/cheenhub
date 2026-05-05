//! Authentication error types.

/// Error returned by authentication application flows.
#[derive(Debug)]
pub(crate) enum AuthError {
    /// Request data is invalid.
    BadRequest(String),
    /// Credentials or tokens are invalid.
    Unauthorized(String),
    /// A unique account field already exists.
    Conflict(String),
    /// Unexpected infrastructure failure.
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for AuthError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
