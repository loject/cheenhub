//! Принятие приглашения сервера.

use cheenhub_contracts::rest::AcceptServerInviteResponse;
use chrono::Utc;
use uuid::Uuid;

use super::support::{map_auth_error, server_summary};
use crate::features::auth::application as auth_application;
use crate::features::servers::error::ServerError;
use crate::features::servers::infrastructure::AcceptInviteOutcome;
use crate::state::AppState;

/// Принимает приглашение сервера для текущего пользователя.
pub(crate) async fn accept_invite(
    state: &AppState,
    access_token: &str,
    code: String,
) -> Result<AcceptServerInviteResponse, ServerError> {
    let user = auth_application::me(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let user_id = Uuid::parse_str(&user.id)
        .map_err(|_| ServerError::Unauthorized("Сессия истекла. Войди снова.".to_owned()))?;
    let code = Uuid::parse_str(&code)
        .map_err(|_| ServerError::BadRequest("Приглашение не найдено.".to_owned()))?;
    let Some(invite) = state
        .server_store
        .find_server_invite(&code)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
    };

    if invite
        .expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
    {
        return Err(ServerError::BadRequest(
            "Срок действия приглашения истек.".to_owned(),
        ));
    }
    if invite.revoked_at.is_some() {
        return Err(ServerError::BadRequest("Приглашение отозвано.".to_owned()));
    }

    let Some(server) = state
        .server_store
        .find_server(&invite.server_id)
        .await
        .map_err(ServerError::Internal)?
    else {
        return Err(ServerError::NotFound("Сервер не найден.".to_owned()));
    };

    let is_owner = server.owner_user_id == user_id;
    let active_member = state
        .server_store
        .find_active_server_member(&server.id, &user_id)
        .await
        .map_err(ServerError::Internal)?
        .is_some();

    if is_owner || active_member {
        return Ok(AcceptServerInviteResponse {
            server: server_summary(state, &server, &user_id, true).await,
            already_member: true,
        });
    }
    let now = Utc::now();
    if let Some(exclusion) = state
        .server_store
        .find_active_server_member_exclusion(&server.id, &user_id, now)
        .await
        .map_err(ServerError::Internal)?
    {
        tracing::warn!(
            server_id = %server.id,
            user_id = %user_id,
            excluded_until = %exclusion.expires_at,
            "blocked invite acceptance for excluded server member"
        );
        return Err(ServerError::BadRequest(format!(
            "Ты временно исключен с сервера до {}.",
            exclusion.expires_at.to_rfc3339()
        )));
    }

    let outcome = state
        .server_store
        .accept_server_invite(&invite.id, &user_id, now)
        .await
        .map_err(ServerError::Internal)?;
    let already_member = match outcome {
        AcceptInviteOutcome::Accepted => {
            tracing::info!(
                server_id = %server.id,
                invite_id = %invite.id,
                user_id = %user_id,
                "atomically accepted server invite"
            );
            false
        }
        AcceptInviteOutcome::AlreadyMember => true,
        AcceptInviteOutcome::NotFound => {
            log_rejected_invite(&invite.id, &user_id, outcome);
            return Err(ServerError::NotFound("Приглашение не найдено.".to_owned()));
        }
        AcceptInviteOutcome::Expired => {
            log_rejected_invite(&invite.id, &user_id, outcome);
            return Err(ServerError::BadRequest(
                "Срок действия приглашения истек.".to_owned(),
            ));
        }
        AcceptInviteOutcome::Revoked => {
            log_rejected_invite(&invite.id, &user_id, outcome);
            return Err(ServerError::BadRequest("Приглашение отозвано.".to_owned()));
        }
        AcceptInviteOutcome::Exhausted => {
            log_rejected_invite(&invite.id, &user_id, outcome);
            return Err(ServerError::BadRequest(
                "Лимит использований приглашения исчерпан.".to_owned(),
            ));
        }
    };

    Ok(AcceptServerInviteResponse {
        server: server_summary(state, &server, &user_id, true).await,
        already_member,
    })
}

fn log_rejected_invite(invite_id: &Uuid, user_id: &Uuid, outcome: AcceptInviteOutcome) {
    tracing::warn!(%invite_id, %user_id, ?outcome, "rejected server invite acceptance");
}
