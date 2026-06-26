//! HTTP-хелперы клиентского REST API.

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:3000/api";

/// Собирает полный URL REST API из относительного пути.
pub(crate) fn url(path: &str) -> String {
    format!("{}{}", api_base_url().trim_end_matches('/'), path)
}

/// Создает GET-запрос к REST API.
pub(crate) fn get(path: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new().get(url(path))
}

/// Создает POST-запрос к REST API.
pub(crate) fn post(path: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new().post(url(path))
}

/// Создает PUT-запрос к REST API.
pub(crate) fn put(path: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new().put(url(path))
}

/// Создает PATCH-запрос к REST API.
pub(crate) fn patch(path: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new().patch(url(path))
}

/// Создает DELETE-запрос к REST API.
pub(crate) fn delete(path: &str) -> reqwest::RequestBuilder {
    reqwest::Client::new().delete(url(path))
}

fn api_base_url() -> &'static str {
    option_env!("CHEENHUB_API_BASE_URL").unwrap_or(DEFAULT_API_BASE_URL)
}
