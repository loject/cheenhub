//! Desktop OAuth: системный браузер, ограниченное ожидание и отмена попытки.

use super::super::Progress;
use super::super::backend::DesktopOAuthBackend;
use crate::features::auth::api::{self, OAuthCompletion};
use cheenhub_contracts::rest::{
    GoogleDesktopAuthPollResponse, GoogleDesktopAuthRequest, GoogleDesktopAuthStartResponse,
    OAuthFlow, OAuthStartRequest,
};
use dioxus::logger::tracing::{info, warn};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::{Instant, timeout};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(300);

/// Desktop-реализация входа через системный браузер.
pub(in crate::features::auth::desktop_oauth) struct Platform;
impl DesktopOAuthBackend for Platform {
    fn is_supported() -> bool {
        true
    }
    async fn authenticate(
        flow: OAuthFlow,
        on_progress: impl FnMut(Progress),
    ) -> Result<OAuthCompletion, String> {
        timeout(ATTEMPT_TIMEOUT, authenticate(flow, on_progress))
            .await
            .map_err(|_| {
                "Время ожидания истекло. Попробуй войти через Google ещё раз.".to_owned()
            })?
    }
}

async fn authenticate(
    flow: OAuthFlow,
    mut on_progress: impl FnMut(Progress),
) -> Result<OAuthCompletion, String> {
    info!(?flow, "starting desktop Google OAuth");
    let mut request = api::post("/auth/oauth/google/desktop/start");
    if flow == OAuthFlow::Link {
        let token = timeout(REQUEST_TIMEOUT, api::fresh_access_token())
            .await
            .map_err(|_| "Не удалось подтвердить вход. Попробуй ещё раз.".to_owned())??;
        request = request.bearer_auth(token);
    }
    let start: GoogleDesktopAuthStartResponse =
        json(request.json(&OAuthStartRequest { flow })).await?;
    let mut attempt = Attempt(Some(GoogleDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret,
    }));
    let deadline = Instant::now() + Duration::from_secs(start.expires_in_seconds.min(300));
    validate_authorization_url(&start.authorization_url)?;
    tokio::task::spawn_blocking(move || webbrowser::open(&start.authorization_url))
        .await
        .map_err(|_| "Не удалось открыть браузер. Повтори вход.".to_owned())?
        .map_err(|_| {
            "Не удалось открыть браузер. Проверь браузер по умолчанию и повтори вход.".to_owned()
        })?;
    info!(attempt_id = %start.attempt_id, "desktop Google OAuth browser opened");
    on_progress(Progress::Waiting);
    let interval = poll_delay(start.poll_interval_seconds);
    let mut failures = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(expired_message());
        }
        tokio::time::sleep(interval.min(remaining)).await;
        if Instant::now() >= deadline {
            return Err(expired_message());
        }
        let response = json::<GoogleDesktopAuthPollResponse>(
            api::post("/auth/oauth/google/desktop/poll").json(attempt.0.as_ref().unwrap()),
        )
        .await;
        let response = match response {
            Ok(response) => {
                failures = 0;
                response
            }
            Err(error) => {
                failures += 1;
                warn!(attempt_id = %start.attempt_id, failures, "desktop Google OAuth polling failed");
                if failures >= 3 {
                    return Err(error);
                }
                continue;
            }
        };
        match response {
            GoogleDesktopAuthPollResponse::Pending => {}
            GoogleDesktopAuthPollResponse::Cancelled => {
                return Err("Вход отменён. Можно попробовать снова.".to_owned());
            }
            GoogleDesktopAuthPollResponse::Expired => return Err(expired_message()),
            GoogleDesktopAuthPollResponse::Failed { message } => return Err(message),
            GoogleDesktopAuthPollResponse::Ready => {
                on_progress(Progress::Completing);
                let value: serde_json::Value = json(api::post("/auth/oauth/google/complete").json(
                    &serde_json::json!({"handoff_code": attempt.0.as_ref().unwrap().poll_secret}),
                ))
                .await?;
                // Привязка не должна сохранять чужую сессию даже при неожиданном ответе.
                let result = finish(value, flow)?;
                attempt.0 = None;
                info!(attempt_id = %start.attempt_id, "desktop Google OAuth completed");
                return Ok(result);
            }
        }
    }
}

fn finish(value: serde_json::Value, flow: OAuthFlow) -> Result<OAuthCompletion, String> {
    if flow == OAuthFlow::Link {
        if value.get("kind").and_then(serde_json::Value::as_str) == Some("linked") {
            return Ok(OAuthCompletion::Linked);
        }
        return Err(
            "Не удалось подтвердить подключение Google. Обнови список аккаунтов.".to_owned(),
        );
    }
    // Нет await между проверкой результата и сохранением: отменённая задача сюда не попадёт.
    api::parse_oauth_completion(value)
}

async fn json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, String> {
    timeout(REQUEST_TIMEOUT, async {
        let response = request.timeout(REQUEST_TIMEOUT).send().await.map_err(|_| {
            "Не удалось связаться с сервером. Проверь подключение и повтори вход.".to_owned()
        })?;
        if !response.status().is_success() {
            return Err(api::read_error(response).await);
        }
        response
            .json()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера. Повтори вход.".to_owned())
    })
    .await
    .map_err(|_| "Сервер не ответил вовремя. Повтори вход.".to_owned())?
}

fn expired_message() -> String {
    "Время ожидания истекло. Попробуй войти ещё раз.".to_owned()
}
fn poll_delay(seconds: u64) -> Duration {
    Duration::from_secs(seconds.clamp(2, 10))
}
fn validate_authorization_url(value: &str) -> Result<(), String> {
    let valid = url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some("accounts.google.com")
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
    });
    if valid {
        Ok(())
    } else {
        Err("Не удалось получить ссылку Google. Повтори вход.".to_owned())
    }
}

struct Attempt(Option<GoogleDesktopAuthRequest>);
impl Drop for Attempt {
    fn drop(&mut self) {
        let Some(attempt) = self.0.take() else {
            return;
        };
        // Отмена не зависит от уже закрытого компонента; TTL страхует выход из приложения.
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            warn!(attempt_id = %attempt.attempt_id, "desktop OAuth cancellation unavailable during shutdown; attempt expires automatically");
            return;
        };
        runtime.spawn(async move {
            info!(attempt_id = %attempt.attempt_id, "cancelling desktop Google OAuth attempt");
            match api::post("/auth/oauth/google/desktop/cancel")
                .timeout(REQUEST_TIMEOUT)
                .json(&attempt)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {}
                _ => {
                    warn!(attempt_id = %attempt.attempt_id, "desktop Google OAuth cancellation failed; attempt expires automatically")
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{finish, poll_delay, validate_authorization_url};
    use cheenhub_contracts::rest::OAuthFlow;
    use std::time::Duration;
    #[test]
    fn bounds_polling_interval() {
        assert_eq!(poll_delay(0), Duration::from_secs(2));
        assert_eq!(poll_delay(3), Duration::from_secs(3));
        assert_eq!(poll_delay(u64::MAX), Duration::from_secs(10));
    }
    #[test]
    fn opens_only_google_https_authorization_urls() {
        assert!(
            validate_authorization_url(
                "https://accounts.google.com/o/oauth2/v2/auth?state=example"
            )
            .is_ok()
        );
        for invalid in [
            "file:///tmp/auth",
            "http://accounts.google.com/auth",
            "https://evil.test/auth",
            "https://accounts.google.com@evil.test/auth",
        ] {
            assert!(validate_authorization_url(invalid).is_err());
        }
    }
    #[test]
    fn linking_rejects_session_and_registration_payloads() {
        assert!(finish(serde_json::json!({"kind":"linked"}), OAuthFlow::Link).is_ok());
        assert!(finish(serde_json::json!({"auth_response": {}}), OAuthFlow::Link).is_err());
        assert!(
            finish(
                serde_json::json!({"kind":"registration_required"}),
                OAuthFlow::Link
            )
            .is_err()
        );
    }
}
