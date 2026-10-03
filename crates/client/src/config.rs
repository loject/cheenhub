//! Конфигурация сетевых адресов клиента.

use url::Url;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3000";

/// Собирает URL REST API из относительного пути.
pub(crate) fn api_url(path: &str) -> Result<Url, String> {
    api_url_from_base(configured_base_url(), path)
}

/// Возвращает URL WebSocket fallback для realtime-соединения.
pub(crate) fn realtime_websocket_url() -> Result<Url, String> {
    realtime_websocket_url_from_base(configured_base_url())
}

/// Возвращает URL WebTransport для realtime-соединения.
pub(crate) fn realtime_webtransport_url() -> Result<Url, String> {
    realtime_webtransport_url_from_base(configured_base_url())
}

fn configured_base_url() -> &'static str {
    option_env!("CHEENHUB_BASE_URL").unwrap_or(DEFAULT_BASE_URL)
}

fn api_url_from_base(base_url: &str, path: &str) -> Result<Url, String> {
    let mut url = parse_base_url(base_url)?;
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let path = path.trim_start_matches('/');
    let api_path = if path.is_empty() {
        "/api".to_owned()
    } else {
        format!("/api/{path}")
    };
    url.set_path(&api_path);
    if !query.is_empty() {
        url.set_query(Some(query));
    }
    Ok(url)
}

fn realtime_websocket_url_from_base(base_url: &str) -> Result<Url, String> {
    let mut url = parse_base_url(base_url)?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => unreachable!("схема проверена при разборе базового URL"),
    };
    url.set_scheme(scheme)
        .map_err(|_| "Не удалось установить WebSocket-схему".to_owned())?;
    url.set_path("/api/realtime/ws");
    Ok(url)
}

fn realtime_webtransport_url_from_base(base_url: &str) -> Result<Url, String> {
    let mut url = parse_base_url(base_url)?;
    url.set_scheme("https")
        .map_err(|_| "Не удалось установить WebTransport-схему".to_owned())?;
    url.set_path("/realtime");
    Ok(url)
}

fn parse_base_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value)
        .map_err(|error| format!("CHEENHUB_BASE_URL содержит некорректный URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "CHEENHUB_BASE_URL должен использовать схему http или https, получена {}",
            url.scheme()
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("CHEENHUB_BASE_URL не должен содержать учетные данные".to_owned());
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err(
            "CHEENHUB_BASE_URL должен содержать только схему, хост и необязательный порт"
                .to_owned(),
        );
    }

    Ok(url)
}

pub(crate) fn public_url(path: &str) -> Result<Url, String> {
    let mut url = parse_base_url(configured_base_url())?;
    url.set_path(&format!("/{}", path.trim_start_matches('/')));
    Ok(url)
}

#[cfg(test)]
mod tests;
