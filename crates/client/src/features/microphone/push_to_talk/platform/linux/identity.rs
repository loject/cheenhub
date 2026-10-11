//! Идентификация установленного приложения до первого portal-вызова; launcher не создаётся.

use ashpd::{AppID, zbus::Connection};

/// Совпадает с Linux bundle identifier и именем desktop entry пакета.
pub(super) const APP_ID: &str = "ru.cheenhub";

/// Регистрирует выделенное соединение; sandbox использует свою системную идентичность.
///
/// Разработка может выбрать отдельный установленный desktop entry через
/// CHEENHUB_PORTAL_APP_ID; runtime никогда не создаёт и не перезаписывает launcher.
///
/// # Errors
/// Возвращает ошибку D-Bus или Registry, если app ID не соответствует desktop entry.
pub(super) async fn connect() -> Result<Connection, String> {
    let app_id = std::env::var("CHEENHUB_PORTAL_APP_ID").unwrap_or_else(|_| {
        option_env!("CHEENHUB_PORTAL_APP_ID")
            .unwrap_or(APP_ID)
            .into()
    });
    let app_id = AppID::try_from(app_id.as_str()).map_err(|error| error.to_string())?;
    let connection = ashpd::zbus::connection::Builder::session()
        .map_err(|error| error.to_string())?
        .method_timeout(std::time::Duration::from_secs(5))
        .build()
        .await
        .map_err(|error| error.to_string())?;
    dioxus::prelude::debug!(app_id = %app_id, kind = "portal_identity", "Linux PTT registering application identity");
    ashpd::register_host_app_with_connection(connection.clone(), app_id)
        .await
        .map_err(|error| format!("host portal registry: {error}"))?;
    Ok(connection)
}
