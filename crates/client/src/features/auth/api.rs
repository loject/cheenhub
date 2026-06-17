//! Клиент API аутентификации.

use cheenhub_contracts::rest::{
    ApiError, AuthResponse, AuthUser, LoginRequest, LogoutRequest, OAuthFlow,
    OAuthRegistrationRequest, OAuthStartRequest, PasswordResetConfirmRequest, PasswordResetRequest,
    RefreshRequest, RegisterRequest,
};
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::features::auth::{jwt, storage};

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:3000/api";
const NETWORK_ERROR_MESSAGE: &str = "Не удалось связаться с сервером.";

pub(crate) use super::profile_api::{
    change_current_user_password, update_current_user, update_current_user_avatar,
};

/// Регистрирует новую учетную запись и сохраняет возвращенные токены.
pub(crate) async fn register(request: RegisterRequest) -> Result<AuthUser, String> {
    let response = post_json("/auth/register", &request).await?;
    save_response(response)
}

/// Входит по email и паролю и сохраняет возвращенные токены.
pub(crate) async fn login(request: LoginRequest) -> Result<AuthUser, String> {
    let response = post_json("/auth/login", &request).await?;
    save_response(response)
}

/// Запрашивает письмо для сброса пароля для учетной записи.
pub(crate) async fn request_password_reset(request: PasswordResetRequest) -> Result<(), String> {
    post_empty("/auth/password-reset/request", &request).await
}

/// Подтверждает токен сброса пароля и сохраняет новый пароль.
pub(crate) async fn confirm_password_reset(
    request: PasswordResetConfirmRequest,
) -> Result<(), String> {
    post_empty("/auth/password-reset/confirm", &request).await
}

/// Запускает вход через Google OAuth и возвращает URL авторизации провайдера.
pub(crate) async fn start_google_oauth(redirect_uri: String) -> Result<String, String> {
    let _ = redirect_uri;
    start_oauth("/auth/oauth/google/start", OAuthFlow::Login, None).await
}

/// Запускает привязку аккаунта Google и возвращает URL авторизации провайдера.
pub(crate) async fn start_google_account_link(redirect_uri: String) -> Result<String, String> {
    let _ = redirect_uri;
    let access_token = fresh_access_token().await?;
    start_oauth(
        "/auth/oauth/google/start",
        OAuthFlow::Link,
        Some(access_token),
    )
    .await
}

/// Завершает вход через OAuth с кодом handoff от бэкенда.
pub(crate) async fn complete_google_oauth(
    handoff_code: String,
    nickname: Option<String>,
) -> Result<OAuthCompletion, String> {
    if let Some(nickname) = nickname {
        let response = post_json(
            "/auth/oauth/google/register",
            &OAuthRegistrationRequest {
                registration_token: handoff_code,
                nickname,
                accepts_policies: true,
            },
        )
        .await?;
        return save_response(response).map(OAuthCompletion::Authenticated);
    }

    complete_oauth("/auth/oauth/google/complete", handoff_code).await
}

/// Завершает привязку аккаунта через OAuth с кодом handoff от бэкенда.
pub(crate) async fn complete_google_account_link(handoff_code: String) -> Result<(), String> {
    match complete_oauth("/auth/oauth/google/complete", handoff_code).await? {
        OAuthCompletion::Authenticated(_) | OAuthCompletion::Linked => Ok(()),
        OAuthCompletion::RegistrationRequired(_) => {
            Err("Этот Google аккаунт нужно сначала зарегистрировать.".to_owned())
        }
    }
}

/// Загружает привязанные внешние аккаунты текущего пользователя.
pub(crate) async fn linked_accounts() -> Result<Vec<LinkedAccount>, String> {
    let access_token = fresh_access_token().await?;
    let response = Request::get(&url("/auth/linked-accounts"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        return parse_linked_accounts(response).await;
    }

    Err(read_error(response).await)
}

/// Отвязывает внешний аккаунт от текущего пользователя.
pub(crate) async fn unlink_account(provider: &str) -> Result<(), String> {
    if provider != "google" {
        return Err("Этот провайдер пока нельзя отключить.".to_owned());
    }
    let access_token = fresh_access_token().await?;
    let response = Request::post(&url("/auth/linked-accounts/google/unlink"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        Ok(())
    } else {
        Err(read_error(response).await)
    }
}

/// Возвращает, доступна ли локально пара токенов.
pub(crate) fn has_tokens() -> bool {
    storage::load().is_some()
}

/// Возвращает, вызвана ли ошибка API неудачным сетевым запросом.
pub(crate) fn is_network_error(error: &str) -> bool {
    error == NETWORK_ERROR_MESSAGE
}

/// Загружает текущего аутентифицированного пользователя, обновляя access token при необходимости.
pub(crate) async fn current_user() -> Result<AuthUser, String> {
    let access_token = fresh_access_token().await?;
    let response = Request::get(&url("/auth/me"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())?;

    if response.status() == 401 {
        let access_token = refresh_access_token().await?;
        return Request::get(&url("/auth/me"))
            .header("Authorization", &format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())?
            .json::<AuthUser>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    if response.ok() {
        return response
            .json::<AuthUser>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned());
    }

    Err(read_error(response).await)
}

/// Делает текущую refresh-сессию недействительной и очищает сохраненные токены после попытки.
pub(crate) async fn logout() -> Result<(), String> {
    let Some(tokens) = storage::load() else {
        storage::clear();
        return Ok(());
    };

    let request = match Request::post(&url("/auth/logout")).json(&LogoutRequest {
        refresh_token: tokens.refresh_token,
    }) {
        Ok(request) => request,
        Err(_) => {
            storage::clear();
            return Err("Не удалось подготовить запрос.".to_owned());
        }
    };

    let response = request.send().await;
    storage::clear();
    let response = response.map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())?;

    if response.ok() {
        Ok(())
    } else {
        Err(read_error(response).await)
    }
}

pub(crate) async fn fresh_access_token() -> Result<String, String> {
    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };

    if jwt::is_fresh(&tokens.access_token) {
        return Ok(tokens.access_token);
    }

    refresh_access_token().await
}

pub(crate) async fn refresh_access_token() -> Result<String, String> {
    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };
    let response: AuthResponse = match post_json(
        "/auth/refresh",
        &RefreshRequest {
            refresh_token: tokens.refresh_token.clone(),
        },
    )
    .await
    {
        Ok(response) => response,
        Err(error) => {
            if let Some(access_token) = changed_access_token(&tokens) {
                return Ok(access_token);
            }
            if !is_network_error(&error) {
                storage::clear();
            }
            return Err(error);
        }
    };

    jwt::verify(&response.access_token)?;
    storage::save(&response.access_token, &response.refresh_token);
    Ok(response.access_token)
}

/// Результат завершения OAuth, возвращаемый auth API.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum OAuthCompletion {
    /// OAuth completed with a full auth session.
    Authenticated(AuthUser),
    /// OAuth completed, but the user must choose a CheenHub nickname.
    RegistrationRequired(OAuthRegistrationRequired),
    /// OAuth account linking completed.
    Linked,
}

/// Дополнительные данные, необходимые для завершения регистрации OAuth.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct OAuthRegistrationRequired {
    /// Email address returned by the OAuth provider when available.
    pub(crate) email: Option<String>,
    /// Suggested display name returned by the OAuth provider when available.
    pub(crate) suggested_nickname: Option<String>,
}

/// Привязанный внешний аккаунт, показываемый в настройках пользователя.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LinkedAccount {
    /// OAuth provider identifier such as `google`.
    pub(crate) provider: String,
    /// Human-readable provider label.
    #[serde(default)]
    pub(crate) provider_label: Option<String>,
    /// Email address exposed by the provider.
    #[serde(default)]
    pub(crate) email: Option<String>,
    /// Provider account display name.
    #[serde(default)]
    pub(crate) display_name: Option<String>,
    /// Provider-side account identifier, if exposed by the API.
    #[serde(default)]
    pub(crate) provider_user_id: Option<String>,
    /// RFC 3339 link timestamp, if exposed by the API.
    #[serde(default)]
    pub(crate) linked_at: Option<String>,
}

#[derive(Serialize)]
struct OAuthCompleteRequest {
    handoff_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    nickname: Option<String>,
}

async fn start_oauth(
    path: &str,
    flow: OAuthFlow,
    access_token: Option<String>,
) -> Result<String, String> {
    let mut request = Request::post(&url(path));
    if let Some(access_token) = access_token {
        request = request.header("Authorization", &format!("Bearer {access_token}"));
    }

    let response = request
        .json(&OAuthStartRequest { flow })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if !response.ok() {
        return Err(read_error(response).await);
    }

    let value = response
        .json::<Value>()
        .await
        .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())?;

    value
        .get("authorization_url")
        .or_else(|| value.get("redirect_url"))
        .or_else(|| value.get("url"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "Сервер не вернул ссылку для входа через Google.".to_owned())
}

async fn complete_oauth(path: &str, handoff_code: String) -> Result<OAuthCompletion, String> {
    let response = Request::post(&url(path))
        .json(&OAuthCompleteRequest {
            handoff_code,
            nickname: None,
        })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if !response.ok() {
        return Err(read_error(response).await);
    }

    let value = response
        .json::<Value>()
        .await
        .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())?;

    parse_oauth_completion(value)
}

fn parse_oauth_completion(value: Value) -> Result<OAuthCompletion, String> {
    if let Ok(response) = serde_json::from_value::<AuthResponse>(value.clone()) {
        return save_response(response).map(OAuthCompletion::Authenticated);
    }

    let kind = value
        .get("kind")
        .or_else(|| value.get("status"))
        .or_else(|| value.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    if kind == "registration_required" || kind == "RegistrationRequired" {
        return Ok(OAuthCompletion::RegistrationRequired(
            registration_required_from_value(&value),
        ));
    }

    if let Some(response_value) = value
        .get("auth_response")
        .or_else(|| value.get("auth"))
        .or_else(|| value.get("authenticated"))
    {
        let response = serde_json::from_value::<AuthResponse>(response_value.clone())
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())?;
        return save_response(response).map(OAuthCompletion::Authenticated);
    }

    if value.get("registration_required").and_then(Value::as_bool) == Some(true) {
        return Ok(OAuthCompletion::RegistrationRequired(
            registration_required_from_value(&value),
        ));
    }

    if kind == "linked" || value.get("linked").and_then(Value::as_bool) == Some(true) {
        return Ok(OAuthCompletion::Linked);
    }

    Err("Сервер вернул неизвестный результат входа через Google.".to_owned())
}

fn registration_required_from_value(value: &Value) -> OAuthRegistrationRequired {
    let details = value
        .get("registration")
        .or_else(|| value.get("registration_required"))
        .unwrap_or(value);

    OAuthRegistrationRequired {
        email: string_field(details, &["email", "provider_email"]),
        suggested_nickname: string_field(
            details,
            &["suggested_nickname", "nickname", "display_name", "name"],
        ),
    }
}

fn string_field(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

async fn parse_linked_accounts(
    response: gloo_net::http::Response,
) -> Result<Vec<LinkedAccount>, String> {
    let value = response
        .json::<Value>()
        .await
        .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())?;

    if let Ok(accounts) = serde_json::from_value::<Vec<LinkedAccount>>(value.clone()) {
        return Ok(accounts);
    }

    let accounts = value
        .get("accounts")
        .or_else(|| value.get("linked_accounts"))
        .cloned()
        .unwrap_or(Value::Null);

    serde_json::from_value::<Vec<LinkedAccount>>(accounts)
        .map_err(|_| "Не удалось прочитать список связанных аккаунтов.".to_owned())
}

fn changed_access_token(tokens: &storage::StoredTokens) -> Option<String> {
    let stored = storage::load()?;
    if stored.access_token == tokens.access_token && stored.refresh_token == tokens.refresh_token {
        return None;
    }

    Some(stored.access_token)
}

async fn post_json<T>(path: &str, request: &T) -> Result<AuthResponse, String>
where
    T: serde::Serialize,
{
    let response = Request::post(&url(path))
        .json(request)
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| NETWORK_ERROR_MESSAGE.to_owned())?;

    if response.ok() {
        response
            .json::<AuthResponse>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())
    } else {
        Err(read_error(response).await)
    }
}

async fn post_empty<T>(path: &str, request: &T) -> Result<(), String>
where
    T: serde::Serialize,
{
    let response = Request::post(&url(path))
        .json(request)
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        Ok(())
    } else {
        Err(read_error(response).await)
    }
}

fn save_response(response: AuthResponse) -> Result<AuthUser, String> {
    jwt::verify(&response.access_token)?;
    storage::save(&response.access_token, &response.refresh_token);
    Ok(response.user)
}

pub(crate) async fn read_error(response: gloo_net::http::Response) -> String {
    response
        .json::<ApiError>()
        .await
        .map(|error| error.message)
        .unwrap_or_else(|_| "Не удалось выполнить запрос. Попробуй еще раз.".to_owned())
}

pub(crate) fn url(path: &str) -> String {
    format!("{}{}", api_base_url().trim_end_matches('/'), path)
}

fn api_base_url() -> &'static str {
    option_env!("CHEENHUB_API_BASE_URL").unwrap_or(DEFAULT_API_BASE_URL)
}
