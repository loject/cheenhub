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
    if !valid_nickname(&nickname) {
        return Err("Никнейм должен быть длиной 3-32 символа и содержать латиницу, цифры или _.");
    }
    if !valid_email(&email_normalized) {
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

    if !valid_email(&email_normalized) {
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

fn valid_nickname(nickname: &str) -> bool {
    let len = nickname.chars().count();
    (3..=32).contains(&len)
        && nickname
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && email.len() <= 320
}
