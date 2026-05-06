//! Control realtime module contracts.

use serde::{Deserialize, Serialize};

use crate::rest::AuthUser;

/// Control module message kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    /// Authenticate a newly opened realtime session.
    Authenticate,
    /// Confirm a successful realtime authentication.
    Authenticated,
    /// Temporary reliable control diagnostic request.
    ControlText,
    /// Temporary reliable control diagnostic response.
    ControlAck,
    /// Reject a realtime request or session.
    Rejected,
}

/// Request payload used to authenticate a realtime session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authenticate {
    /// Short-lived access JWT.
    pub access_token: String,
}

/// Response payload returned after a successful realtime authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authenticated {
    /// Authenticated user bound to the realtime session.
    pub user: AuthUser,
}

/// Temporary reliable diagnostic control request payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlText {
    /// Diagnostic text body.
    pub body: String,
}

/// Temporary reliable diagnostic control response payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlAck {
    /// Diagnostic response body.
    pub body: String,
}

/// Stable realtime rejection code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    /// Credentials or session state are invalid.
    Unauthorized,
    /// Message shape, module, kind, or payload is invalid.
    BadRequest,
    /// The requested module is not supported.
    UnsupportedModule,
    /// The requested message kind is not supported by the module.
    UnsupportedMessage,
    /// Unexpected server failure.
    InternalError,
}

/// Rejection payload returned for realtime protocol errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejected {
    /// Stable machine-readable rejection code.
    pub code: RejectionCode,
    /// Human-readable rejection message.
    pub message: String,
}
