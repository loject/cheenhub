//! Вспомогательные функции валидации текстового чата.

const MAX_MESSAGE_BODY_CHARS: usize = 2000;

/// Проверенное тело сообщения текстового чата.
pub(crate) struct ValidMessageBody {
    /// Обрезанное тело сообщения.
    pub(crate) body: String,
}

/// Проверяет и нормализует тело сообщения.
pub(crate) fn message_body(body: String) -> Result<ValidMessageBody, &'static str> {
    let body = body.trim().to_owned();
    if body.is_empty() {
        return Err("Сообщение не может быть пустым.");
    }
    if body.chars().count() > MAX_MESSAGE_BODY_CHARS {
        return Err("Сообщение слишком длинное.");
    }

    Ok(ValidMessageBody { body })
}
