//! Заглушка потока логов для неподдерживаемой конфигурации сборки.

use cheenhub_contracts::rest::HostLogStreamMessage;
use futures_channel::mpsc;

pub(in crate::features::host_settings::log_stream) async fn run(
    _access_token: String,
    _output: mpsc::UnboundedSender<HostLogStreamMessage>,
) -> Result<(), String> {
    Err("Realtime-журнал недоступен для этой конфигурации клиента.".to_owned())
}
