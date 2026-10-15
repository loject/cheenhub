//! Проверки безопасной генерации и сохранения JWT-ключей.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use ed25519_dalek::SigningKey;

use super::{PRIVATE, PUBLIC, ensure};
use crate::dev_env::env_file::EnvFile;

#[test]
fn generates_matching_pair_only_once() {
    let mut env = EnvFile::new(String::new());

    ensure(&mut env).unwrap();
    let first = env.content().to_owned();
    ensure(&mut env).unwrap();

    assert_eq!(env.content(), first);
    assert!(env.nonempty(PRIVATE).unwrap().is_some());
    assert!(env.nonempty(PUBLIC).unwrap().is_some());
}

#[test]
fn derives_missing_public_from_existing_private() {
    let key = SigningKey::from_bytes(&[7_u8; 32]);
    let private = STANDARD.encode(key.to_bytes());
    let mut env = EnvFile::new(format!("{PRIVATE}={private}\n"));

    ensure(&mut env).unwrap();

    assert_eq!(
        env.nonempty(PRIVATE).unwrap().as_deref(),
        Some(private.as_str())
    );
    assert_eq!(
        env.nonempty(PUBLIC).unwrap().unwrap(),
        STANDARD.encode(key.verifying_key().to_bytes())
    );
}

#[test]
fn refuses_public_key_without_private() {
    let mut env = EnvFile::new(format!("{PUBLIC}=any-value\n"));
    let before = env.content().to_owned();

    assert!(ensure(&mut env).is_err());

    assert_eq!(env.content(), before);
}

#[test]
fn refuses_mismatching_public_key() {
    let key = SigningKey::from_bytes(&[7_u8; 32]);
    let other = SigningKey::from_bytes(&[8_u8; 32]);
    let mut env = EnvFile::new(format!(
        "{PRIVATE}={}\n{PUBLIC}={}\n",
        STANDARD.encode(key.to_bytes()),
        STANDARD.encode(other.verifying_key().to_bytes())
    ));
    let before = env.content().to_owned();

    assert!(ensure(&mut env).is_err());

    assert_eq!(env.content(), before);
}

#[test]
fn refuses_invalid_private_key_instead_of_rotating() {
    let mut env = EnvFile::new(format!("{PRIVATE}=not-base64\n"));

    assert!(ensure(&mut env).is_err());
    assert_eq!(
        env.nonempty(PRIVATE).unwrap().as_deref(),
        Some("not-base64")
    );
}

#[test]
fn refuses_conflicting_key_ids() {
    let mut env = EnvFile::new("JWT_KEY_ID=one\nCHEENHUB_JWT_KEY_ID=two\n".into());
    let before = env.content().to_owned();

    assert!(ensure(&mut env).is_err());

    assert_eq!(env.content(), before);
}
