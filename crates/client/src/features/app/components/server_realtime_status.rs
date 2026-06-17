//! Поверхность realtime-статуса в боковой панели сервера.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::network::RealtimeConnectionStatusIndicator;
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle, RealtimeTransportKind};

use super::server_rooms_sidebar_styles as sidebar_styles;

/// Рендерит строку realtime-статуса в боковой панели сервера.
#[component]
pub(crate) fn ServerRealtimeStatus(
    server_name: String,
    settings_workspace_active: bool,
) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut realtime_status = use_signal(|| realtime.connection_status());
    let connection_status_class =
        sidebar_styles::connection_status_class(settings_workspace_active);
    let connection_details_class =
        sidebar_styles::connection_details_class(settings_workspace_active);

    use_hook(move || {
        let realtime = realtime.clone();
        spawn(async move {
            let mut receiver = realtime.subscribe_connection_status();
            while let Some(next_status) = receiver.next().await {
                realtime_status.set(next_status);
            }
            debug!("realtime status sidebar subscription closed");
        });
    });

    let realtime_status_label = realtime_connection_status_label(realtime_status());

    rsx! {
        div { class: connection_status_class,
            RealtimeConnectionStatusIndicator {}
            div { class: connection_details_class,
                div { class: "truncate text-[11px] font-medium text-zinc-100", "{server_name}" }
                div { class: "truncate text-[11px] text-zinc-500", "{realtime_status_label}" }
            }
        }
    }
}

fn realtime_connection_status_label(status: RealtimeConnectionStatus) -> &'static str {
    match status {
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebTransport) => {
            "Подключено через WebTransport"
        }
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebSocketFallback) => {
            "Подключено через WebSocket fallback"
        }
        RealtimeConnectionStatus::Disconnected => "Отключено",
    }
}
