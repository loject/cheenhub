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
    /// Request is valid but currently rate limited.
    RateLimited(String),
    /// A required runtime integration is not configured.
    Misconfigured {
        /// Integration or feature name.
        feature: &'static str,
        /// Missing environment variable names.
        missing: Vec<&'static str>,
        /// User-facing message.
        message: String,
    },
    /// Unexpected infrastructure failure.
    Internal(anyhow::Error),
}

impl AuthError {
    /// Returns the user-facing error message when this error is safe to expose.
    pub(crate) fn user_message(&self) -> Option<&str> {
        match self {
            Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::Conflict(message)
            | Self::RateLimited(message) => Some(message),
            Self::Misconfigured { message, .. } => Some(message),
            Self::Internal(_) => None,
        }
    }
}

impl From<anyhow::Error> for AuthError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}
