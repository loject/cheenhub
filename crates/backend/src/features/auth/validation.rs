//! Валидация входных данных аутентификации.

/// Нормализованный ввод для регистрации.
#[derive(Debug, Clone)]
pub(crate) struct ValidRegister {
    /// Никнейм пользователя.
    pub(crate) nickname: String,
    /// Исходный email.
    pub(crate) email: String,
    /// Нормализованный email для поиска и уникальности.
    pub(crate) email_normalized: String,
    /// Обычный пароль.
    pub(crate) password: String,
}

/// Нормализованный ввод для входа.
#[derive(Debug, Clone)]
pub(crate) struct ValidLogin {
    /// Нормализованный email для поиска.
    pub(crate) email_normalized: String,
    /// Обычный пароль.
    pub(crate) password: String,
}

/// Нормализованный ввод для запроса сброса пароля.
#[derive(Debug, Clone)]
pub(crate) struct ValidPasswordResetRequest {
    /// Нормализованный email для поиска.
    pub(crate) email_normalized: String,
}

/// Нормализованный ввод для подтверждения сброса пароля.
#[derive(Debug, Clone)]
pub(crate) struct ValidPasswordResetConfirm {
    /// Непрозрачный токен сброса.
    pub(crate) token: String,
    /// Новый обычный пароль.
    pub(crate) new_password: String,
}

/// Нормализованный ввод для смены пароля.
#[derive(Debug, Clone)]
pub(crate) struct ValidPasswordChange {
    /// Текущий обычный пароль.
    pub(crate) current_password: String,
    /// Новый обычный пароль.
    pub(crate) new_password: String,
}

/// Нормализованный ввод для обновления текущего пользователя.
#[derive(Debug, Clone)]
pub(crate) struct ValidCurrentUserUpdate {
    /// Никнейм пользователя.
    pub(crate) nickname: String,
}

/// Проверяет и нормализует ввод для регистрации.
pub(crate) fn register(
    nickname: String,
    email: String,
    password: String,
    accepts_policies: bool,
) -> Result<ValidRegister, &'static str> {
    let nickname = nickname.trim().to_owned();
    let email = email.trim().to_owned();
    let email_normalized = email.to_lowercase();

    if !accepts_policies {
        return Err("Нужно принять правила сервиса.");
    }
    if !is_valid_nickname(&nickname) {
        return Err("Никнейм должен быть длиной 3-32 символа и содержать латиницу, цифры или _.");
    }
    if !is_valid_email(&email_normalized) {
        return Err("Укажи корректный email.");
    }
    if !(8..=128).contains(&password.chars().count()) {
        return Err("Пароль должен быть длиной от 8 до 128 символов.");
    }

    Ok(ValidRegister {
        nickname,
        email,
        email_normalized,
        password,
    })
}

/// Проверяет и нормализует ввод для входа.
pub(crate) fn login(email: String, password: String) -> Result<ValidLogin, &'static str> {
    let email_normalized = email.trim().to_lowercase();

    if !is_valid_email(&email_normalized) {
        return Err("Укажи корректный email.");
    }
    if password.is_empty() {
        return Err("Укажи пароль.");
    }

    Ok(ValidLogin {
        email_normalized,
        password,
    })
}

/// Проверяет и нормализует ввод для запроса сброса пароля.
pub(crate) fn password_reset_request(
    email: String,
) -> Result<ValidPasswordResetRequest, &'static str> {
    let email_normalized = email.trim().to_lowercase();
    if !is_valid_email(&email_normalized) {
        return Err("Укажи корректный email.");
    }

    Ok(ValidPasswordResetRequest { email_normalized })
}

/// Проверяет и нормализует ввод для подтверждения сброса пароля.
pub(crate) fn password_reset_confirm(
    token: String,
    new_password: String,
) -> Result<ValidPasswordResetConfirm, &'static str> {
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Err("Ссылка для сброса пароля недействительна.");
    }
    if !(8..=128).contains(&new_password.chars().count()) {
        return Err("Пароль должен быть длиной от 8 до 128 символов.");
    }

    Ok(ValidPasswordResetConfirm {
        token,
        new_password,
    })
}

/// Проверяет и нормализует ввод для смены пароля текущего пользователя.
pub(crate) fn password_change(
    current_password: String,
    new_password: String,
    new_password_confirmation: String,
) -> Result<ValidPasswordChange, &'static str> {
    if !(8..=128).contains(&new_password.chars().count()) {
        return Err("Новый пароль должен быть длиной от 8 до 128 символов.");
    }
    if new_password != new_password_confirmation {
        return Err("Новый пароль и подтверждение не совпадают.");
    }
    if current_password == new_password {
        return Err("Новый пароль должен отличаться от текущего.");
    }

    Ok(ValidPasswordChange {
        current_password,
        new_password,
    })
}

/// Validates and normalizes current user update input.
pub(crate) fn current_user_update(
    nickname: String,
) -> Result<ValidCurrentUserUpdate, &'static str> {
    let nickname = nickname.trim().to_owned();
    if !is_valid_nickname(&nickname) {
        return Err("Никнейм должен быть длиной 3-32 символа и содержать латиницу, цифры или _.");
    }

    Ok(ValidCurrentUserUpdate { nickname })
}

/// Возвращает, удовлетворяет ли никнейм правилам учетной записи.
pub(crate) fn is_valid_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    (3..=32).contains(&len)
        && nickname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Возвращает, примерно ли email удовлетворяет правилам учетной записи.
pub(crate) fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && email.len() <= 320
}
