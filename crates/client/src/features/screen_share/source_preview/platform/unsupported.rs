//! Заглушка превью источников для неподдерживаемых платформ.

use super::super::{MonitorPreview, PreviewLoadError};

/// Сообщает, что выбор физического монитора через превью недоступен.
pub const fn selection_available() -> bool {
    false
}

/// Возвращает пустой список, когда получение превью платформой не поддерживается.
pub async fn load_previews() -> Result<Vec<MonitorPreview>, PreviewLoadError> {
    Ok(Vec::new())
}
