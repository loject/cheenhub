//! Изображения личных сообщений.

use axum::body::Bytes;
use cheenhub_contracts::rest::{DmImageAttachmentSummary, UploadDmImageResponse};
use image::GenericImageView;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::features::auth::application::require_current_user;
use crate::features::images::domain::{NewStoredImage, StoredImage};
use crate::features::social::error::SocialError;
use crate::features::social::support::{load_user_conversation, map_auth_error, parse_id};
use crate::state::AppState;

const DM_IMAGE_KIND_PREFIX: &str = "direct_message_image:";
const MAX_DM_IMAGE_BYTES: usize = 8 * 1024 * 1024;

/// Загружает изображение, которое затем можно прикрепить к личному сообщению.
pub(crate) async fn upload_dm_image(
    state: &AppState,
    access_token: &str,
    conversation_id: String,
    bytes: Bytes,
) -> Result<UploadDmImageResponse, SocialError> {
    let (user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    load_user_conversation(state, &conversation_id, &user.id).await?;
    let image = validate_image(user.id, conversation_id, bytes)?;
    let summary = summary(&image);
    state
        .image_store
        .insert_image(image)
        .await
        .map_err(SocialError::Internal)?;
    tracing::info!(conversation_id = %conversation_id, user_id = %user.id, image_id = %summary.id, "uploaded direct message image");
    Ok(UploadDmImageResponse { image: summary })
}

/// Загружает байты изображения для участника диалога.
pub(crate) async fn dm_image(
    state: &AppState,
    access_token: &str,
    conversation_id: String,
    image_id: String,
) -> Result<StoredImage, SocialError> {
    let (user, _) = require_current_user(state, access_token)
        .await
        .map_err(map_auth_error)?;
    let conversation_id = parse_id(&conversation_id, "Диалог не найден.")?;
    let conversation = load_user_conversation(state, &conversation_id, &user.id).await?;
    let image_id = parse_id(&image_id, "Изображение не найдено.")?;
    if state
        .social_store
        .dm_message_by_image_id(&conversation.id, &image_id)
        .await
        .map_err(SocialError::Internal)?
        .is_none()
    {
        tracing::warn!(conversation_id = %conversation.id, user_id = %user.id, %image_id, "rejected unattached direct message image read");
        return Err(SocialError::NotFound("Изображение не найдено.".to_owned()));
    }
    let image = state
        .image_store
        .find_image(&image_id)
        .await
        .map_err(SocialError::Internal)?
        .filter(|image| {
            image.kind == dm_image_kind(conversation.id)
                && (image.owner_user_id == conversation.user_low_id
                    || image.owner_user_id == conversation.user_high_id)
        })
        .ok_or_else(|| SocialError::NotFound("Изображение не найдено.".to_owned()))?;
    Ok(image)
}

pub(crate) async fn attachment_summary(
    state: &AppState,
    conversation_id: Uuid,
    image_id: Option<Uuid>,
) -> Result<Option<DmImageAttachmentSummary>, SocialError> {
    let Some(image_id) = image_id else {
        return Ok(None);
    };
    Ok(state
        .image_store
        .find_image(&image_id)
        .await
        .map_err(SocialError::Internal)?
        .filter(|image| image.kind == dm_image_kind(conversation_id))
        .map(|image| summary_stored(&image)))
}

pub(super) async fn validate_attachment_owner(
    state: &AppState,
    conversation_id: Uuid,
    image_id: Uuid,
    user_id: Uuid,
) -> Result<(), SocialError> {
    if state
        .social_store
        .dm_message_by_image_id(&conversation_id, &image_id)
        .await
        .map_err(SocialError::Internal)?
        .is_some()
    {
        tracing::warn!(%conversation_id, %image_id, %user_id, "rejected reused direct message image");
        return Err(SocialError::BadRequest(
            "Изображение уже прикреплено к сообщению.".to_owned(),
        ));
    }
    let valid = state
        .image_store
        .find_image(&image_id)
        .await
        .map_err(SocialError::Internal)?
        .is_some_and(|image| {
            image.kind == dm_image_kind(conversation_id) && image.owner_user_id == user_id
        });
    if valid {
        Ok(())
    } else {
        Err(SocialError::BadRequest(
            "Изображение недоступно.".to_owned(),
        ))
    }
}

fn validate_image(
    owner_user_id: Uuid,
    conversation_id: Uuid,
    bytes: Bytes,
) -> Result<NewStoredImage, SocialError> {
    if bytes.is_empty() || bytes.len() > MAX_DM_IMAGE_BYTES {
        tracing::warn!(user_id = %owner_user_id, byte_size = bytes.len(), "rejected direct message image by size");
        return Err(SocialError::BadRequest(
            "Изображение должно быть не больше 8 МБ.".to_owned(),
        ));
    }
    let format = image::guess_format(&bytes).map_err(|_| {
        SocialError::BadRequest("Выберите изображение PNG, JPEG, GIF или WebP.".to_owned())
    })?;
    let content_type = match format {
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        _ => {
            return Err(SocialError::BadRequest(
                "Выберите изображение PNG, JPEG, GIF или WebP.".to_owned(),
            ));
        }
    };
    let decoded = crate::features::images::application::decode_image_limited(&bytes)
        .map_err(|_| SocialError::BadRequest("Не удалось прочитать изображение.".to_owned()))?;
    let (width, height) = decoded.dimensions();
    let id = Uuid::new_v4();
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok(NewStoredImage {
        id,
        owner_user_id,
        kind: dm_image_kind(conversation_id),
        content_type: content_type.to_owned(),
        width: width as i32,
        height: height as i32,
        byte_size: bytes.len() as i64,
        sha256: digest,
        storage_backend: "database".to_owned(),
        storage_key: None,
        data: Some(bytes.to_vec()),
    })
}

fn dm_image_kind(conversation_id: Uuid) -> String {
    format!("{DM_IMAGE_KIND_PREFIX}{conversation_id}")
}

fn summary(image: &NewStoredImage) -> DmImageAttachmentSummary {
    DmImageAttachmentSummary {
        id: image.id.to_string(),
        content_type: image.content_type.clone(),
        width: image.width,
        height: image.height,
    }
}

fn summary_stored(image: &StoredImage) -> DmImageAttachmentSummary {
    DmImageAttachmentSummary {
        id: image.id.to_string(),
        content_type: image.content_type.clone(),
        width: image.width,
        height: image.height,
    }
}

#[cfg(test)]
mod tests {
    use super::dm_image_kind;
    use uuid::Uuid;

    #[test]
    fn image_kind_is_scoped_to_one_direct_conversation() {
        let first_conversation_id = Uuid::new_v4();
        let second_conversation_id = Uuid::new_v4();

        assert_ne!(
            dm_image_kind(first_conversation_id),
            dm_image_kind(second_conversation_id)
        );
        assert!(dm_image_kind(first_conversation_id).ends_with(&first_conversation_id.to_string()));
    }
}
