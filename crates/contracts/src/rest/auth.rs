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

/// Request body used to start a password reset email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetRequest {
    /// Email address that should receive a password reset link.
    pub email: String,
}

/// Request body used to finish a password reset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordResetConfirmRequest {
    /// Opaque reset token from the password reset link.
    pub token: String,
    /// New plain password submitted over HTTPS.
    pub new_password: String,
}

/// External OAuth provider supported by the REST API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthProvider {
    /// Google OAuth identity provider.
    Google,
}

/// OAuth flow kind requested by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OAuthFlow {
    /// Log in or register with the external provider.
    Login,
    /// Link the external provider to the current authenticated account.
    Link,
}

/// Request body used to start an OAuth authorization flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthStartRequest {
    /// OAuth flow kind requested by the client.
    pub flow: OAuthFlow,
}

/// Response returned after an OAuth authorization flow is prepared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthStartResponse {
    /// Provider authorization URL where the browser should navigate.
    pub authorization_url: String,
}

/// Request body used to finish an OAuth callback handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthCompleteRequest {
    /// One-time handoff code returned through the frontend callback URL.
    pub handoff_code: String,
}

/// Response returned when completing an OAuth handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OAuthCompleteResponse {
    /// OAuth produced an authenticated CheenHub session.
    Authenticated {
        /// Authentication tokens and current user.
        auth: AuthResponse,
    },
    /// OAuth identity is verified but a new CheenHub account needs a nickname.
    RegistrationRequired {
        /// One-time token used to finish registration.
        registration_token: String,
        /// Verified email address from the OAuth provider.
        email: String,
        /// Display name returned by the OAuth provider.
        display_name: Option<String>,
    },
    /// OAuth linked a provider to the current account.
    Linked {
        /// Linked external account.
        account: LinkedAccount,
    },
}

/// Request body used to finish OAuth registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAuthRegistrationRequest {
    /// One-time registration token returned by OAuth completion.
    pub registration_token: String,
    /// Public nickname chosen for the new CheenHub account.
    pub nickname: String,
    /// Whether the user accepted mandatory policies.
    pub accepts_policies: bool,
}

/// Request body used to update the current authenticated user profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCurrentUserRequest {
    /// New public nickname shown to other users.
    pub nickname: String,
}

/// Request body used to change the current user's password.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeCurrentUserPasswordRequest {
    /// Current plain password submitted over HTTPS.
    pub current_password: String,
    /// New plain password submitted over HTTPS.
    pub new_password: String,
    /// Repeated new password submitted to prevent mistyped changes.
    pub new_password_confirmation: String,
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

/// Linked external account returned to account settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAccount {
    /// External OAuth provider.
    pub provider: OAuthProvider,
    /// Email address reported by the provider.
    pub email: String,
    /// Display name reported by the provider.
    pub display_name: Option<String>,
    /// RFC 3339 timestamp when the provider was linked.
    pub linked_at: String,
}

/// Response containing external accounts linked to the current user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedAccountsResponse {
    /// Linked external accounts.
    pub accounts: Vec<LinkedAccount>,
}

/// Request body used to unlink an external provider account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlinkProviderRequest {
    /// External OAuth provider to unlink.
    pub provider: OAuthProvider,
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
    /// Whether the account has a local password.
    pub has_password: bool,
}
