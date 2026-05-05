//! Refresh token generation and hashing.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

/// Generates a new opaque refresh token.
pub(crate) fn generate() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Returns a stable SHA-256 hex digest for a refresh token.
pub(crate) fn hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut encoded = String::with_capacity(64);

    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }

    encoded
}
