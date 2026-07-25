//! Криптографическая проверка Google ID Token.

use base64::{
    Engine as _,
    engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD},
};
use chrono::{DateTime, Utc};
use rsa::{
    BigUint, RsaPublicKey,
    pkcs1v15::{Signature, VerifyingKey},
    signature::Verifier,
};
use serde::Deserialize;
use sha2::Sha256;

const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

/// Публичный набор ключей Google в формате JWKS.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GoogleJwks {
    /// Доступные ключи подписи.
    keys: Vec<GoogleJwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleJwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

#[derive(Debug, Deserialize)]
struct GoogleClaims {
    iss: String,
    aud: Audience,
    azp: Option<String>,
    sub: String,
    email: Option<String>,
    email_verified: bool,
    nonce: Option<String>,
    name: Option<String>,
    exp: i64,
    nbf: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

/// Проверенная личность из Google ID Token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedGoogleIdentity {
    /// Стабильный subject Google.
    pub(crate) subject: String,
    /// Подтвержденный email Google.
    pub(crate) email: String,
    /// Отображаемое имя Google.
    pub(crate) display_name: Option<String>,
}

impl GoogleJwks {
    /// Возвращает, содержит ли набор ключ с указанным `kid`.
    pub(crate) fn contains_kid(&self, kid: &str) -> bool {
        self.keys.iter().any(|key| key.kid == kid)
    }

    /// Возвращает количество доступных ключей.
    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }
}

/// Извлекает идентификатор ключа из заголовка токена без доверия к его содержимому.
pub(crate) fn unverified_key_id(token: &str) -> anyhow::Result<String> {
    let header_segment = token
        .split('.')
        .next()
        .ok_or_else(|| anyhow::anyhow!("google id token has no header"))?;
    let header: JwtHeader = decode_json_segment(header_segment)?;
    if header.alg != "RS256" || header.kid.trim().is_empty() {
        anyhow::bail!("google id token header is invalid");
    }

    Ok(header.kid)
}

/// Проверяет подпись RS256 и обязательные claims Google ID Token.
pub(crate) fn verify(
    token: &str,
    jwks: &GoogleJwks,
    expected_audience: &str,
    expected_nonce: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<VerifiedGoogleIdentity> {
    let mut segments = token.split('.');
    let header_segment = segments
        .next()
        .ok_or_else(|| anyhow::anyhow!("google id token has no header"))?;
    let claims_segment = segments
        .next()
        .ok_or_else(|| anyhow::anyhow!("google id token has no claims"))?;
    let signature_segment = segments
        .next()
        .ok_or_else(|| anyhow::anyhow!("google id token has no signature"))?;
    if segments.next().is_some() {
        anyhow::bail!("google id token has unexpected segments");
    }

    let header: JwtHeader = decode_json_segment(header_segment)?;
    if header.alg != "RS256" {
        anyhow::bail!("google id token uses unsupported algorithm");
    }
    let jwk = jwks
        .keys
        .iter()
        .find(|key| key.kid == header.kid)
        .ok_or_else(|| anyhow::anyhow!("google id token signing key is unknown"))?;
    if jwk.kty != "RSA" || jwk.alg.as_deref().is_some_and(|alg| alg != "RS256") {
        anyhow::bail!("google jwk is incompatible with RS256");
    }

    let modulus = decode_base64url(&jwk.n)?;
    let exponent = decode_base64url(&jwk.e)?;
    let public_key = RsaPublicKey::new(
        BigUint::from_bytes_be(&modulus),
        BigUint::from_bytes_be(&exponent),
    )?;
    let signature = Signature::try_from(decode_base64url(signature_segment)?.as_slice())?;
    let signing_input = format!("{header_segment}.{claims_segment}");
    VerifyingKey::<Sha256>::new(public_key)
        .verify(signing_input.as_bytes(), &signature)
        .map_err(|_| anyhow::anyhow!("google id token signature is invalid"))?;

    let claims: GoogleClaims = decode_json_segment(claims_segment)?;
    validate_claims(claims, expected_audience, expected_nonce, now)
}

fn validate_claims(
    claims: GoogleClaims,
    expected_audience: &str,
    expected_nonce: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<VerifiedGoogleIdentity> {
    if !GOOGLE_ISSUERS.contains(&claims.iss.as_str()) {
        anyhow::bail!("google id token issuer is invalid");
    }
    let audience_matches = match &claims.aud {
        Audience::One(audience) => audience == expected_audience,
        Audience::Many(audiences) => audiences
            .iter()
            .any(|audience| audience == expected_audience),
    };
    if !audience_matches {
        anyhow::bail!("google id token audience is invalid");
    }
    if matches!(&claims.aud, Audience::Many(audiences) if audiences.len() > 1)
        && claims.azp.as_deref() != Some(expected_audience)
    {
        anyhow::bail!("google id token authorized party is invalid");
    }
    if claims.exp <= now.timestamp() {
        anyhow::bail!("google id token has expired");
    }
    if claims
        .nbf
        .is_some_and(|not_before| not_before > now.timestamp())
    {
        anyhow::bail!("google id token is not active yet");
    }
    if claims.nonce.as_deref() != Some(expected_nonce) {
        anyhow::bail!("google id token nonce is invalid");
    }
    if !claims.email_verified {
        anyhow::bail!("google id token email is not verified");
    }
    if claims.sub.trim().is_empty() {
        anyhow::bail!("google id token subject is empty");
    }
    let email = claims
        .email
        .filter(|email| !email.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("google id token has no email"))?;

    Ok(VerifiedGoogleIdentity {
        subject: claims.sub,
        email,
        display_name: claims.name,
    })
}

fn decode_json_segment<T: for<'de> Deserialize<'de>>(segment: &str) -> anyhow::Result<T> {
    Ok(serde_json::from_slice(&decode_base64url(segment)?)?)
}

fn decode_base64url(value: &str) -> anyhow::Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(value)
        .or_else(|_| URL_SAFE.decode(value))
        .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
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
}
