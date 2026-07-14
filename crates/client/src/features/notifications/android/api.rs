//! REST-клиент регистрации Android-установки системных push-уведомлений.

use cheenhub_contracts::rest::{PushPlatform, UpsertPushInstallationRequest};

use crate::features::auth::api::{fresh_access_token, put, read_error};

/// Регистрирует или обновляет Android push-установку текущей auth-сессии.
pub(super) async fn upsert_installation(
    installation_id: &str,
    token: String,
) -> Result<(), String> {
    let access_token = fresh_access_token().await?;
    let response = put(&format!("/push/installations/{installation_id}"))
        .header("Authorization", format!("Bearer {access_token}"))
        .json(&UpsertPushInstallationRequest {
            platform: PushPlatform::Android,
            token,
        })
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером регистрации уведомлений.".to_owned())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(read_error(response).await)
    }
}
