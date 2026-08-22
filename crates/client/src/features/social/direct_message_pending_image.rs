//! Временное изображение, ожидающее отправки в одном личном диалоге.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Изображение, которое будет загружено перед отправкой личного сообщения.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PendingDirectMessageImage {
    /// Имя, отображаемое в preview.
    pub(super) display_name: String,
    /// Исходное имя файла, если оно известно.
    pub(super) file_name: Option<String>,
    /// Размер изображения в байтах.
    pub(super) byte_size: usize,
    /// Байты изображения до успешной загрузки.
    pub(super) bytes: Vec<u8>,
    /// Идентификатор уже загруженного изображения для безопасной повторной отправки.
    pub(super) uploaded_id: Option<String>,
    /// Локальный thumbnail без обращения к platform API.
    pub(super) preview_data_url: String,
}

/// Создаёт ожидающее изображение, предварительно сверяя размер и сигнатуру с серверным контрактом.
pub(super) fn pending_direct_message_image(
    file_name: Option<String>,
    bytes: Vec<u8>,
    max_bytes: usize,
) -> Result<PendingDirectMessageImage, String> {
    if bytes.is_empty() {
        return Err("Выбранное изображение пустое.".to_owned());
    }
    if bytes.len() > max_bytes {
        return Err(format!(
            "Изображение слишком большое. Максимум — {} МБ.",
            max_bytes / (1024 * 1024)
        ));
    }
    let content_type = image_content_type(&bytes)
        .ok_or_else(|| "Выберите изображение PNG, JPEG, GIF или WebP.".to_owned())?;
    let display_name = file_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Изображение из буфера")
        .to_owned();
    let preview_data_url = format!("data:{content_type};base64,{}", BASE64.encode(&bytes));
    Ok(PendingDirectMessageImage {
        display_name,
        file_name,
        byte_size: bytes.len(),
        bytes,
        uploaded_id: None,
        preview_data_url,
    })
}

/// Возвращает поддерживаемый сервером MIME type по сигнатуре файла.
pub(super) fn image_content_type(bytes: &[u8]) -> Option<&'static str> {
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

/// Форматирует размер для компактного preview.
pub(super) fn format_attachment_size(byte_size: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = KIB * KIB;
    if byte_size < MIB {
        format!("{} КБ", byte_size.div_ceil(KIB))
    } else {
        format!("{:.1} МБ", byte_size as f64 / MIB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::{image_content_type, pending_direct_message_image};

    #[test]
    fn creates_thumbnail_for_supported_image() {
        let image =
            pending_direct_message_image(None, b"\x89PNG\r\n\x1a\nimage".to_vec(), 1024).unwrap();
        assert!(image.preview_data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn rejects_unsupported_image_signature() {
        assert!(pending_direct_message_image(None, b"not image".to_vec(), 1024).is_err());
        assert_eq!(image_content_type(b"GIF89a"), Some("image/gif"));
    }
}
