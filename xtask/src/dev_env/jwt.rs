//! Безопасная подготовка пары Ed25519 для локального JWT.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

use super::env_file::EnvFile;
use crate::XtaskResult;

const PRIVATE: &str = "JWT_ED25519_PRIVATE_KEY_BASE64";
const PUBLIC: &str = "CHEENHUB_JWT_PUBLIC_KEY_BASE64";
const BACKEND_ID: &str = "JWT_KEY_ID";
const CLIENT_ID: &str = "CHEENHUB_JWT_KEY_ID";
const DEFAULT_ID: &str = "dev-ed25519-1";

/// Добавляет отсутствующие JWT-настройки, не заменяя существующий ключ.
pub(super) fn ensure(env: &mut EnvFile) -> XtaskResult<()> {
    let private = env.nonempty(PRIVATE)?;
    let public = env.nonempty(PUBLIC)?;
    if private.is_none() && public.is_some() {
        return Err(format!(
            "{PUBLIC} exists without {PRIVATE}; refusing to rotate the JWT key"
        ));
    }

    let signing = if let Some(encoded) = &private {
        decode_signing_key(encoded)?
    } else {
        SigningKey::generate(&mut OsRng)
    };
    let expected_public = signing.verifying_key().to_bytes();
    if let Some(encoded) = &public {
        let decoded = STANDARD
            .decode(encoded)
            .map_err(|_| format!("invalid {PUBLIC}: expected base64"))?;
        if decoded.as_slice() != expected_public {
            return Err(format!(
                "{PUBLIC} does not match {PRIVATE}; refusing to replace existing JWT keys"
            ));
        }
    }

    let backend_id = env.nonempty(BACKEND_ID)?;
    let client_id = env.nonempty(CLIENT_ID)?;
    if let (Some(backend), Some(client)) = (&backend_id, &client_id)
        && backend != client
    {
        return Err(format!("{BACKEND_ID} conflicts with {CLIENT_ID}"));
    }
    let key_id = backend_id
        .or(client_id)
        .unwrap_or_else(|| DEFAULT_ID.to_owned());
    if key_id.contains(['\r', '\n']) {
        return Err("invalid JWT key ID".into());
    }

    if private.is_none() {
        env.set(PRIVATE, &STANDARD.encode(signing.to_bytes()))?;
    }
    if public.is_none() {
        env.set(PUBLIC, &STANDARD.encode(expected_public))?;
    }
    if env.nonempty(BACKEND_ID)?.is_none() {
        env.set(BACKEND_ID, &key_id)?;
    }
    if env.nonempty(CLIENT_ID)?.is_none() {
        env.set(CLIENT_ID, &key_id)?;
    }
    Ok(())
}

fn decode_signing_key(encoded: &str) -> XtaskResult<SigningKey> {
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| format!("invalid {PRIVATE}: expected base64"))?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| format!("invalid {PRIVATE}: expected 32 bytes"))?;
    Ok(SigningKey::from_bytes(&seed))
}

#[cfg(test)]
mod tests;
