//! Запросы удаления и восстановления аккаунта.

use cheenhub_contracts::rest::AccountRestoreRequest;

use super::{api, storage};

/// Начинает удаление аккаунта и очищает локальную сессию после подтверждения сервера.
pub(crate) async fn delete_account() -> Result<(), String> {
    let access_token = api::fresh_access_token().await?;
    let response = api::delete("/auth/me")
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером. Попробуй ещё раз.".to_owned())?;
    if !response.status().is_success() {
        return Err(api::read_error(response).await);
    }
    storage::clear();
    Ok(())
}

/// Восстанавливает аккаунт после явного подтверждения пользователем.
pub(crate) async fn restore_account(token: String) -> Result<(), String> {
    let response = api::post("/auth/account/restore")
        .json(&AccountRestoreRequest { token })
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером. Попробуй ещё раз.".to_owned())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(api::read_error(response).await)
    }
}
