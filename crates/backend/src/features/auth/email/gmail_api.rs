//! HTTPS-транспорт аутентификационных писем через Gmail API.

use std::time::Duration;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};

use super::message::{password_changed_message, password_reset_message};
use super::{AuthMailer, EmailError, PasswordChangedEmail, PasswordResetEmail};

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SEND_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me/messages/send";

#[derive(Clone)]
struct GmailApiConfig {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    from: String,
}

/// Отправитель аутентификационных писем через Gmail API по HTTPS.
pub(crate) struct GmailApiAuthMailer {
    client: reqwest::Client,
    config: Option<GmailApiConfig>,
    missing: Vec<&'static str>,
    timeout: Duration,
}

impl GmailApiAuthMailer {
    /// Создает HTTPS-отправитель из OAuth-конфигурации Gmail API.
    pub(crate) fn new(
        client_id: Option<String>,
        client_secret: Option<String>,
        refresh_token: Option<String>,
        from: Option<String>,
        timeout: Duration,
    ) -> anyhow::Result<Self> {
        let missing = missing_gmail_api_config(&client_id, &client_secret, &refresh_token, &from);
        if !missing.is_empty() {
            tracing::warn!(
                email_transport = "gmail_api",
                missing_fields = ?missing,
                "auth email transport is not fully configured"
            );
        }
        let config = if missing.is_empty() {
            Some(GmailApiConfig {
                client_id: client_id.expect("client ID was checked"),
                client_secret: client_secret.expect("client secret was checked"),
                refresh_token: refresh_token.expect("refresh token was checked"),
                from: from.expect("from address was checked"),
            })
        } else {
            None
        };
        let client = reqwest::Client::builder().timeout(timeout).build()?;

        Ok(Self {
            client,
            config,
            missing,
            timeout,
        })
    }

    async fn send(&self, message: lettre::Message, kind: &'static str) -> Result<(), EmailError> {
        let config = self.configured()?;
        tokio::time::timeout(self.timeout, self.send_inner(config, message))
            .await
            .map_err(|_| {
                EmailError::Internal(anyhow::anyhow!(
                    "Gmail API email send timed out after {} seconds",
                    self.timeout.as_secs()
                ))
            })??;
        tracing::info!(
            email_transport = "gmail_api",
            email_kind = kind,
            "auth email sent"
        );
        Ok(())
    }

    async fn send_inner(
        &self,
        config: &GmailApiConfig,
        message: lettre::Message,
    ) -> Result<(), EmailError> {
        let access_token = self.fetch_access_token(config).await?;
        let response = self
            .client
            .post(SEND_URL)
            .bearer_auth(access_token)
            .json(&GmailMessage {
                raw: encode_raw_message(&message),
            })
            .send()
            .await
            .map_err(|error| EmailError::Internal(error.into()))?;
        if !response.status().is_success() {
            return Err(EmailError::Internal(anyhow::anyhow!(
                "Gmail API message send failed with HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    async fn fetch_access_token(&self, config: &GmailApiConfig) -> Result<String, EmailError> {
        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &config.client_id)
            .append_pair("client_secret", &config.client_secret)
            .append_pair("refresh_token", &config.refresh_token)
            .append_pair("grant_type", "refresh_token")
            .finish();
        let response = self
            .client
            .post(TOKEN_URL)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(form)
            .send()
            .await
            .map_err(|error| EmailError::Internal(error.into()))?;
        if !response.status().is_success() {
            return Err(EmailError::Internal(anyhow::anyhow!(
                "Gmail OAuth token refresh failed with HTTP {}",
                response.status()
            )));
        }

        response
            .json::<TokenResponse>()
            .await
            .map(|response| response.access_token)
            .map_err(|error| EmailError::Internal(error.into()))
    }

    fn configured(&self) -> Result<&GmailApiConfig, EmailError> {
        self.config
            .as_ref()
            .ok_or_else(|| EmailError::Misconfigured {
                missing: self.missing.clone(),
            })
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Serialize)]
struct GmailMessage {
    raw: String,
}

fn encode_raw_message(message: &lettre::Message) -> String {
    URL_SAFE_NO_PAD.encode(message.formatted())
}

fn missing_gmail_api_config(
    client_id: &Option<String>,
    client_secret: &Option<String>,
    refresh_token: &Option<String>,
    from: &Option<String>,
) -> Vec<&'static str> {
    let values = [
        ("gmail_client_id", client_id),
        ("gmail_client_secret", client_secret),
        ("gmail_refresh_token", refresh_token),
        ("gmail_from_email", from),
    ];
    values
        .into_iter()
        .filter_map(|(key, value)| value.as_deref().is_none_or(str::is_empty).then_some(key))
        .collect()
}

#[async_trait]
impl AuthMailer for GmailApiAuthMailer {
    async fn send_password_reset(&self, email: PasswordResetEmail) -> Result<(), EmailError> {
        let config = self.configured()?;
        let message = password_reset_message(&config.from, &email.to, &email.reset_url)?;
        self.send(message, "password_reset").await
    }

    async fn send_password_changed(&self, email: PasswordChangedEmail) -> Result<(), EmailError> {
        let config = self.configured()?;
        let message = password_changed_message(&config.from, &email.to)?;
        self.send(message, "password_changed").await
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

    use super::{GmailApiAuthMailer, encode_raw_message};
    use crate::features::auth::email::message::password_reset_message;

    #[test]
    fn encodes_rfc_2822_message_as_unpadded_base64url() {
        let message = password_reset_message(
            "sender@example.com",
            "recipient@example.com",
            "https://cheenhub.test/reset?token=test",
        )
        .expect("письмо должно собираться");
        let encoded = encode_raw_message(&message);
        assert!(!encoded.contains(['+', '/', '=']));

        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .expect("raw должен декодироваться как base64url");
        let decoded = String::from_utf8(decoded).expect("сообщение должно быть UTF-8");
        assert!(decoded.contains("Subject: CheenHub password reset"));
        assert!(decoded.contains("From: sender@example.com"));
        assert!(decoded.contains("To: recipient@example.com"));
    }

    #[test]
    fn reports_missing_gmail_api_settings() {
        let mailer = GmailApiAuthMailer::new(None, None, None, None, Duration::from_secs(10))
            .expect("неполная конфигурация должна сохраняться как отключенный mailer");
        assert_eq!(
            mailer.missing,
            [
                "gmail_client_id",
                "gmail_client_secret",
                "gmail_refresh_token",
                "gmail_from_email"
            ]
        );
    }
}
