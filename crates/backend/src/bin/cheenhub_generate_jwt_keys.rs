#![warn(missing_docs)]
//! Генерирует пару ключей Ed25519 для настройки JWT CheenHub.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;

/// Печатает переменные окружения JWT-ключей в формате, пригодном для shell.
fn main() {
    let signing_key = SigningKey::generate(&mut OsRng);
    let private_key = STANDARD.encode(signing_key.to_bytes());
    let public_key = STANDARD.encode(signing_key.verifying_key().to_bytes());

    println!("JWT_ED25519_PRIVATE_KEY_BASE64={private_key}");
    println!("CHEENHUB_JWT_PUBLIC_KEY_BASE64={public_key}");
}
