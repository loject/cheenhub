//! Поток профиля аватара текущего пользователя.

use cheenhub_contracts::rest::AuthUser;
use uuid::Uuid;

use crate::features::auth::error::AuthError;
use crate::features::images::application as image_application;
use crate::state::AppState;

/// Обновляет изображение аватара текущего пользователя.
pub(crate) async fn update_current_user_avatar(
    state: &AppState,
    access_token: &str,
    bytes: bytes::Bytes,
) -> Result<AuthUser, AuthError> {
    let (user, _) = super::require_current_user(state, access_token).await?;
    tracing::info!(
        user_id = %user.id,
        input_bytes = bytes.len(),
        "processing current user avatar upload"
    );
    let processed = image_application::process_user_avatar(state, user.id, bytes.as_ref()).await?;
    let now = chrono::Utc::now();
    let image_id = Uuid::new_v4();
    let output_bytes = processed.byte_len();
    let image = processed.into_new_stored_image(image_id, user.id);
    state
        .image_store
        .insert_image(image)
        .await
        .map_err(AuthError::Internal)?;
    let updated_user = state
        .auth_store
        .update_user_avatar_image_id(&user.id, image_id, now)
        .await
        .map_err(AuthError::Internal)?
        .ok_or_else(super::expired_session)?;

    crate::features::voice_chat::application::update_user_avatar(
        state,
        &updated_user.id,
        updated_user
            .avatar_image_id
            .map(|id| image_application::avatar_url(state, &id)),
    )
    .await;
    tracing::info!(
        user_id = %updated_user.id,
        image_id = %image_id,
        output_bytes,
        width = 512_u32,
        height = 512_u32,
        "updated current user avatar"
    );

    Ok(super::auth_user(state, &updated_user))
}
