//! Server application flows.

use cheenhub_contracts::rest::{
    CreateServerInviteRequest, CreateServerInviteResponse, CreateServerRequest,
    CreateServerResponse, ListServersResponse, ServerSummary,
};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::auth::application as auth_application;
use crate::features::auth::error::AuthError;
use crate::features::servers::domain::Server;
use crate::features::servers::error::ServerError;
use crate::features::servers::validation;
use crate::http::AppState;

/// Creates a server owned by the current user.
pub(crate) async fn create(
    state: &AppState,
    access_token: &str,
    request: CreateServerRequest,
) -> Result<CreateServerResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let valid = validation::create_server(request.name)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let server = state
        .server_store
        .insert_server(&owner_user_id, valid.name)
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerResponse {
        server: server_summary(&server),
    })
}

/// Lists servers owned by the current user.
pub(crate) async fn list(
    state: &AppState,
    access_token: &str,
) -> Result<ListServersResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let servers = state
        .server_store
        .list_servers(&owner_user_id)
        .await
        .map_err(ServerError::Internal)?;

    Ok(ListServersResponse {
        servers: servers.iter().map(server_summary).collect(),
    })
}

/// Creates an invite for a server owned by the current user.
pub(crate) async fn create_invite(
    state: &AppState,
    access_token: &str,
    server_id: String,
    request: CreateServerInviteRequest,
) -> Result<CreateServerInviteResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let owner_user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let server_id = Uuid::parse_str(&server_id)
        .map_err(|_| ServerError::BadRequest("Сервер не найден.".to_owned()))?;
    let valid = validation::create_server_invite(request.max_uses, request.expires_in_days)
        .map_err(|message| ServerError::BadRequest(message.to_owned()))?;
    let Some(server) = state
        .server_store
        .find_owned_server(&server_id, &owner_user_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound(
            "Сервер не найден или недоступен.".to_owned(),
        ));
    };
    let expires_at = valid
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days.into()));
    let invite = state
        .server_store
        .insert_server_invite(&server.id, &owner_user_id, valid.max_uses, expires_at)
        .await
        .map_err(ServerError::Internal)?;

    Ok(CreateServerInviteResponse {
        code: invite.id.to_string(),
    })
}

fn server_summary(server: &Server) -> ServerSummary {
    ServerSummary {
        id: server.id.to_string(),
        name: server.name.clone(),
        is_owner: true,
    }
}

fn map_auth_error(error: AuthError) -> ServerError {
    match error {
        AuthError::BadRequest(message) | AuthError::Unauthorized(message) => {
            ServerError::Unauthorized(message)
        }
        AuthError::Conflict(message) => ServerError::BadRequest(message),
        AuthError::Internal(error) => ServerError::Internal(error),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cheenhub_contracts::rest::{
        CreateServerInviteRequest, CreateServerRequest, RegisterRequest,
    };

    use super::{create, create_invite, list};
    use crate::features::auth::application as auth_application;
    use crate::features::auth::infrastructure::InMemoryAuthStore;
    use crate::features::auth::security::keys::AuthKeys;
    use crate::features::servers::infrastructure::InMemoryServerStore;
    use crate::http::AppState;

    fn state() -> AppState {
        state_with_store(Arc::new(InMemoryServerStore::default()))
    }

    fn state_with_store(server_store: Arc<InMemoryServerStore>) -> AppState {
        AppState {
            auth_store: Arc::new(InMemoryAuthStore::default()),
            server_store,
            auth_keys: AuthKeys::generate_for_tests(),
            access_token_lifetime_minutes: 15,
            refresh_token_lifetime_days: 30,
        }
    }

    #[tokio::test]
    async fn creates_and_lists_servers_for_current_user() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "cheenhero".to_owned(),
                email: "hero@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("registration should succeed");

        let created = create(
            &state,
            &auth.access_token,
            CreateServerRequest {
                name: "  Dev Server  ".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");
        let listed = list(&state, &auth.access_token)
            .await
            .expect("server list should succeed");

        assert_eq!(created.server.name, "Dev Server");
        assert_eq!(listed.servers, vec![created.server]);
    }

    #[tokio::test]
    async fn lists_only_current_users_servers() {
        let state = state();
        let first_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "first_user".to_owned(),
                email: "first@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("first registration should succeed");
        let second_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "second_user".to_owned(),
                email: "second@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("second registration should succeed");

        let first_server = create(
            &state,
            &first_auth.access_token,
            CreateServerRequest {
                name: "First".to_owned(),
            },
        )
        .await
        .expect("first server should be created");
        create(
            &state,
            &second_auth.access_token,
            CreateServerRequest {
                name: "Second".to_owned(),
            },
        )
        .await
        .expect("second server should be created");

        let listed = list(&state, &first_auth.access_token)
            .await
            .expect("server list should succeed");

        assert_eq!(listed.servers, vec![first_server.server]);
    }

    #[tokio::test]
    async fn list_rejects_invalid_access_token() {
        let state = state();

        assert!(list(&state, "not-a-token").await.is_err());
    }

    #[tokio::test]
    async fn owner_can_create_server_invite() {
        let server_store = Arc::new(InMemoryServerStore::default());
        let state = state_with_store(server_store.clone());
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "invite_owner".to_owned(),
                email: "invite-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("registration should succeed");
        let server = create(
            &state,
            &auth.access_token,
            CreateServerRequest {
                name: "Invite Hub".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        let response = create_invite(
            &state,
            &auth.access_token,
            server.server.id.clone(),
            CreateServerInviteRequest {
                max_uses: Some(5),
                expires_in_days: Some(3),
            },
        )
        .await
        .expect("invite creation should succeed");
        let invites = server_store
            .invites_for_tests()
            .expect("invites should be readable");

        assert_eq!(invites.len(), 1);
        assert_eq!(response.code, invites[0].id.to_string());
        assert_eq!(invites[0].server_id.to_string(), server.server.id);
        assert_eq!(invites[0].creator_user_id.to_string(), auth.user.id);
        assert_eq!(invites[0].max_uses, Some(5));
        assert!(invites[0].expires_at.is_some());
        assert!(invites[0].created_at <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn non_owner_cannot_create_server_invite() {
        let state = state();
        let owner_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "owner_user".to_owned(),
                email: "owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("owner registration should succeed");
        let guest_auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "guest_user".to_owned(),
                email: "guest@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("guest registration should succeed");
        let server = create(
            &state,
            &owner_auth.access_token,
            CreateServerRequest {
                name: "Private".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        let error = create_invite(
            &state,
            &guest_auth.access_token,
            server.server.id,
            CreateServerInviteRequest {
                max_uses: None,
                expires_in_days: None,
            },
        )
        .await
        .expect_err("non-owner invite creation should fail");

        assert!(matches!(
            error,
            crate::features::servers::error::ServerError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn create_invite_rejects_invalid_settings() {
        let state = state();
        let auth = auth_application::register(
            &state,
            RegisterRequest {
                nickname: "invalid_invite_owner".to_owned(),
                email: "invalid-invite-owner@example.com".to_owned(),
                password: "password123".to_owned(),
                accepts_policies: true,
            },
        )
        .await
        .expect("registration should succeed");
        let server = create(
            &state,
            &auth.access_token,
            CreateServerRequest {
                name: "Validation".to_owned(),
            },
        )
        .await
        .expect("server creation should succeed");

        assert!(
            create_invite(
                &state,
                &auth.access_token,
                server.server.id.clone(),
                CreateServerInviteRequest {
                    max_uses: Some(0),
                    expires_in_days: None,
                },
            )
            .await
            .is_err()
        );
        assert!(
            create_invite(
                &state,
                &auth.access_token,
                server.server.id,
                CreateServerInviteRequest {
                    max_uses: None,
                    expires_in_days: Some(366),
                },
            )
            .await
            .is_err()
        );
    }
}
