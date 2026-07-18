//! Платформенный доступ к системному буферу обмена.

mod native;

/// Копирует текст в системный буфер обмена.
pub(crate) async fn copy_text(text: String) -> Result<(), String> {
    native::copy_text(text).await
}
