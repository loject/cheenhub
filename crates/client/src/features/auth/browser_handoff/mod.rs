//! Временная browser-попытка входа в desktop и её безопасное подтверждение.

use cheenhub_contracts::rest::BrowserDesktopAuthDetailsResponse;
use uuid::Uuid;

use crate::Route;
use crate::features::auth::api;

#[cfg(feature = "web")]
use dioxus_sdk_storage::{SessionStorage, StorageBacking};

#[cfg(feature = "web")]
const PENDING_ATTEMPT_KEY: &str = "cheenhub.auth.desktop_browser_pending";

/// Запоминает только публичный ID попытки в пределах браузерной вкладки.
pub(crate) fn remember(attempt_id: Uuid) {
    #[cfg(feature = "web")]
    SessionStorage::set(
        PENDING_ATTEMPT_KEY.to_owned(),
        &Some(attempt_id.to_string()),
    );
    #[cfg(not(feature = "web"))]
    let _ = attempt_id;
}

/// Читает ожидающую попытку; неизвестные или повреждённые значения игнорируются.
pub(crate) fn pending() -> Option<Uuid> {
    #[cfg(feature = "web")]
    {
        SessionStorage::get::<Option<String>>(&PENDING_ATTEMPT_KEY.to_owned())
            .flatten()
            .and_then(|id| Uuid::parse_str(&id).ok())
    }
    #[cfg(not(feature = "web"))]
    {
        None
    }
}

/// Удаляет запомненную попытку после подтверждения или отказа.
pub(crate) fn clear() {
    #[cfg(feature = "web")]
    SessionStorage::set(PENDING_ATTEMPT_KEY.to_owned(), &Option::<String>::None);
}

/// После авторизации возвращает на подтверждение, если оно открыто в этой вкладке.
pub(crate) fn after_login_route() -> Route {
    match pending() {
        Some(attempt_id) => Route::BrowserDesktopConfirm {
            attempt: Some(attempt_id.to_string()),
        },
        None => Route::AppHome {},
    }
}

/// Код используется только для визуальной сверки, не как секрет авторизации.
#[cfg(any(
    test,
    all(feature = "desktop", any(target_os = "linux", target_os = "windows"))
))]
pub(crate) fn verification_code_for_attempt(id: Uuid) -> String {
    let value = id.as_simple().to_string();
    format!("{}-{}", &value[..4], &value[4..8]).to_uppercase()
}

/// Скрывает почтовый адрес, оставляя первые буквы и домен верхнего уровня.
pub(crate) fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "***".to_owned();
    };
    let parts: Vec<_> = domain.split('.').collect();
    if local.is_empty() || parts.len() < 2 || parts.iter().any(|part| part.is_empty()) {
        return "***".to_owned();
    }
    let first = |part: &str| format!("{}***", part.chars().next().unwrap_or('*'));
    let masked_domain = parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            if index + 1 == parts.len() {
                (*part).to_owned()
            } else {
                first(part)
            }
        })
        .collect::<Vec<_>>()
        .join(".");
    format!("{}@{}", first(local), masked_domain)
}

/// Проверяет существование попытки и получает лишь публичный сверочный код.
pub(crate) async fn details(attempt_id: Uuid) -> Result<BrowserDesktopAuthDetailsResponse, String> {
    let response = api::get(&format!("/auth/browser/desktop/{attempt_id}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;
    if !response.status().is_success() {
        return Err(api::read_error(response).await);
    }
    response
        .json()
        .await
        .map_err(|_| "Не удалось прочитать подтверждение входа.".to_owned())
}

/// Явное разрешение сессии desktop от имени авторизованного пользователя браузера.
pub(crate) async fn approve(attempt_id: Uuid) -> Result<(), String> {
    let access_token = api::fresh_access_token().await?;
    let response = api::post(&format!("/auth/browser/desktop/{attempt_id}/approve"))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(api::read_error(response).await)
    }
}

#[cfg(test)]
mod tests;
