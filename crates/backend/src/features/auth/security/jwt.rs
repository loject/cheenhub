//! Access JWT signing and verification.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::features::auth::domain::UserAccount;
use crate::features::auth::error::AuthError;
use crate::features::auth::security::keys::AuthKeys;

/// Access JWT claims used by the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
    kid: String,
}

/// Signs a short-lived access JWT for a user session.
pub(crate) fn sign_access_token(
    signing_key: &SigningKey,
    key_id: &str,
    access_token_lifetime_minutes: i64,
    user: &UserAccount,
    session_id: &Uuid,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let header = JwtHeader {
        alg: "EdDSA".to_owned(),
        typ: "JWT".to_owned(),
        kid: key_id.to_owned(),
    };
    let claims = AccessClaims {
        sub: user.id.to_string(),
        nickname: user.nickname.clone(),
        email: user.email.clone(),
        iat: now.timestamp(),
        exp: (now + Duration::minutes(access_token_lifetime_minutes)).timestamp(),
        session_id: session_id.to_string(),
        kid: key_id.to_owned(),
    };
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).map_err(anyhow::Error::from)?);
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).map_err(anyhow::Error::from)?);
    let signing_input = format!("{header}.{payload}");
    let signature = signing_key.sign(signing_input.as_bytes());

    Ok(format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}

/// Verifies an access JWT and returns its claims.
pub(crate) fn verify_access_token(keys: &AuthKeys, token: &str) -> Result<AccessClaims, AuthError> {
    let mut parts = token.split('.');
    let header = parts.next().ok_or_else(invalid_token)?;
    let payload = parts.next().ok_or_else(invalid_token)?;
    let signature = parts.next().ok_or_else(invalid_token)?;
    if parts.next().is_some() {
        return Err(invalid_token());
    }

    let header_bytes = URL_SAFE_NO_PAD
        .decode(header)
        .map_err(|_| invalid_token())?;
    let parsed_header: JwtHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| invalid_token())?;
    if parsed_header.alg != "EdDSA" || parsed_header.kid != keys.key_id {
        return Err(invalid_token());
    }

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature)
        .map_err(|_| invalid_token())?;
    let signature_bytes: [u8; 64] = signature_bytes.try_into().map_err(|_| invalid_token())?;
    let signature = ed25519_dalek::Signature::from_bytes(&signature_bytes);
    let verifying_key = keys.signing_key.verifying_key();
    verifying_key
        .verify_strict(format!("{header}.{payload}").as_bytes(), &signature)
        .map_err(|_| invalid_token())?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| invalid_token())?;
    let claims: AccessClaims =
        serde_json::from_slice(&payload_bytes).map_err(|_| invalid_token())?;
    if claims.kid != keys.key_id || claims.exp <= Utc::now().timestamp() {
        return Err(AuthError::Unauthorized(
            "Сессия истекла. Войди снова.".to_owned(),
        ));
    }

    Ok(claims)
}

fn invalid_token() -> AuthError {
    AuthError::Unauthorized("Сессия истекла. Войди снова.".to_owned())
}
