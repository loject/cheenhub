//! Общие контракты выбора изображения.

/// Изображение, выбранное пользователем на текущей платформе.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PickedImage {
    /// Исходное имя файла, если платформа смогла его определить.
    pub(crate) file_name: Option<String>,
    /// Полное содержимое выбранного изображения.
    pub(crate) bytes: Vec<u8>,
}

/// Результат локальной команды выбора изображения.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ImagePickerOutcome {
    /// Пользователь выбрал и приложение прочитало изображение.
    Selected(PickedImage),
    /// Выбор или чтение изображения завершились ошибкой.
    Failed(String),
}

pub(super) fn oversized_image_message(max_bytes: usize) -> String {
    let max_mebibytes = max_bytes / (1024 * 1024);
    format!("Изображение слишком большое. Максимум — {max_mebibytes} МБ.")
}

#[cfg(test)]
mod tests {
    use super::oversized_image_message;

    #[test]
    fn formats_attachment_limit_in_mebibytes() {
        assert_eq!(
            oversized_image_message(8 * 1024 * 1024),
            "Изображение слишком большое. Максимум — 8 МБ."
        );
    }
}
