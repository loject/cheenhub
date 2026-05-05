//! Server feature error types.

/// Error returned by server application flows.
#[derive(Debug)]
pub(crate) enum ServerError {
    /// Request data is invalid.
    BadRequest(String),
    /// Requested server resource is not available.
    NotFound(String),
    /// Credentials or tokens are invalid.
    Unauthorized(String),
    /// Unexpected infrastructure failure.
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for ServerError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
