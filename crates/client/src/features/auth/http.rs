//! HTTP-хелперы клиентского REST API.

use dioxus::logger::tracing::debug;

mod native;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:3000/api";

/// Собирает полный URL REST API из относительного пути.
pub(crate) fn url(path: &str) -> String {
    format!("{}{}", api_base_url().trim_end_matches('/'), path)
}

/// Создает GET-запрос к REST API.
pub(crate) fn get(path: &str) -> reqwest::RequestBuilder {
    request(reqwest::Method::GET, path)
}

/// Создает POST-запрос к REST API.
pub(crate) fn post(path: &str) -> reqwest::RequestBuilder {
    request(reqwest::Method::POST, path)
}

/// Создает PUT-запрос к REST API.
pub(crate) fn put(path: &str) -> reqwest::RequestBuilder {
    request(reqwest::Method::PUT, path)
}

/// Создает PATCH-запрос к REST API.
pub(crate) fn patch(path: &str) -> reqwest::RequestBuilder {
    request(reqwest::Method::PATCH, path)
}

/// Создает DELETE-запрос к REST API.
pub(crate) fn delete(path: &str) -> reqwest::RequestBuilder {
    request(reqwest::Method::DELETE, path)
}

fn request(method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
    let request = reqwest::Client::new().request(method, url(path));
    let Some(user_agent) = native::client_user_agent() else {
        return request;
    };

    debug!(
        client_platform = native::client_platform(),
        "attaching native client identity to auth HTTP request"
    );
    request.header(reqwest::header::USER_AGENT, user_agent)
}

fn api_base_url() -> &'static str {
    option_env!("CHEENHUB_API_BASE_URL").unwrap_or(DEFAULT_API_BASE_URL)
}

#[cfg(test)]
mod tests {
    use reqwest::header::USER_AGENT;

    use super::{get, native};

    #[test]
    fn attaches_platform_user_agent_only_when_available() {
        let request = get("/auth/sessions")
            .build()
            .expect("статический auth-запрос должен собираться");
        let actual = request
            .headers()
            .get(USER_AGENT)
            .and_then(|value| value.to_str().ok());

        assert_eq!(actual, native::client_user_agent());
    }
}
