//! Shared REST API contracts.

pub mod auth;
pub mod error;
pub mod servers;

pub use auth::{
    AuthResponse, AuthUser, LinkedAccount, LinkedAccountsResponse, LoginRequest, LogoutRequest,
    OAuthCompleteRequest, OAuthCompleteResponse, OAuthFlow, OAuthProvider,
    OAuthRegistrationRequest, OAuthStartRequest, OAuthStartResponse, PasswordResetConfirmRequest,
    PasswordResetRequest, RefreshRequest, RegisterRequest, UnlinkProviderRequest,
};
pub use error::ApiError;
pub use servers::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    ListServerRoomsResponse, ListServersResponse, ServerInviteInfoResponse, ServerInviteSummary,
    ServerRoomKind, ServerRoomSummary, ServerSummary, UpdateServerRoomRequest,
    UpdateServerRoomResponse,
};
