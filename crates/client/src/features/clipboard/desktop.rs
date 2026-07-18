//! Desktop-реализация текстового буфера обмена.

/// Копирует текст в системный буфер обмена вне UI-потока.
pub(super) async fn copy_text(text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|_| "Не удалось открыть системный буфер обмена.".to_owned())?;
        clipboard
            .set_text(text)
            .map_err(|_| "Не удалось скопировать ссылку в буфер обмена.".to_owned())
    })
    .await
    .map_err(|_| "Не удалось завершить копирование ссылки.".to_owned())?
}
