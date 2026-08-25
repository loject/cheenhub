//! Конфигурация сетевых адресов клиента.

use url::Url;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:3000";

/// Собирает URL REST API из относительного пути.
pub(crate) fn api_url(path: &str) -> Result<Url, String> {
    api_url_from_base(configured_base_url(), path)
}

/// Возвращает URL realtime-потока логов владельца хоста.
pub(crate) fn host_logs_websocket_url() -> Result<Url, String> {
    host_logs_websocket_url_from_base(configured_base_url())
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

fn host_logs_websocket_url_from_base(base_url: &str) -> Result<Url, String> {
    let mut url = api_url_from_base(base_url, "/host-settings/logs/ws")?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => unreachable!("схема проверена при разборе базового URL"),
    };
    url.set_scheme(scheme)
        .map_err(|_| "Не удалось установить WebSocket-схему".to_owned())?;
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

#[cfg(test)]
mod tests {
    use super::{
        api_url_from_base, host_logs_websocket_url_from_base, realtime_websocket_url_from_base,
        realtime_webtransport_url_from_base,
    };

    #[test]
    fn derives_all_endpoints_from_http_base_url() {
        let base_url = "http://192.168.2.2:3000";

        assert_eq!(
            api_url_from_base(base_url, "/auth/sessions")
                .expect("REST URL должен собираться")
                .as_str(),
            "http://192.168.2.2:3000/api/auth/sessions"
        );
        assert_eq!(
            api_url_from_base(base_url, "/friends/search?q=alice%20smith")
                .expect("REST URL с query string должен собираться")
                .as_str(),
            "http://192.168.2.2:3000/api/friends/search?q=alice%20smith"
        );
        assert_eq!(
            host_logs_websocket_url_from_base(base_url)
                .expect("URL логов должен собираться")
                .as_str(),
            "ws://192.168.2.2:3000/api/host-settings/logs/ws"
        );
        assert_eq!(
            realtime_websocket_url_from_base(base_url)
                .expect("WebSocket URL должен собираться")
                .as_str(),
            "ws://192.168.2.2:3000/api/realtime/ws"
        );
        assert_eq!(
            realtime_webtransport_url_from_base(base_url)
                .expect("WebTransport URL должен собираться")
                .as_str(),
            "https://192.168.2.2:3000/realtime"
        );
    }

    #[test]
    fn derives_secure_endpoints_from_https_base_url() {
        let base_url = "https://cheenhub.test:8443/";

        assert_eq!(
            host_logs_websocket_url_from_base(base_url)
                .expect("URL логов должен собираться")
                .as_str(),
            "wss://cheenhub.test:8443/api/host-settings/logs/ws"
        );
        assert_eq!(
            realtime_websocket_url_from_base(base_url)
                .expect("WebSocket URL должен собираться")
                .as_str(),
            "wss://cheenhub.test:8443/api/realtime/ws"
        );
        assert_eq!(
            realtime_webtransport_url_from_base(base_url)
                .expect("WebTransport URL должен собираться")
                .as_str(),
            "https://cheenhub.test:8443/realtime"
        );
    }

    #[test]
    fn rejects_base_url_with_path_or_unsupported_scheme() {
        assert!(api_url_from_base("https://cheenhub.test/root", "/users").is_err());
        assert!(realtime_websocket_url_from_base("ftp://cheenhub.test").is_err());
    }
}
