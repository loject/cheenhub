//! Server application flows.

use cheenhub_contracts::rest::{
    CreateServerRequest, CreateServerResponse, ListServersResponse, ServerSummary,
};
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

fn server_summary(server: &Server) -> ServerSummary {
    ServerSummary {
        id: server.id.to_string(),
        name: server.name.clone(),
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

    use cheenhub_contracts::rest::{CreateServerRequest, RegisterRequest};

    use super::{create, list};
    use crate::features::auth::application as auth_application;
    use crate::features::auth::infrastructure::InMemoryAuthStore;
    use crate::features::auth::security::keys::AuthKeys;
    use crate::features::servers::infrastructure::InMemoryServerStore;
    use crate::http::AppState;

    fn state() -> AppState {
        AppState {
            auth_store: Arc::new(InMemoryAuthStore::default()),
            server_store: Arc::new(InMemoryServerStore::default()),
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
}
