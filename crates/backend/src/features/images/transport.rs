//! HTTP-обработчики публичных изображений.

use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use uuid::Uuid;

use crate::features::auth::error::AuthError;
use crate::features::images::application;
use crate::state::AppState;

/// Собирает маршруты публичных изображений.
pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/{image_id}", get(image))
}

/// Возвращает одно публичное изображение.
pub(crate) async fn image(
    State(state): State<AppState>,
    Path(image_id): Path<Uuid>,
) -> Result<Response, AuthError> {
    let image = application::public_image(&state, &image_id).await?;
    tracing::debug!(
        image_id = %image.id,
        owner_user_id = %image.owner_user_id,
        width = image.width,
        height = image.height,
        byte_size = image.byte_size,
        "serving public image"
    );
    let Some(data) = image.data else {
        tracing::error!(
            image_id = %image.id,
            owner_user_id = %image.owner_user_id,
            storage_backend = %image.storage_backend,
            "public image bytes are not available from configured storage backend"
        );
        return Err(AuthError::Internal(anyhow::anyhow!(
            "image bytes are not available"
        )));
    };

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, image.content_type),
            (
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable".to_owned(),
            ),
        ],
        data,
    )
        .into_response())
}
