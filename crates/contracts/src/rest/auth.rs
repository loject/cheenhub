//! Authentication REST contracts.

use serde::{Deserialize, Serialize};

/// Request body used to create a new email/password account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    /// Public nickname shown to other users.
    pub nickname: String,
    /// Email address used for login.
    pub email: String,
    /// Plain password submitted over HTTPS.
    pub password: String,
    /// Whether the user accepted mandatory policies.
    pub accepts_policies: bool,
}

/// Request body used to login with email/password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    /// Email address used for login.
    pub email: String,
    /// Plain password submitted over HTTPS.
    pub password: String,
}

/// Request body used to rotate a refresh token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshRequest {
    /// Opaque refresh token previously issued by the backend.
    pub refresh_token: String,
}

/// Request body used to invalidate a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// Opaque refresh token identifying the session to invalidate.
    pub refresh_token: String,
}

/// Successful authentication response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    /// Short-lived Ed25519-signed access JWT.
    pub access_token: String,
    /// Long-lived opaque refresh token.
    pub refresh_token: String,
    /// Authenticated user profile.
    pub user: AuthUser,
}

/// User data returned by authentication endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    /// Stable user identifier.
    pub id: String,
    /// Public nickname shown to other users.
    pub nickname: String,
    /// Email address used for login.
    pub email: String,
    /// RFC 3339 registration timestamp.
    pub registered_at: String,
}
