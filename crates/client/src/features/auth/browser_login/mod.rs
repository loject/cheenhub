//! Одноразовый вход desktop-клиента через текущую веб-сессию CheenHub.

use cheenhub_contracts::rest::{
    AuthResponse, AuthUser, BrowserDesktopAuthPollResponse, BrowserDesktopAuthRequest,
    BrowserDesktopAuthStartResponse,
};
use dioxus::logger::tracing::{info, warn};
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::{Instant, timeout};
use url::Url;
use uuid::Uuid;

use crate::features::auth::{api, browser_handoff};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(300);

/// Стадия авторизации в системном браузере.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Progress {
    /// Создание попытки и открытие ссылки.
    Opening,
    /// Ожидание пользователя, показывая короткий код сверки.
    Waiting(String),
    /// Получение отдельной desktop-сессии.
    Completing,
}

/// Открывает системный браузер, ожидает явного разрешения и сохраняет новую сессию.
pub(crate) async fn authenticate(on_progress: impl FnMut(Progress)) -> Result<AuthUser, String> {
    timeout(ATTEMPT_TIMEOUT, run(on_progress))
        .await
        .map_err(|_| expired_message())?
}

async fn run(mut on_progress: impl FnMut(Progress)) -> Result<AuthUser, String> {
    on_progress(Progress::Opening);
    let start: BrowserDesktopAuthStartResponse =
        json(api::post("/auth/browser/desktop/start")).await?;
    let mut attempt = Attempt(Some(BrowserDesktopAuthRequest {
        attempt_id: start.attempt_id,
        poll_secret: start.poll_secret,
    }));
    let expected_url = crate::config::public_url("/auth/desktop")?;
    validate_authorization_url(&start.authorization_url, start.attempt_id, &expected_url)?;
    if start.verification_code != browser_handoff::verification_code_for_attempt(start.attempt_id) {
        return Err("Сервер вернул некорректный проверочный код.".to_owned());
    }
    let deadline = Instant::now() + Duration::from_secs(start.expires_in_seconds.min(300));
    let url = start.authorization_url;
    tokio::task::spawn_blocking(move || webbrowser::open(&url))
        .await
        .map_err(|_| "Не удалось открыть браузер. Повтори вход.".to_owned())?
        .map_err(|_| "Не удалось открыть браузер. Проверь браузер по умолчанию.".to_owned())?;
    info!(attempt_id = %start.attempt_id, "desktop browser sign-in opened");
    on_progress(Progress::Waiting(start.verification_code));
    let delay = poll_delay(start.poll_interval_seconds);
    let mut failures = 0;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(expired_message());
        }
        tokio::time::sleep(delay.min(remaining)).await;
        if Instant::now() >= deadline {
            return Err(expired_message());
        }
        let result = json::<BrowserDesktopAuthPollResponse>(
            api::post("/auth/browser/desktop/poll")
                .json(attempt.0.as_ref().expect("attempt active")),
        )
        .await;
        let result = match result {
            Ok(value) => {
                failures = 0;
                value
            }
            Err(error) => {
                failures += 1;
                warn!(attempt_id = %start.attempt_id, failures, "desktop browser sign-in polling failed");
                if failures >= 3 {
                    return Err(error);
                }
                continue;
            }
        };
        match result {
            BrowserDesktopAuthPollResponse::Pending => {}
            BrowserDesktopAuthPollResponse::Ready => {
                on_progress(Progress::Completing);
                let response: AuthResponse = json(
                    api::post("/auth/browser/desktop/complete")
                        .json(attempt.0.as_ref().expect("attempt active")),
                )
                .await?;
                let user = api::save_browser_auth_response(response).await?;
                attempt.0 = None;
                info!(attempt_id = %start.attempt_id, "desktop browser sign-in completed");
                return Ok(user);
            }
        }
    }
}

async fn json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, String> {
    timeout(REQUEST_TIMEOUT, async {
        let response = request
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|_| "Не удалось связаться с сервером. Проверь подключение.".to_owned())?;
        if !response.status().is_success() {
            return Err(api::read_error(response).await);
        }
        response
            .json()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())
    })
    .await
    .map_err(|_| "Сервер не ответил вовремя.".to_owned())?
}

fn expired_message() -> String {
    "Время ожидания истекло. Попробуй войти снова.".to_owned()
}

fn poll_delay(seconds: u64) -> Duration {
    Duration::from_secs(seconds.clamp(2, 10))
}

fn validate_authorization_url(
    value: &str,
    attempt_id: Uuid,
    expected_origin: &Url,
) -> Result<(), String> {
    let actual = Url::parse(value).map_err(|_| "Некорректная ссылка входа.".to_owned())?;
    let development = expected_origin.scheme() == "http"
        && matches!(
            expected_origin.host_str(),
            Some("localhost" | "127.0.0.1" | "[::1]")
        );
    let same_origin = actual.scheme() == expected_origin.scheme()
        && actual.host_str() == expected_origin.host_str()
        && (development
            || actual.port_or_known_default() == expected_origin.port_or_known_default());
    let query: Vec<_> = actual.query_pairs().collect();
    if (expected_origin.scheme() == "https" || development)
        && same_origin
        && actual.path() == "/auth/desktop"
        && actual.username().is_empty()
        && actual.password().is_none()
        && actual.fragment().is_none()
        && query.len() == 1
        && query[0].0 == "attempt"
        && Uuid::parse_str(query[0].1.as_ref()).ok() == Some(attempt_id)
    {
        Ok(())
    } else {
        Err("Сервер вернул небезопасную ссылку подтверждения.".to_owned())
    }
}

/// При закрытии окна или отмене задачи серверная попытка отзывается.
struct Attempt(Option<BrowserDesktopAuthRequest>);
impl Drop for Attempt {
    fn drop(&mut self) {
        let Some(attempt) = self.0.take() else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            warn!(attempt_id = %attempt.attempt_id, "desktop browser attempt will expire automatically");
            return;
        };
        runtime.spawn(async move {
            let id = attempt.attempt_id;
            let response = api::post("/auth/browser/desktop/cancel")
                .timeout(REQUEST_TIMEOUT)
                .json(&attempt)
                .send()
                .await;
            if !matches!(response, Ok(result) if result.status().is_success()) {
                warn!(attempt_id = %id, "desktop browser cancellation failed; attempt will expire");
            }
        });
    }
}

#[cfg(test)]
mod tests;
