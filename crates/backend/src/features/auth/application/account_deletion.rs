//! Удаление аккаунта через tombstone и ограниченное по времени восстановление.

use cheenhub_contracts::rest::{AccountDeletionResponse, AccountRestoreRequest};
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::features::auth::email::{AccountDeletionEmail, EmailError};
use crate::features::auth::error::AuthError;
use crate::features::auth::security::refresh_token;
use crate::state::AppState;

/// Отключает аккаунт и отзывает сессии после отправки ссылки восстановления.
pub(crate) async fn delete_current_user(
    state: &AppState,
    access_token: &str,
) -> Result<AccountDeletionResponse, AuthError> {
    let (user, _) = super::require_current_user(state, access_token).await?;
    let _lifecycle = state.auth_store.lock_account_lifecycle(&user.id).await?;
    super::require_current_user(state, access_token).await?;
    if crate::features::servers::user_owns_servers(state, &user.id).await? {
        tracing::warn!(user_id = %user.id, "rejected account deletion for server owner");
        return Err(AuthError::Conflict(
            "Сначала передай владение своими серверами или удали их, затем повтори удаление аккаунта.".to_owned(),
        ));
    }

    let now = Utc::now();
    let restore_until = now + Duration::days(30);
    let token = refresh_token::generate();
    // Ошибка почтового транспорта оставляет аккаунт активным и не теряет путь восстановления.
    state.auth_mailer.send_account_deletion(AccountDeletionEmail {
        to: user.email.clone(),
        restore_url: format!("{}/restore-account?token={token}", state.cheenhub_client_base_url.trim_end_matches('/')),
        restore_until: restore_until.format("%d.%m.%Y %H:%M:%S UTC").to_string(),
    }).await.map_err(|error| {
        tracing::warn!(user_id = %user.id, "account deletion email delivery failed; account remains active");
        match error {
            EmailError::Misconfigured { missing } => AuthError::Misconfigured {
                feature: "account_deletion_email",
                missing,
                message: "Не удалось отправить письмо для восстановления. Аккаунт не удалён. Попробуй позже.".to_owned(),
            },
            EmailError::Internal(_) => AuthError::Internal(anyhow::anyhow!("account deletion notification delivery failed")),
        }
    })?;
    if !state
        .auth_store
        .begin_account_deletion(&user.id, refresh_token::hash(&token), now, restore_until)
        .await?
    {
        return Err(super::expired_session());
    }
    let disconnected_sessions = state.realtime_hub.disconnect_user_sessions(&user.id).await;
    tracing::info!(user_id = %user.id, %restore_until, disconnected_sessions, "account tombstone created and sessions revoked");
    Ok(AccountDeletionResponse {
        restore_until: restore_until.to_rfc3339(),
    })
}

/// Восстанавливает аккаунт по одноразовому секрету строго до окончания срока.
pub(crate) async fn restore_account(
    state: &AppState,
    request: AccountRestoreRequest,
) -> Result<(), AuthError> {
    if request.token.is_empty() || request.token.len() > 256 {
        return Err(invalid_restore_link());
    }
    if !state
        .auth_store
        .restore_account(&refresh_token::hash(&request.token), Utc::now())
        .await?
    {
        tracing::warn!("rejected invalid, consumed or expired account restoration token");
        return Err(invalid_restore_link());
    }
    tracing::info!("account restored through one-time email confirmation; new login required");
    Ok(())
}

pub(super) async fn require_active_account(
    state: &AppState,
    user_id: &Uuid,
) -> Result<(), AuthError> {
    if let Some(deletion) = state.auth_store.account_deletion(user_id).await? {
        tracing::info!(%user_id, requested_at = %deletion.requested_at, "rejected authentication for account tombstone");
        return Err(AuthError::Unauthorized(
            if Utc::now() < deletion.restore_until {
                "Аккаунт удалён. Восстановить его можно по ссылке из письма в течение 30 дней."
                    .to_owned()
            } else {
                "Аккаунт удалён. Срок восстановления истёк.".to_owned()
            },
        ));
    }
    Ok(())
}

/// Завершает удаление просроченных аккаунтов, сохраняя tombstone и стабильный UUID.
pub(crate) async fn run_account_deletion_worker(state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        match state
            .auth_store
            .finalize_expired_account_deletions(Utc::now())
            .await
        {
            Ok(count) if count > 0 => tracing::info!(count, "finalized expired account tombstones"),
            Ok(_) => {}
            Err(error) => {
                tracing::error!(%error, "account tombstone finalization failed; retrying on next tick")
            }
        }
    }
}

fn invalid_restore_link() -> AuthError {
    AuthError::BadRequest(
        "Ссылка восстановления недействительна, уже использована или срок 30 дней истёк."
            .to_owned(),
    )
}
