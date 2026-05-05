//! Shared REST API contracts.

pub mod auth;
pub mod error;
pub mod servers;

pub use auth::{
    AuthResponse, AuthUser, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
};
pub use error::ApiError;
pub use servers::{CreateServerRequest, CreateServerResponse, ListServersResponse, ServerSummary};
