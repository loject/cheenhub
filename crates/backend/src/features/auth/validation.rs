//! Authentication input validation.

/// Normalized registration input.
#[derive(Debug, Clone)]
pub(crate) struct ValidRegister {
    /// User nickname.
    pub(crate) nickname: String,
    /// Original email.
    pub(crate) email: String,
    /// Normalized email for lookup and uniqueness.
    pub(crate) email_normalized: String,
    /// Plain password.
    pub(crate) password: String,
}

/// Normalized login input.
#[derive(Debug, Clone)]
pub(crate) struct ValidLogin {
    /// Normalized email for lookup.
    pub(crate) email_normalized: String,
    /// Plain password.
    pub(crate) password: String,
}

/// Normalized password reset request input.
#[derive(Debug, Clone)]
pub(crate) struct ValidPasswordResetRequest {
    /// Normalized email for lookup.
    pub(crate) email_normalized: String,
}

/// Normalized password reset confirmation input.
#[derive(Debug, Clone)]
pub(crate) struct ValidPasswordResetConfirm {
    /// Opaque reset token.
    pub(crate) token: String,
    /// New plain password.
    pub(crate) new_password: String,
}

/// Validates and normalizes registration input.
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

/// Validates and normalizes login input.
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

/// Validates and normalizes password reset request input.
pub(crate) fn password_reset_request(
    email: String,
) -> Result<ValidPasswordResetRequest, &'static str> {
    let email_normalized = email.trim().to_lowercase();
    if !is_valid_email(&email_normalized) {
        return Err("Укажи корректный email.");
    }

    Ok(ValidPasswordResetRequest { email_normalized })
}

/// Validates and normalizes password reset confirmation input.
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

/// Returns whether a nickname satisfies account rules.
pub(crate) fn is_valid_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    (3..=32).contains(&len)
        && nickname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Returns whether an email roughly satisfies account rules.
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
