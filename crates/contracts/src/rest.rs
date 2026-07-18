//! Общие контракты REST API.

pub mod auth;
pub mod error;
pub mod push_notifications;
pub mod servers;
pub mod social;

pub use auth::{
    ActiveSession, ActiveSessionsResponse, AuthResponse, AuthUser,
    ChangeCurrentUserPasswordRequest, LinkedAccount, LinkedAccountsResponse, LoginRequest,
    LogoutRequest, OAuthCompleteRequest, OAuthCompleteResponse, OAuthFlow, OAuthProvider,
    OAuthRegistrationRequest, OAuthStartRequest, OAuthStartResponse, PasswordResetConfirmRequest,
    PasswordResetRequest, RefreshRequest, RegisterRequest, SessionClientInfo, SessionDeviceKind,
    UnlinkProviderRequest, UpdateCurrentUserRequest,
};
pub use error::ApiError;
pub use push_notifications::{PushPlatform, UpsertPushInstallationRequest};
pub use servers::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerInviteSummary,
    ServerRoomKind, ServerRoomSummary, ServerSummary, UpdateServerAvatarResponse,
    UpdateServerRequest, UpdateServerResponse, UpdateServerRoomRequest, UpdateServerRoomResponse,
};
pub use social::{
    DmConversationSummary, DmImageAttachmentSummary, DmMessageDeliveryStatus, DmMessageSummary,
    FriendRequestStatus, FriendRequestSummary, FriendSummary, ListDmConversationsResponse,
    ListDmMessagesResponse, ListFriendRequestsResponse, ListFriendsResponse,
    MarkDmConversationReadRequest, MarkDmConversationReadResponse, OpenDmConversationRequest,
    OpenDmConversationResponse, SearchUsersResponse, SendDmMessageRequest, SendDmMessageResponse,
    SendFriendRequestRequest, SendFriendRequestResponse, UploadDmImageResponse, UserRelationStatus,
    UserSearchResult,
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
