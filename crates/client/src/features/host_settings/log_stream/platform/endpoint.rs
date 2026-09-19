//! Адрес WebSocket-потока журнала хоста.

use url::Url;

pub(super) fn url() -> Result<Url, String> {
    let mut url = crate::config::api_url("/host-settings/logs/ws")?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        _ => unreachable!("схема проверена при разборе базового URL"),
    };
    url.set_scheme(scheme)
        .map_err(|_| "Не удалось установить WebSocket-схему".to_owned())?;
    Ok(url)
}
