//! Тесты проверки Google ID token.

use super::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rsa::{
    RsaPrivateKey,
    pkcs1v15::SigningKey,
    signature::{SignatureEncoding, Signer},
    traits::PublicKeyParts,
};
use serde_json::json;

fn signed_token(overrides: serde_json::Value) -> (String, GoogleJwks) {
    let private_key =
        RsaPrivateKey::new(&mut rand_core::OsRng, 2048).expect("test rsa key should generate");
    let public_key = private_key.to_public_key();
    let header = json!({"alg": "RS256", "kid": "test-key", "typ": "JWT"});
    let mut claims = json!({
        "iss": "https://accounts.google.com",
        "aud": "test-client",
        "sub": "google-subject",
        "email": "person@example.com",
        "email_verified": true,
        "nonce": "test-nonce",
        "name": "Test Person",
        "exp": 2_000_000_000_i64
    });
    for (key, value) in overrides
        .as_object()
        .expect("overrides should be an object")
    {
        claims[key] = value.clone();
    }
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).expect("header"));
    let claims = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).expect("claims"));
    let signing_input = format!("{header}.{claims}");
    let signature = SigningKey::<Sha256>::new(private_key).sign(signing_input.as_bytes());
    let token = format!(
        "{signing_input}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    );
    let jwks = GoogleJwks {
        keys: vec![GoogleJwk {
            kid: "test-key".to_owned(),
            kty: "RSA".to_owned(),
            alg: Some("RS256".to_owned()),
            n: URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be()),
            e: URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be()),
        }],
    };
    (token, jwks)
}

#[test]
fn verifies_valid_google_id_token() {
    let (token, jwks) = signed_token(json!({}));

    let identity = verify(
        &token,
        &jwks,
        "test-client",
        "test-nonce",
        DateTime::from_timestamp(1_900_000_000, 0).expect("timestamp"),
    )
    .expect("token should verify");

    assert_eq!(identity.subject, "google-subject");
    assert_eq!(identity.email, "person@example.com");
}

#[test]
fn rejects_wrong_audience_nonce_and_expired_token() {
    for overrides in [
        json!({"aud": "other-client"}),
        json!({"nonce": "other-nonce"}),
        json!({"exp": 1_800_000_000_i64}),
        json!({"email_verified": false}),
    ] {
        let (token, jwks) = signed_token(overrides);
        assert!(
            verify(
                &token,
                &jwks,
                "test-client",
                "test-nonce",
                DateTime::from_timestamp(1_900_000_000, 0).expect("timestamp"),
            )
            .is_err()
        );
    }
}

#[test]
fn rejects_tampered_signature() {
    let (mut token, jwks) = signed_token(json!({}));
    token.push('x');

    assert!(
        verify(
            &token,
            &jwks,
            "test-client",
            "test-nonce",
            DateTime::from_timestamp(1_900_000_000, 0).expect("timestamp"),
        )
        .is_err()
    );
}
