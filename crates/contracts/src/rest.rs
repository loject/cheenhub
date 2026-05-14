//! Shared REST API contracts.

pub mod auth;
pub mod error;
pub mod servers;

pub use auth::{
    AuthResponse, AuthUser, ChangeCurrentUserPasswordRequest, LinkedAccount,
    LinkedAccountsResponse, LoginRequest, LogoutRequest, OAuthCompleteRequest,
    OAuthCompleteResponse, OAuthFlow, OAuthProvider, OAuthRegistrationRequest, OAuthStartRequest,
    OAuthStartResponse, PasswordResetConfirmRequest, PasswordResetRequest, RefreshRequest,
    RegisterRequest, UnlinkProviderRequest, UpdateCurrentUserRequest,
};
pub use error::ApiError;
pub use servers::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerInviteSummary,
    ServerRoomKind, ServerRoomSummary, ServerSummary, UpdateServerRoomRequest,
    UpdateServerRoomResponse,
};

#[cfg(test)]
mod tests {
    use super::AuthUser;

    #[test]
    fn auth_user_avatar_url_round_trips() {
        let user = AuthUser {
            id: "user-id".to_owned(),
            nickname: "avatar_user".to_owned(),
            email: "avatar@example.com".to_owned(),
            registered_at: "2026-05-13T00:00:00Z".to_owned(),
            has_password: true,
            avatar_url: Some("http://localhost/api/images/avatar".to_owned()),
        };

        let decoded: AuthUser =
            serde_json::from_str(&serde_json::to_string(&user).expect("user serializes"))
                .expect("user decodes");

        assert_eq!(decoded.avatar_url, user.avatar_url);
    }
}
