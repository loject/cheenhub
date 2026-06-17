//! Доменные модели изображений.

use uuid::Uuid;

/// Обработанные байты изображения, готовые к сохранению.
#[derive(Debug, Clone)]
pub(crate) struct NewStoredImage {
    /// Стабильный идентификатор изображения.
    pub(crate) id: Uuid,
    /// Пользователь, которому принадлежит это изображение.
    pub(crate) owner_user_id: Uuid,
    /// Назначение изображения.
    pub(crate) kind: String,
    /// Сохраненный MIME-тип.
    pub(crate) content_type: String,
    /// Ширина в пикселях.
    pub(crate) width: i32,
    /// Высота в пикселях.
    pub(crate) height: i32,
    /// Сохраненный размер в байтах.
    pub(crate) byte_size: i64,
    /// SHA-256-хэш в hex-формате.
    pub(crate) sha256: String,
    /// Имя бэкенда хранения.
    pub(crate) storage_backend: String,
    /// Ключ внешнего объектного хранилища.
    pub(crate) storage_key: Option<String>,
    /// Сохраненные байты изображения.
    pub(crate) data: Option<Vec<u8>>,
}

/// Данные сохраненного изображения.
#[derive(Debug, Clone)]
pub(crate) struct StoredImage {
    /// Стабильный идентификатор изображения.
    pub(crate) id: Uuid,
    /// Пользователь, которому принадлежит это изображение.
    pub(crate) owner_user_id: Uuid,
    /// Назначение изображения.
    pub(crate) kind: String,
    /// Сохраненный MIME-тип.
    pub(crate) content_type: String,
    /// Ширина в пикселях.
    pub(crate) width: i32,
    /// Высота в пикселях.
    pub(crate) height: i32,
    /// Сохраненный размер в байтах.
    pub(crate) byte_size: i64,
    /// Имя бэкенда хранения.
    pub(crate) storage_backend: String,
    /// Сохраненные байты изображения при хранении в базе данных.
    pub(crate) data: Option<Vec<u8>>,
}
