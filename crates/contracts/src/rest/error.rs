//! Shared REST error contracts.

use serde::{Deserialize, Serialize};

/// Structured error response returned by REST endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    /// Stable machine-readable error code.
    pub code: String,
    /// User-facing error message.
    pub message: String,
}
