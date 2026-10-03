//! Временное вложение общей формы сообщения.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Изображение, ожидающее отправки вместе с черновиком.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingImageAttachment {
    /// Имя, показываемое пользователю.
    pub(crate) display_name: String,
    /// Исходное имя файла, если оно известно платформе.
    pub(crate) file_name: Option<String>,
    /// Размер содержимого в байтах.
    pub(crate) byte_size: usize,
    /// Содержимое изображения до первой успешной загрузки.
    pub(crate) bytes: Vec<u8>,
    /// Идентификатор уже загруженного изображения для повторной отправки.
    pub(crate) uploaded_id: Option<String>,
    /// Безопасный data URL для локального thumbnail без platform API.
    pub(crate) preview_data_url: String,
}

/// Создаёт ожидающее вложение после локальной проверки размера.
pub(crate) fn pending_image_attachment(
    file_name: Option<String>,
    bytes: Vec<u8>,
    max_bytes: usize,
) -> Result<PendingImageAttachment, String> {
    if bytes.is_empty() {
        return Err("Выбранное изображение пустое.".to_owned());
    }
    if bytes.len() > max_bytes {
        return Err(format!(
            "Изображение слишком большое. Максимум — {} МБ.",
            max_bytes / (1024 * 1024)
        ));
    }
    let display_name = file_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Изображение из буфера")
        .to_owned();
    let content_type = image_content_type(&bytes)
        .ok_or_else(|| "Выберите изображение PNG, JPEG, GIF или WebP.".to_owned())?;
    let preview_data_url = format!("data:{content_type};base64,{}", BASE64.encode(&bytes));
    Ok(PendingImageAttachment {
        display_name,
        file_name,
        byte_size: bytes.len(),
        bytes,
        uploaded_id: None,
        preview_data_url,
    })
}

/// Возвращает тип поддерживаемого изображения по его сигнатуре.
pub(crate) fn image_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

/// Проверяет, что форма содержит данные для отправки и не занята операцией.
pub(crate) fn can_send_message(draft: &str, has_attachment: bool, busy: bool) -> bool {
    !busy && (has_attachment || !draft.trim().is_empty())
}

/// Форматирует размер вложения для компактного preview.
pub(crate) fn format_attachment_size(byte_size: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = KIB * KIB;
    if byte_size < MIB {
        format!("{} КБ", byte_size.div_ceil(KIB))
    } else {
        format!("{:.1} МБ", byte_size as f64 / MIB as f64)
    }
}

#[cfg(test)]
mod tests;
