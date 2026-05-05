//! Ed25519 JWT key handling.

use anyhow::{Context, anyhow};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;

/// Active authentication signing keys.
#[derive(Clone)]
pub(crate) struct AuthKeys {
    /// Active key identifier embedded into JWT headers and claims.
    pub(crate) key_id: String,
    /// Active Ed25519 signing key.
    pub(crate) signing_key: SigningKey,
}

impl AuthKeys {
    /// Builds authentication keys from backend configuration.
    pub(crate) fn from_config(private_key_base64: &str, key_id: String) -> anyhow::Result<Self> {
        if key_id.trim().is_empty() {
            return Err(anyhow!("JWT_KEY_ID must not be empty"));
        }

        let bytes = STANDARD
            .decode(private_key_base64)
            .context("JWT_ED25519_PRIVATE_KEY_BASE64 must be valid base64")?;
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("JWT_ED25519_PRIVATE_KEY_BASE64 must decode to 32 bytes"))?;

        Ok(Self {
            key_id,
            signing_key: SigningKey::from_bytes(&seed),
        })
    }

    /// Builds deterministic authentication keys for tests.
    #[cfg(test)]
    pub(crate) fn generate_for_tests() -> Self {
        Self {
            key_id: "test-key".to_owned(),
            signing_key: SigningKey::from_bytes(&[7; 32]),
        }
    }
}
