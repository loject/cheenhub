//! Помощники приложения для изображений.

use image::GenericImageView;
use image::ImageEncoder;
use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::imageops::FilterType;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::features::auth::error::AuthError;
use crate::features::images::domain::{NewStoredImage, StoredImage};
use crate::state::AppState;

const USER_AVATAR_KIND: &str = "user_avatar";
const SERVER_AVATAR_KIND: &str = "server_avatar";
const PNG_CONTENT_TYPE: &str = "image/png";
const DATABASE_STORAGE_BACKEND: &str = "database";
const AVATAR_SIZE_PX: u32 = 512;
const MAX_AVATAR_UPLOAD_BYTES: usize = 8 * 1024 * 1024;
/// Максимальная сторона изображения, которую мы готовы декодировать.
const MAX_DECODE_DIMENSION: u32 = 8192;
/// Жесткий потолок аллокации памяти при декодировании.
const MAX_DECODE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;

/// Декодирует изображение с ограничениями на размеры и аллокацию памяти.
///
/// `image::load_from_memory` разворачивает весь несжатый буфер ДО любой проверки
/// размеров, поэтому крошечный «бомбовый» файл (например, PNG 30000×30000) мог
/// раздуться в гигабайты и привести к OOM/DoS. `ImageReader` с `Limits` применяет
/// ограничения во время декодирования.
pub(crate) fn decode_image_limited(bytes: &[u8]) -> image::ImageResult<image::DynamicImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(image::ImageError::IoError)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODE_DIMENSION);
    limits.max_image_height = Some(MAX_DECODE_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_ALLOC_BYTES);
    reader.limits(limits);
    reader.decode()
}

/// Обработанный аватар пользователя, готовый к сохранению.
pub(crate) struct ProcessedUserAvatar {
    data: Vec<u8>,
}

impl ProcessedUserAvatar {
    /// Возвращает размер сохраненного изображения в байтах.
    pub(crate) fn byte_len(&self) -> usize {
        self.data.len()
    }

    /// Преобразует этот обработанный аватар в полезную нагрузку строки изображения БД.
    pub(crate) fn into_new_stored_image(self, id: Uuid, owner_user_id: Uuid) -> NewStoredImage {
        self.into_new_stored_image_with_kind(id, owner_user_id, USER_AVATAR_KIND)
    }

    /// Преобразует этот обработанный аватар в полезную нагрузку строки аватара сервера.
    pub(crate) fn into_new_server_avatar_image(
        self,
        id: Uuid,
        owner_user_id: Uuid,
    ) -> NewStoredImage {
        self.into_new_stored_image_with_kind(id, owner_user_id, SERVER_AVATAR_KIND)
    }

    fn into_new_stored_image_with_kind(
        self,
        id: Uuid,
        owner_user_id: Uuid,
        kind: &str,
    ) -> NewStoredImage {
        NewStoredImage {
            id,
            owner_user_id,
            kind: kind.to_owned(),
            content_type: PNG_CONTENT_TYPE.to_owned(),
            width: i32::try_from(AVATAR_SIZE_PX).unwrap_or(512),
            height: i32::try_from(AVATAR_SIZE_PX).unwrap_or(512),
            byte_size: i64::try_from(self.data.len()).unwrap_or(i64::MAX),
            sha256: sha256_hex(&self.data),
            storage_backend: DATABASE_STORAGE_BACKEND.to_owned(),
            storage_key: None,
            data: Some(self.data),
        }
    }
}

/// Обрабатывает загрузку аватара пользователя через глобальную очередь обработки изображений.
pub(crate) async fn process_user_avatar(
    state: &AppState,
    user_id: Uuid,
    bytes: &[u8],
) -> Result<ProcessedUserAvatar, AuthError> {
    tracing::debug!(
        user_id = %user_id,
        input_bytes = bytes.len(),
        "waiting for image processing queue"
    );
    let _permit = state
        .image_processing_queue
        .clone()
        .acquire_owned()
        .await
        .map_err(|error| AuthError::Internal(error.into()))?;
    tracing::debug!(
        user_id = %user_id,
        input_bytes = bytes.len(),
        "entered image processing queue"
    );
    let processed = process_avatar(bytes);
    tracing::debug!(user_id = %user_id, "leaving image processing queue");

    processed
}

/// Обрабатывает загрузку аватара сервера через глобальную очередь обработки изображений.
pub(crate) async fn process_server_avatar(
    state: &AppState,
    server_id: Uuid,
    bytes: &[u8],
) -> Result<ProcessedUserAvatar, AuthError> {
    tracing::debug!(
        server_id = %server_id,
        input_bytes = bytes.len(),
        "waiting for image processing queue"
    );
    let _permit = state
        .image_processing_queue
        .clone()
        .acquire_owned()
        .await
        .map_err(|error| AuthError::Internal(error.into()))?;
    tracing::debug!(
        server_id = %server_id,
        input_bytes = bytes.len(),
        "entered image processing queue"
    );
    let processed = process_avatar(bytes);
    tracing::debug!(server_id = %server_id, "leaving image processing queue");

    processed
}

/// Загружает публичное изображение по идентификатору.
pub(crate) async fn public_image(
    state: &AppState,
    image_id: &Uuid,
) -> Result<StoredImage, AuthError> {
    let Some(image) = state
        .image_store
        .find_image(image_id)
        .await
        .map_err(AuthError::Internal)?
    else {
        return Err(AuthError::BadRequest("Изображение не найдено.".to_owned()));
    };
    if !matches!(image.kind.as_str(), USER_AVATAR_KIND | SERVER_AVATAR_KIND)
        || image.content_type != PNG_CONTENT_TYPE
    {
        return Err(AuthError::BadRequest("Изображение не найдено.".to_owned()));
    }

    Ok(image)
}

/// Строит публичный URL аватара по идентификатору изображения.
pub(crate) fn avatar_url(state: &AppState, image_id: &Uuid) -> String {
    format!(
        "{}/images/{}",
        state.cheenhub_api_base_url.trim_end_matches('/'),
        image_id
    )
}

/// Загружает публичные URL аватаров, индексированные по идентификатору пользователя.
pub(crate) async fn avatar_urls_by_user_ids(
    state: &AppState,
    user_ids: impl IntoIterator<Item = Uuid>,
) -> anyhow::Result<HashMap<Uuid, String>> {
    let user_ids = user_ids.into_iter().collect::<HashSet<_>>();
    let image_ids = state
        .auth_store
        .avatar_image_ids_by_user_ids(&user_ids.into_iter().collect::<Vec<_>>())
        .await?;

    Ok(image_ids
        .into_iter()
        .map(|(user_id, image_id)| (user_id, avatar_url(state, &image_id)))
        .collect())
}

fn process_avatar(bytes: &[u8]) -> Result<ProcessedUserAvatar, AuthError> {
    if bytes.is_empty() {
        tracing::warn!("rejected empty avatar upload");
        return Err(AuthError::BadRequest(
            "Выбери изображение для аватара.".to_owned(),
        ));
    }
    if bytes.len() > MAX_AVATAR_UPLOAD_BYTES {
        tracing::warn!(
            input_bytes = bytes.len(),
            "rejected oversized avatar upload"
        );
        return Err(AuthError::BadRequest(
            "Изображение слишком большое. Загрузи файл до 8 МБ.".to_owned(),
        ));
    }

    let decoded = decode_image_limited(bytes).map_err(|error| {
        tracing::warn!(%error, input_bytes = bytes.len(), "rejected invalid avatar image");
        AuthError::BadRequest("Не удалось прочитать изображение.".to_owned())
    })?;
    let (width, height) = decoded.dimensions();
    if width == 0 || height == 0 {
        tracing::warn!(width, height, "rejected empty-dimension avatar image");
        return Err(AuthError::BadRequest("Изображение пустое.".to_owned()));
    }

    let side = width.min(height);
    let cropped = decoded.crop_imm((width - side) / 2, (height - side) / 2, side, side);
    let resized = cropped
        .resize_exact(AVATAR_SIZE_PX, AVATAR_SIZE_PX, FilterType::Lanczos3)
        .to_rgba8();
    let mut data = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut data, CompressionType::Best, PngFilterType::Adaptive);
    encoder
        .write_image(
            resized.as_raw(),
            AVATAR_SIZE_PX,
            AVATAR_SIZE_PX,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|error| {
            tracing::warn!(%error, "failed to encode avatar png");
            AuthError::Internal(error.into())
        })?;

    tracing::debug!(
        input_width = width,
        input_height = height,
        output_bytes = data.len(),
        "processed avatar image"
    );
    Ok(ProcessedUserAvatar { data })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
