//! Client-side access JWT validation.

use web_time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;

const DEFAULT_KEY_ID: &str = "dev-ed25519-1";
const DEFAULT_PUBLIC_KEY_BASE64: &str = "FyeAHCHdj3LQJcxcJv1Zo3mW8m+kqBGytTetC2NCIBU=";
const REFRESH_SKEW_SECONDS: i64 = 30;

/// Access JWT claims used by the client.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct AccessClaims {
    /// User id.
    pub(crate) sub: String,
    /// User nickname.
    pub(crate) nickname: String,
    /// User email.
    pub(crate) email: String,
    /// Issued-at unix timestamp.
    pub(crate) iat: i64,
    /// Expiration unix timestamp.
    pub(crate) exp: i64,
    /// Session id.
    pub(crate) session_id: String,
    /// JWT key identifier.
    pub(crate) kid: String,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

/// Returns whether an access token can be used without immediate refresh.
pub(crate) fn is_fresh(token: &str) -> bool {
    verify(token)
        .map(|claims| claims.exp > now_seconds() + REFRESH_SKEW_SECONDS)
        .unwrap_or(false)
}

/// Verifies a signed access JWT with the embedded public key.
pub(crate) fn verify(token: &str) -> Result<AccessClaims, String> {
    let mut parts = token.split('.');
    let header = parts
        .next()
        .ok_or_else(|| "Некорректная сессия.".to_owned())?;
    let payload = parts
        .next()
        .ok_or_else(|| "Некорректная сессия.".to_owned())?;
    let signature = parts
        .next()
        .ok_or_else(|| "Некорректная сессия.".to_owned())?;
    if parts.next().is_some() {
        return Err("Некорректная сессия.".to_owned());
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header)
        .map_err(|_| "Некорректная сессия.".to_owned())?;
    let parsed_header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| "Некорректная сессия.".to_owned())?;
    if parsed_header.alg != "EdDSA" || parsed_header.kid != active_key_id() {
        return Err("Некорректная сессия.".to_owned());
    }

    let public_key = STANDARD
        .decode(active_public_key())
        .map_err(|_| "Ключ проверки сессии недоступен.".to_owned())?;
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| "Ключ проверки сессии недоступен.".to_owned())?;
    let verifying_key =
        VerifyingKey::from_bytes(&public_key).map_err(|_| "Ключ проверки сессии недоступен.")?;
    let signature = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| "Некорректная сессия.".to_owned())?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| "Некорректная сессия.".to_owned())?;
    let signature = Signature::from_bytes(&signature);
    verifying_key
        .verify(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| "Некорректная сессия.".to_owned())?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| "Некорректная сессия.".to_owned())?;
    let claims: AccessClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| "Некорректная сессия.".to_owned())?;
    if claims.kid != active_key_id() || claims.exp <= now_seconds() {
        return Err("Сессия истекла.".to_owned());
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
