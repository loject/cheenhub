//! Проверка access JWT на стороне клиента.

use std::fmt;

use web_time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;

const DEFAULT_KEY_ID: &str = "dev-ed25519-1";
const DEFAULT_PUBLIC_KEY_BASE64: &str = "FyeAHCHdj3LQJcxcJv1Zo3mW8m+kqBGytTetC2NCIBU=";
const REFRESH_SKEW_SECONDS: i64 = 60;

/// Набор claims access JWT, используемый клиентом.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct AccessClaims {
    /// Идентификатор пользователя.
    pub(crate) sub: String,
    /// Никнейм пользователя.
    pub(crate) nickname: String,
    /// Email пользователя.
    pub(crate) email: String,
    /// Unix-временная метка выпуска.
    pub(crate) iat: i64,
    /// Unix-временная метка истечения.
    pub(crate) exp: i64,
    /// Идентификатор сессии.
    pub(crate) session_id: String,
    /// Идентификатор JWT-ключа.
    pub(crate) kid: String,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

/// Ошибка проверки access JWT на стороне клиента.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JwtVerifyError {
    /// Токен имеет неверный формат, claims, алгоритм, kid или подпись.
    InvalidToken,
    /// Встроенный публичный ключ проверки недоступен или имеет неверный формат.
    VerificationKeyUnavailable,
    /// Токен корректно подписан, но срок его действия уже истёк.
    Expired,
}

impl fmt::Display for JwtVerifyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidToken => "Некорректная сессия.",
            Self::VerificationKeyUnavailable => "Ключ проверки сессии недоступен.",
            Self::Expired => "Сессия истекла.",
        })
    }
}

impl std::error::Error for JwtVerifyError {}

/// Возвращает, можно ли использовать access token без немедленного обновления.
pub(crate) fn is_fresh(token: &str) -> bool {
    verify(token)
        .map(|claims| claims.exp > now_seconds() + REFRESH_SKEW_SECONDS)
        .unwrap_or(false)
}

/// Возвращает число секунд до обновления access JWT.
pub(crate) fn seconds_until_refresh(token: &str) -> Result<u32, JwtVerifyError> {
    let claims = match verify(token) {
        Ok(claims) => claims,
        Err(JwtVerifyError::Expired) => return Ok(0),
        Err(error) => return Err(error),
    };
    let seconds = claims.exp - now_seconds() - REFRESH_SKEW_SECONDS;
    Ok(seconds.max(0) as u32)
}

/// Проверяет подписанный access JWT с помощью встроенного публичного ключа.
pub(crate) fn verify(token: &str) -> Result<AccessClaims, JwtVerifyError> {
    let mut parts = token.split('.');
    let header = parts
        .next()
        .ok_or(JwtVerifyError::InvalidToken)?;
    let payload = parts
        .next()
        .ok_or(JwtVerifyError::InvalidToken)?;
    let signature = parts
        .next()
        .ok_or(JwtVerifyError::InvalidToken)?;
    if parts.next().is_some() {
        return Err(JwtVerifyError::InvalidToken);
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header)
        .map_err(|_| JwtVerifyError::InvalidToken)?;
    let parsed_header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| JwtVerifyError::InvalidToken)?;
    if parsed_header.alg != "EdDSA" || parsed_header.kid != active_key_id() {
        return Err(JwtVerifyError::InvalidToken);
    }

    let public_key = STANDARD
        .decode(active_public_key())
        .map_err(|_| JwtVerifyError::VerificationKeyUnavailable)?;
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| JwtVerifyError::VerificationKeyUnavailable)?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key)
            .map_err(|_| JwtVerifyError::VerificationKeyUnavailable)?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| JwtVerifyError::InvalidToken)?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| JwtVerifyError::InvalidToken)?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| JwtVerifyError::InvalidToken)?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| JwtVerifyError::InvalidToken)?;
    let claims: AccessClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| JwtVerifyError::InvalidToken)?;
    if claims.kid != active_key_id() {
        return Err(JwtVerifyError::InvalidToken);
    }
    if claims.exp <= now_seconds() {
        return Err(JwtVerifyError::Expired);
    }

    Ok(claims)
}

fn active_key_id() -> &'static str {
    option_env!("CHEENHUB_JWT_KEY_ID").unwrap_or(DEFAULT_KEY_ID)
}

fn active_public_key() -> &'static str {
    option_env!("CHEENHUB_JWT_PUBLIC_KEY_BASE64").unwrap_or(DEFAULT_PUBLIC_KEY_BASE64)
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
