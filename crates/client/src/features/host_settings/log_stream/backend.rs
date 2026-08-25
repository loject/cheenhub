//! Платформенно-независимый вход в realtime-поток логов.

use cheenhub_contracts::rest::HostLogStreamMessage;
use dioxus::prelude::{debug, warn};
use futures_channel::mpsc;

use super::platform;

/// Подключается к одному сеансу realtime-журнала.
pub(in crate::features::host_settings) async fn run(
    access_token: String,
    output: mpsc::UnboundedSender<HostLogStreamMessage>,
) -> Result<(), String> {
    debug!("opening host backend log stream");
    let result = platform::run(access_token, output).await;
    if let Err(error) = &result {
        warn!(%error, "host backend log stream disconnected");
    }
    result
}
