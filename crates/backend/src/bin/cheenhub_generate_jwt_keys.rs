#![warn(missing_docs)]
//! Generates an Ed25519 key pair for CheenHub JWT configuration.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

/// Prints shell-compatible JWT key environment variables.
fn main() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let private_key = STANDARD.encode(signing_key.to_bytes());
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());

    println!("JWT_ED25519_PRIVATE_KEY_BASE64={private_key}");
    println!("CHEENHUB_JWT_PUBLIC_KEY_BASE64={public_key}");
}
