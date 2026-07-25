//! Управление внешними учетными записями текущего пользователя.

use cheenhub_contracts::rest::{LinkedAccount, LinkedAccountsResponse, OAuthProvider};
use uuid::Uuid;

use super::{expired_session, me};
use crate::features::auth::domain::OAuthAccount;
use crate::features::auth::error::AuthError;
use crate::state::AppState;

const GOOGLE_PROVIDER: &str = "google";

/// Перечисляет внешние аккаунты, привязанные к текущему пользователю.
pub(crate) async fn linked_accounts(
    state: &AppState,
    access_token: &str,
) -> Result<LinkedAccountsResponse, AuthError> {
    let user = me(state, access_token).await?;
    let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
    let accounts = state
        .auth_store
        .list_oauth_accounts(&user_id)
        .await
        .map_err(AuthError::Internal)?
        .iter()
        .map(linked_account)
        .collect();

    Ok(LinkedAccountsResponse { accounts })
}

/// Отвязывает Google от текущего пользователя, если остается другой способ входа.
pub(crate) async fn unlink_google(
    state: &AppState,
    access_token: &str,
) -> Result<LinkedAccountsResponse, AuthError> {
    let user = me(state, access_token).await?;
    if !user.has_password {
        return Err(AuthError::BadRequest(
            "Сначала добавь пароль, чтобы не потерять доступ к аккаунту.".to_owned(),
        ));
    }
    let user_id = Uuid::parse_str(&user.id).map_err(|_| expired_session())?;
    state
        .auth_store
        .delete_oauth_account(GOOGLE_PROVIDER, &user_id)
        .await
        .map_err(AuthError::Internal)?;
    linked_accounts(state, access_token).await
}

pub(super) fn linked_account(account: &OAuthAccount) -> LinkedAccount {
    LinkedAccount {
        provider: OAuthProvider::Google,
        email: account.email.clone(),
        display_name: account.display_name.clone(),
        linked_at: account.linked_at.to_rfc3339(),
    }
}
