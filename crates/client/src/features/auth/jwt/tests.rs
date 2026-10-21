use super::*;
use ed25519_dalek::{Signer, SigningKey};

fn key_and_token() -> (cheenhub_contracts::rest::AuthVerificationKey, String) {
    let key = SigningKey::from_bytes(&[42; 32]);
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"EdDSA","kid":"runtime-key"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&serde_json::json!({
            "sub": "user", "nickname": "person", "email": "person@example.com",
            "iat": now_seconds(), "exp": now_seconds() + 600,
            "session_id": "session", "kid": "runtime-key"
        }))
        .unwrap(),
    );
    let input = format!("{header}.{payload}");
    let signature = URL_SAFE_NO_PAD.encode(key.sign(input.as_bytes()).to_bytes());
    (
        cheenhub_contracts::rest::AuthVerificationKey {
            key_id: "runtime-key".to_owned(),
            public_key_base64: STANDARD.encode(key.verifying_key().to_bytes()),
        },
        format!("{input}.{signature}"),
    )
}

#[test]
fn verifies_with_the_runtime_key_instead_of_build_settings() {
    let (key, token) = key_and_token();

    let claims = verify(&token, &key).unwrap();

    assert_eq!(claims.kid, "runtime-key");
}

#[test]
fn rejects_a_signature_from_another_server() {
    let (mut key, token) = key_and_token();
    key.public_key_base64 =
        STANDARD.encode(SigningKey::from_bytes(&[43; 32]).verifying_key().to_bytes());

    assert_eq!(verify(&token, &key), Err(JwtVerifyError::InvalidToken));
}

#[test]
fn rejects_a_different_runtime_key_id() {
    let (mut key, token) = key_and_token();
    key.key_id = "other-key".to_owned();

    assert_eq!(verify(&token, &key), Err(JwtVerifyError::InvalidToken));
}

#[test]
fn rejects_malformed_public_key_material() {
    let (mut key, token) = key_and_token();
    key.public_key_base64 = "invalid".to_owned();

    assert_eq!(
        verify(&token, &key),
        Err(JwtVerifyError::VerificationKeyUnavailable)
    );
}
