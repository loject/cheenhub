//! Text chat attachment application flows.

use cheenhub_contracts::realtime::ChatImageUploadResponse;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{TextChatApplicationError, ensure_room_text_available, parse_id};
use crate::features::text_chat::domain::{ChatAttachment, NewChatAttachment};
use crate::state::AppState;

const MAX_CHAT_IMAGE_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

/// Uploads one image attachment for a text chat room.
pub(crate) async fn upload_chat_image(
    state: &AppState,
    user_id: &Uuid,
    server_id: String,
    room_id: String,
    original_filename: Option<String>,
    bytes: &[u8],
) -> Result<ChatImageUploadResponse, TextChatApplicationError> {
    let server_id = parse_id(&server_id, "Сервер не найден.")?;
    let room_id = parse_id(&room_id, "Комната не найдена.")?;
    ensure_room_text_available(state, user_id, &server_id, &room_id).await?;

    let validated = validate_chat_image(bytes)?;
    let attachment_id = Uuid::new_v4();
    let object_key = format!(
        "chat-images/{server_id}/{room_id}/{attachment_id}.{}",
        validated.extension
    );
    let Some(bucket) = state.chat_attachment_object_store.bucket() else {
        tracing::warn!(
            server_id = %server_id,
            room_id = %room_id,
            user_id = %user_id,
            "rejected chat image upload because S3 storage is not configured"
        );
        return Err(TextChatApplicationError::Misconfigured {
            feature: "chat_images_s3",
            missing: vec![
                "CHAT_IMAGES_S3_ENDPOINT",
                "CHAT_IMAGES_S3_REGION",
                "CHAT_IMAGES_S3_BUCKET",
                "CHAT_IMAGES_S3_ACCESS_KEY_ID",
                "CHAT_IMAGES_S3_SECRET_ACCESS_KEY",
            ],
            message: "Загрузка изображений пока не настроена.".to_owned(),
        });
    };
    let bucket = bucket.to_owned();

    state
        .chat_attachment_object_store
        .put_object(&object_key, validated.content_type, bytes.to_vec())
        .await
        .map_err(|error| {
            tracing::error!(
                attachment_id = %attachment_id,
                server_id = %server_id,
                room_id = %room_id,
                user_id = %user_id,
                object_key = %object_key,
                error = ?error,
                "failed to upload chat image to object storage"
            );
            TextChatApplicationError::Internal(error)
        })?;

    let attachment = NewChatAttachment {
        id: attachment_id,
        server_id,
        room_id,
        uploader_user_id: *user_id,
        message_id: None,
        bucket,
        object_key: object_key.clone(),
        content_type: validated.content_type.to_owned(),
        byte_size: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
        width: i32::try_from(validated.width).unwrap_or(i32::MAX),
        height: i32::try_from(validated.height).unwrap_or(i32::MAX),
        sha256: sha256_hex(bytes),
        original_filename: original_filename.and_then(clean_filename),
    };

    state
        .text_chat_store
        .insert_chat_attachment(attachment.clone())
        .await
        .map_err(|error| {
            tracing::error!(
                attachment_id = %attachment_id,
                server_id = %server_id,
                room_id = %room_id,
                user_id = %user_id,
                object_key = %object_key,
                %error,
                "failed to persist chat image metadata"
            );
            TextChatApplicationError::Internal(error)
        })?;

    tracing::info!(
        attachment_id = %attachment_id,
        server_id = %server_id,
        room_id = %room_id,
        user_id = %user_id,
        byte_size = bytes.len(),
        content_type = validated.content_type,
        width = validated.width,
        height = validated.height,
        "uploaded chat image"
    );

    Ok(ChatImageUploadResponse {
        id: attachment_id.to_string(),
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        content_type: validated.content_type.to_owned(),
        byte_size: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
        width: i32::try_from(validated.width).unwrap_or(i32::MAX),
        height: i32::try_from(validated.height).unwrap_or(i32::MAX),
    })
}

/// Loads one image attachment after checking room visibility.
pub(crate) async fn chat_image(
    state: &AppState,
    user_id: &Uuid,
    attachment_id: String,
) -> Result<(ChatAttachment, Vec<u8>), TextChatApplicationError> {
    let attachment_id = parse_id(&attachment_id, "Изображение не найдено.")?;
    let attachment = state
        .text_chat_store
        .find_chat_attachment(&attachment_id)
        .await
        .map_err(TextChatApplicationError::Internal)?
        .ok_or_else(|| TextChatApplicationError::NotFound("Изображение не найдено.".to_owned()))?;
    ensure_room_text_available(state, user_id, &attachment.server_id, &attachment.room_id).await?;

    let object = state
        .chat_attachment_object_store
        .get_object(&attachment.object_key)
        .await
        .map_err(|error| {
            tracing::error!(
                attachment_id = %attachment.id,
                server_id = %attachment.server_id,
                room_id = %attachment.room_id,
                user_id = %user_id,
                object_key = %attachment.object_key,
                error = ?error,
                "failed to read chat image from object storage"
            );
            TextChatApplicationError::Internal(error)
        })?;

    tracing::debug!(
        attachment_id = %attachment.id,
        server_id = %attachment.server_id,
        room_id = %attachment.room_id,
        uploader_user_id = %attachment.uploader_user_id,
        user_id = %user_id,
        bucket = %attachment.bucket,
        stored_byte_size = attachment.byte_size,
        width = attachment.width,
        height = attachment.height,
        sha256 = %attachment.sha256,
        original_filename = attachment.original_filename.as_deref().unwrap_or(""),
        created_at = %attachment.created_at,
        byte_size = object.bytes.len(),
        content_type = %object.content_type,
        "serving chat image through backend proxy"
    );

    Ok((attachment, object.bytes))
}

struct ValidatedChatImage {
    content_type: &'static str,
    extension: &'static str,
    width: u32,
    height: u32,
}

fn validate_chat_image(bytes: &[u8]) -> Result<ValidatedChatImage, TextChatApplicationError> {
    if bytes.is_empty() {
        tracing::warn!("rejected empty chat image upload");
        return Err(TextChatApplicationError::BadRequest(
            "Выбери изображение для отправки.".to_owned(),
        ));
    }
    if bytes.len() > MAX_CHAT_IMAGE_UPLOAD_BYTES {
        tracing::warn!(
            input_bytes = bytes.len(),
            "rejected oversized chat image upload"
        );
        return Err(TextChatApplicationError::BadRequest(
            "Изображение слишком большое. Загрузи файл до 10 МБ.".to_owned(),
        ));
    }

    let format = image::guess_format(bytes).map_err(|error| {
        tracing::warn!(%error, input_bytes = bytes.len(), "rejected chat upload with unknown image format");
        TextChatApplicationError::BadRequest("Не удалось прочитать изображение.".to_owned())
    })?;
    let (content_type, extension) = match format {
        image::ImageFormat::Jpeg => ("image/jpeg", "jpg"),
        image::ImageFormat::Png => ("image/png", "png"),
        image::ImageFormat::WebP => ("image/webp", "webp"),
        image::ImageFormat::Gif => ("image/gif", "gif"),
        _ => {
            tracing::warn!(?format, "rejected unsupported chat image format");
            return Err(TextChatApplicationError::BadRequest(
                "Поддерживаются только JPEG, PNG, WebP и GIF.".to_owned(),
            ));
        }
    };

    let decoded = image::load_from_memory(bytes).map_err(|error| {
        tracing::warn!(%error, input_bytes = bytes.len(), "rejected invalid chat image upload");
        TextChatApplicationError::BadRequest("Не удалось прочитать изображение.".to_owned())
    })?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        tracing::warn!(width, height, "rejected empty-dimension chat image");
        return Err(TextChatApplicationError::BadRequest(
            "Изображение пустое.".to_owned(),
        ));
    }

    Ok(ValidatedChatImage {
        content_type,
        extension,
        width,
        height,
    })
}

fn clean_filename(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(255).collect())
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
