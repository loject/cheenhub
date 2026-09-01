//! HTTP-хелперы клиентского REST API.

use dioxus::logger::tracing::{debug, error};

mod native;

/// Собирает полный URL REST API из относительного пути.
pub(crate) fn url(path: &str) -> String {
    crate::config::api_url(path)
        .map(|url| url.to_string())
        .unwrap_or_else(|config_error| {
            error!(
                error = %config_error,
                path,
                "failed to build client REST API URL"
            );
            panic!("Некорректная конфигурация CHEENHUB_BASE_URL: {config_error}");
        })
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

#[cfg(test)]
fn attach_platform_user_agent(headers: &mut reqwest::header::HeaderMap) {
    let Some(user_agent) = native::client_user_agent() else {
        return;
    };

    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(user_agent),
    );
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, USER_AGENT};

    use super::{attach_platform_user_agent, native};

    #[test]
    fn attaches_platform_user_agent_only_when_available() {
        let mut headers = HeaderMap::new();

        attach_platform_user_agent(&mut headers);

        let actual = headers
            .get(USER_AGENT)
            .and_then(|value| value.to_str().ok());

        assert_eq!(actual, native::client_user_agent());
    }
}
