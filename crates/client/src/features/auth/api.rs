//! Authentication API client.

use cheenhub_contracts::rest::{
    ApiError, AuthResponse, AuthUser, LoginRequest, LogoutRequest, RefreshRequest, RegisterRequest,
};
use gloo_net::http::Request;

use crate::features::auth::{jwt, storage};

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:3000/api";

/// Registers a new account and stores the returned tokens.
pub(crate) async fn register(request: RegisterRequest) -> Result<AuthUser, String> {
    let response = post_json("/auth/register", &request).await?;
    save_response(response)
}

/// Logs in with email/password and stores the returned tokens.
pub(crate) async fn login(request: LoginRequest) -> Result<AuthUser, String> {
    let response = post_json("/auth/login", &request).await?;
    save_response(response)
}

/// Returns whether a token pair is available locally.
pub(crate) fn has_tokens() -> bool {
    storage::load().is_some()
}

/// Loads the current authenticated user, refreshing access token when needed.
pub(crate) async fn current_user() -> Result<AuthUser, String> {
    let access_token = fresh_access_token().await?;
    let response = Request::get(&url("/auth/me"))
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.status() == 401 {
        let access_token = rotate_refresh().await?;
        return Request::get(&url("/auth/me"))
            .header("Authorization", &format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|_| "Не удалось связаться с сервером.".to_owned())?
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

/// Invalidates the current refresh session and clears stored tokens.
#[allow(dead_code)]
pub(crate) async fn logout() -> Result<(), String> {
    let Some(tokens) = storage::load() else {
        storage::clear();
        return Ok(());
    };

    let response = Request::post(&url("/auth/logout"))
        .json(&LogoutRequest {
            refresh_token: tokens.refresh_token,
        })
        .map_err(|_| "Не удалось подготовить запрос.".to_owned())?
        .send()
        .await
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;
    storage::clear();

    if response.ok() {
        Ok(())
    } else {
        Err(read_error(response).await)
    }
}

async fn fresh_access_token() -> Result<String, String> {
    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };

    if jwt::is_fresh(&tokens.access_token) {
        return Ok(tokens.access_token);
    }

    rotate_refresh().await
}

async fn rotate_refresh() -> Result<String, String> {
    let Some(tokens) = storage::load() else {
        return Err("Войди, чтобы продолжить.".to_owned());
    };
    let response: AuthResponse = post_json(
        "/auth/refresh",
        &RefreshRequest {
            refresh_token: tokens.refresh_token,
        },
    )
    .await
    .inspect_err(|_| storage::clear())?;

    jwt::verify(&response.access_token)?;
    storage::save(&response.access_token, &response.refresh_token);
    Ok(response.access_token)
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
        .map_err(|_| "Не удалось связаться с сервером.".to_owned())?;

    if response.ok() {
        response
            .json::<AuthResponse>()
            .await
            .map_err(|_| "Не удалось прочитать ответ сервера.".to_owned())
    } else {
        Err(read_error(response).await)
    }
}

fn save_response(response: AuthResponse) -> Result<AuthUser, String> {
    jwt::verify(&response.access_token)?;
    storage::save(&response.access_token, &response.refresh_token);
    Ok(response.user)
}

async fn read_error(response: gloo_net::http::Response) -> String {
    response
        .json::<ApiError>()
        .await
        .map(|error| error.message)
        .unwrap_or_else(|_| "Не удалось выполнить запрос. Попробуй еще раз.".to_owned())
}

fn url(path: &str) -> String {
    format!("{}{}", api_base_url().trim_end_matches('/'), path)
}

fn api_base_url() -> &'static str {
    option_env!("CHEENHUB_API_BASE_URL").unwrap_or(DEFAULT_API_BASE_URL)
}
