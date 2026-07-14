//! Адаптер отправки data-only уведомлений через FCM HTTP v1.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use base64::Engine;
use chrono::Utc;
use reqwest::StatusCode;
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::Mutex;

use crate::features::push_notifications::domain::DirectMessagePush;

const FCM_SCOPE: &str = "https://www.googleapis.com/auth/firebase.messaging";

/// Результат отклонённой FCM-доставки.
#[derive(Debug)]
pub(crate) enum FcmSendError {
    /// Повторная попытка может завершиться успешно.
    Retry(anyhow::Error),
    /// Токен или запрос окончательно непригоден для доставки.
    Permanent(anyhow::Error),
}

/// Клиент FCM с кэшированием короткоживущего OAuth access token.
#[derive(Clone)]
pub(crate) struct FcmClient {
    http: reqwest::Client,
    credentials: Arc<ServiceAccount>,
    access_token: Arc<Mutex<Option<CachedAccessToken>>>,
}

impl FcmClient {
    /// Загружает service account из внешнего JSON-файла.
    pub(crate) fn from_service_account_file(path: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path).with_context(|| {
            format!("failed to read FCM service account file {}", path.display())
        })?;
        let credentials: ServiceAccount =
            serde_json::from_slice(&bytes).context("failed to parse FCM service account JSON")?;
        if credentials.project_id.trim().is_empty()
            || credentials.client_email.trim().is_empty()
            || credentials.private_key.trim().is_empty()
            || credentials.token_uri.trim().is_empty()
        {
            return Err(anyhow!("FCM service account JSON misses required fields"));
        }
        Ok(Self {
            http: reqwest::Client::new(),
            credentials: Arc::new(credentials),
            access_token: Arc::new(Mutex::new(None)),
        })
    }

    /// Отправляет одно data-only уведомление с Android high priority.
    pub(crate) async fn send(
        &self,
        token: &str,
        payload: &DirectMessagePush,
    ) -> Result<(), FcmSendError> {
        let access_token = self.access_token().await.map_err(FcmSendError::Retry)?;
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.credentials.project_id
        );
        let response = self
            .http
            .post(url)
            .bearer_auth(access_token)
            .json(&FcmRequest {
                message: FcmMessage {
                    token,
                    data: payload,
                    android: AndroidConfig { priority: "high" },
                },
            })
            .send()
            .await
            .map_err(|error| FcmSendError::Retry(error.into()))?;
        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let response_body = response.text().await.unwrap_or_default();
        let provider_status = serde_json::from_str::<FcmErrorResponse>(&response_body)
            .ok()
            .and_then(|body| body.error.status);
        let error = anyhow!(
            "FCM rejected delivery with HTTP {} and status {}",
            status,
            provider_status.as_deref().unwrap_or("unknown")
        );
        if status == StatusCode::UNAUTHORIZED {
            *self.access_token.lock().await = None;
            return Err(FcmSendError::Retry(error));
        }
        if status == StatusCode::NOT_FOUND
            || provider_status.as_deref() == Some("UNREGISTERED")
            || provider_status.as_deref() == Some("INVALID_ARGUMENT")
        {
            Err(FcmSendError::Permanent(error))
        } else {
            Err(FcmSendError::Retry(error))
        }
    }

    async fn access_token(&self) -> anyhow::Result<String> {
        let mut cached = self.access_token.lock().await;
        let now = Utc::now().timestamp();
        if let Some(token) = cached.as_ref()
            && token.expires_at > now + 60
        {
            return Ok(token.value.clone());
        }

        let assertion = self.signed_assertion(now)?;
        let response = self
            .http
            .post(&self.credentials.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", assertion.as_str()),
            ])
            .send()
            .await
            .context("failed to request FCM OAuth access token")?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!(
                "FCM OAuth token endpoint rejected request with HTTP {status}"
            ));
        }
        let token: OAuthTokenResponse = response
            .json()
            .await
            .context("failed to decode FCM OAuth access token")?;
        let expires_at = now + token.expires_in.max(60);
        *cached = Some(CachedAccessToken {
            value: token.access_token.clone(),
            expires_at,
        });
        Ok(token.access_token)
    }

    fn signed_assertion(&self, issued_at: i64) -> anyhow::Result<String> {
        let encoder = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let header = encoder.encode(serde_json::to_vec(&JwtHeader {
            alg: "RS256",
            typ: "JWT",
        })?);
        let claims = encoder.encode(serde_json::to_vec(&JwtClaims {
            iss: &self.credentials.client_email,
            scope: FCM_SCOPE,
            aud: &self.credentials.token_uri,
            iat: issued_at,
            exp: issued_at + 3600,
        })?);
        let signing_input = format!("{header}.{claims}");
        let private_key = RsaPrivateKey::from_pkcs8_pem(&self.credentials.private_key)
            .context("failed to decode FCM service account private key")?;
        let signature = SigningKey::<Sha256>::new(private_key).sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            encoder.encode(signature.to_vec())
        ))
    }
}

#[derive(Debug, Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
    token_uri: String,
}

#[derive(Debug, Clone)]
struct CachedAccessToken {
    value: String,
    expires_at: i64,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: i64,
}

#[derive(Debug, Serialize)]
struct JwtHeader<'a> {
    alg: &'a str,
    typ: &'a str,
}

#[derive(Debug, Serialize)]
struct JwtClaims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

#[derive(Debug, Serialize)]
struct FcmRequest<'a> {
    message: FcmMessage<'a>,
}

#[derive(Debug, Serialize)]
struct FcmMessage<'a> {
    token: &'a str,
    data: &'a DirectMessagePush,
    android: AndroidConfig<'a>,
}

#[derive(Debug, Serialize)]
struct AndroidConfig<'a> {
    priority: &'a str,
}

#[derive(Debug, Deserialize)]
struct FcmErrorResponse {
    error: FcmErrorBody,
}

#[derive(Debug, Deserialize)]
struct FcmErrorBody {
    status: Option<String>,
}
