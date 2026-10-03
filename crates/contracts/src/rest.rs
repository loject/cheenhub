//! Общие контракты REST API.

pub mod auth;
pub mod desktop_oauth;
pub mod error;
pub mod host_settings;
pub mod push_notifications;
pub mod servers;
pub mod social;

pub use auth::{
    AccountDeletionResponse, AccountRestoreRequest, ActiveSession, ActiveSessionsResponse,
    AuthResponse, AuthUser, ChangeCurrentUserPasswordRequest, GoogleNativeAuthCompleteRequest,
    GoogleNativeAuthStartResponse, LinkedAccount, LinkedAccountsResponse, LoginRequest,
    LogoutRequest, OAuthCompleteRequest, OAuthCompleteResponse, OAuthFlow, OAuthProvider,
    OAuthRegistrationRequest, OAuthStartRequest, OAuthStartResponse, PasswordResetConfirmRequest,
    PasswordResetRequest, RefreshRequest, RegisterRequest, SessionClientInfo, SessionDeviceKind,
    UnlinkProviderRequest, UpdateCurrentUserRequest,
};
pub use desktop_oauth::{
    GoogleDesktopAuthPollResponse, GoogleDesktopAuthRequest, GoogleDesktopAuthStartResponse,
};
pub use error::ApiError;
pub use host_settings::{
    EmailTransport, GmailConnectionStartResponse, HostAccessResponse, HostCpuMetrics,
    HostDiskMetrics, HostEmailSettingsResponse, HostLogEntry, HostLogStreamMessage,
    HostMemoryMetrics, HostMessagesPerMinuteSample, HostMetricsResponse, HostMetricsSample,
    HostNetworkMetrics, HostStatsResponse, HostVoiceActivityHistoryResponse,
    HostVoiceActivityResponse, HostVoiceActivitySample, UpdateHostEmailSettingsRequest,
};
pub use push_notifications::{PushPlatform, UpsertPushInstallationRequest};
pub use servers::{
    AcceptServerInviteResponse, CreateServerInviteRequest, CreateServerInviteResponse,
    CreateServerRequest, CreateServerResponse, CreateServerRoomRequest, CreateServerRoomResponse,
    DeleteServerInviteResponse, ListServerRoomsResponse, ListServersResponse,
    ServerInviteInfoResponse, ServerInviteLinksResponse, ServerInviteSummary, ServerOwnInviteLink,
    ServerRoomKind, ServerRoomSummary, ServerRoomWriteAccess, ServerRoomWriteAccessMode,
    ServerSummary, ServerVoiceSettings, UpdateServerAvatarResponse, UpdateServerRequest,
    UpdateServerResponse, UpdateServerRoomRequest, UpdateServerRoomResponse,
};
pub use social::{
    DmConversationSummary, DmImageAttachmentSummary, DmLastMessageSummary, DmMessageDeliveryStatus,
    DmMessageSummary, FriendRequestStatus, FriendRequestSummary, FriendSummary,
    ListDmConversationsResponse, ListDmMessagesResponse, ListFriendRequestsResponse,
    ListFriendsQuery, ListFriendsResponse, MarkDmConversationReadRequest,
    MarkDmConversationReadResponse, OpenDmConversationRequest, OpenDmConversationResponse,
    SearchUsersResponse, SendDmMessageRequest, SendDmMessageResponse, SendFriendRequestRequest,
    SendFriendRequestResponse, UploadDmImageResponse, UserRelationStatus, UserSearchResult,
};

#[cfg(test)]
mod tests;
