//! Подтверждение запуска после первого закодированного кадра.

use super::StartupInfo;
use dioxus::logger::tracing::info;
use futures_channel::oneshot;
use std::time::Instant;

/// Один раз сообщает размер источника после получения закодированного кадра.
pub(super) fn confirm_first_frame(
    received: &mut bool,
    sender: &mut Option<oneshot::Sender<Result<StartupInfo, String>>>,
    source_id: &str,
    source_size: (u32, u32),
    started: Instant,
) {
    if *received {
        return;
    }
    *received = true;
    if let Some(sender) = sender.take() {
        let _ = sender.send(Ok(StartupInfo {
            width: source_size.0,
            height: source_size.1,
        }));
    }
    info!(source_id = %source_id, native_width = source_size.0, native_height = source_size.1, startup_elapsed_ms = started.elapsed().as_millis(), "Windows screen capture first encoded frame confirmed");
}
