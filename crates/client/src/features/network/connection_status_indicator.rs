//! Realtime connection status indicator component.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle, RealtimeTransportKind};

/// Renders the current WebTransport connection state.
#[component]
pub(crate) fn RealtimeConnectionStatusIndicator() -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let mut status = use_signal(|| realtime.connection_status());

    use_hook(move || {
        let realtime = realtime.clone();
        spawn(async move {
            let mut receiver = realtime.subscribe_connection_status();
            while let Some(next_status) = receiver.next().await {
                status.set(next_status);
            }
        });
    });

    let (label, tooltip, class) = match status() {
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebTransport) => (
            "Подключен",
            "Соединение установлено",
            "border-emerald-500/20 bg-emerald-500/10 text-emerald-300 hover:border-emerald-400/35 hover:bg-emerald-500/15",
        ),
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebSocketFallback) => (
            "Fallback",
            "Используется более медленный WebSocket fallback",
            "border-amber-500/25 bg-amber-500/10 text-amber-300 hover:border-amber-400/40 hover:bg-amber-500/15",
        ),
        RealtimeConnectionStatus::Disconnected => (
            "Отключен",
            "Соединение отключено",
            "border-red-500/20 bg-red-500/10 text-red-300 hover:border-red-400/35 hover:bg-red-500/15",
        ),
    };

    rsx! {
        button {
            r#type: "button",
            class: "group relative flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border {class}",
            "aria-label": "{tooltip}",
            span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-0 z-[90] w-max min-w-[170px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 p-3 text-left opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                span { class: "block text-[12px] font-medium text-zinc-100", "{label}" }
                span { class: "mt-1 block text-[11px] text-zinc-500", "{tooltip}" }
            }
            svg { class: "h-[18px] w-[18px]", fill: "none", stroke: "currentColor", stroke_width: "1.9", view_box: "0 0 24 24", "aria-hidden": "true",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18.5v-3.25" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.5 18.5h7" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.25 13.75a4 4 0 0 1 5.5 0" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6.5 11a8 8 0 0 1 11 0" }
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.75 8.25a12 12 0 0 1 16.5 0" }
                circle { cx: "12", cy: "18.5", r: "1.15", fill: "currentColor", stroke: "none" }
            }
        }
    }
}
