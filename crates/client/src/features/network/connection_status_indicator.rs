//! Компонент индикатора состояния realtime-соединения.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::network::{NetworkQualityHandle, PingSample};
use crate::features::realtime::{
    RealtimeConnectionStatus, RealtimeFallbackInfo, RealtimeHandle, RealtimeTransportKind,
};

const GRAPH_WIDTH: f32 = 220.0;
const GRAPH_HEIGHT: f32 = 76.0;
const GRAPH_PADDING: f32 = 8.0;

/// Рендерит текущее состояние соединения WebTransport.
#[component]
pub(crate) fn RealtimeConnectionStatusIndicator() -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let network_quality = use_context::<NetworkQualityHandle>();
    let mut status = use_signal(|| realtime.connection_status());
    let mut fallback_info = use_signal(|| realtime.fallback_info());
    let mut is_open = use_signal(|| false);

    let status_realtime = realtime.clone();
    use_hook(move || {
        let realtime = status_realtime.clone();
        spawn(async move {
            let mut receiver = realtime.subscribe_connection_status();
            while let Some(next_status) = receiver.next().await {
                status.set(next_status);
            }
        });
    });

    let fallback_realtime = realtime.clone();
    use_hook(move || {
        let realtime = fallback_realtime.clone();
        spawn(async move {
            let mut receiver = realtime.subscribe_fallback_info();
            while let Some(next_fallback) = receiver.next().await {
                fallback_info.set(next_fallback);
            }
        });
    });

    let current_status = status();
    let is_connecting = matches!(
        current_status,
        RealtimeConnectionStatus::ConnectingWebTransport
            | RealtimeConnectionStatus::ConnectingWebSocketFallback
    );
    let quality = network_quality.current();
    let latest_ping = quality.latest_rtt_ms.map(format_ping);
    let ping_text = latest_ping.as_deref().unwrap_or("пинг ожидается");
    let latest_jitter = quality.latest_jitter_ms.map(format_ping);
    let jitter_text = latest_jitter.as_deref().unwrap_or("ожидается");
    let (label, tooltip, class) = match current_status {
        RealtimeConnectionStatus::ConnectingWebTransport => (
            "Подключение",
            "Пробуем подключиться через WebTransport",
            "border-blue-500/25 bg-blue-500/10 text-blue-300 hover:border-blue-400/40 hover:bg-blue-500/15",
        ),
        RealtimeConnectionStatus::ConnectingWebSocketFallback => (
            "Подключение",
            "Пробуем резервное подключение через WebSocket",
            "border-amber-500/25 bg-amber-500/10 text-amber-300 hover:border-amber-400/40 hover:bg-amber-500/15",
        ),
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebTransport) => (
            "Подключен",
            "Соединение установлено",
            "border-emerald-500/20 bg-emerald-500/10 text-emerald-300 hover:border-emerald-400/35 hover:bg-emerald-500/15",
        ),
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebSocketFallback) => (
            "Резерв",
            "Используется резервное соединение через WebSocket",
            "border-amber-500/25 bg-amber-500/10 text-amber-300 hover:border-amber-400/40 hover:bg-amber-500/15",
        ),
        RealtimeConnectionStatus::Disconnected => (
            "Отключен",
            "Соединение отключено",
            "border-red-500/20 bg-red-500/10 text-red-300 hover:border-red-400/35 hover:bg-red-500/15",
        ),
    };
    let active_fallback = active_fallback_info(current_status, fallback_info());
    let tooltip = match active_fallback {
        Some(fallback) => format!(
            "{tooltip}. Код диагностики: {}. {ping_text}",
            fallback.diagnostic_code()
        ),
        None => format!("{tooltip}. {ping_text}"),
    };
    let points = graph_points(&quality.samples);
    let max_ping = quality
        .samples
        .iter()
        .map(|sample| sample.rtt_ms)
        .reduce(f64::max)
        .map(format_ping)
        .unwrap_or_else(|| "нет данных".to_string());
    let max_jitter = quality
        .samples
        .iter()
        .map(|sample| sample.jitter_ms)
        .reduce(f64::max)
        .map(format_ping)
        .unwrap_or_else(|| "нет данных".to_string());

    rsx! {
        div { class: "relative shrink-0",
            button {
                r#type: "button",
                class: "group relative flex h-9 w-9 items-center justify-center rounded-xl border {class}",
                "aria-label": "{tooltip}",
                "aria-expanded": "{is_open()}",
                onclick: move |_| is_open.set(!is_open()),
                span { class: "pointer-events-none absolute bottom-[calc(100%+10px)] left-0 z-[90] w-max min-w-[184px] translate-y-1 rounded-xl border border-zinc-800 bg-zinc-950/95 p-3 text-left opacity-0 shadow-[0_16px_40px_rgba(0,0,0,.45)] backdrop-blur-xl transition-[opacity,transform] duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100",
                    span { class: "block text-[12px] font-medium text-zinc-100", "{label}" }
                    span { class: "mt-1 block text-[11px] text-zinc-500", "{tooltip}" }
                }
                if is_connecting {
                    svg { class: "h-[18px] w-[18px] animate-spin", fill: "none", view_box: "0 0 24 24", "aria-hidden": "true",
                        circle { class: "opacity-25", cx: "12", cy: "12", r: "9", stroke: "currentColor", stroke_width: "2.5" }
                        path { class: "opacity-90", d: "M21 12a9 9 0 0 0-9-9", stroke: "currentColor", stroke_width: "2.5", stroke_linecap: "round" }
                    }
                } else {
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
            if is_open() {
                div {
                    class: "absolute bottom-[calc(100%+10px)] left-0 z-[100] w-[260px] rounded-xl border border-zinc-800 bg-zinc-950/95 p-3 text-left text-zinc-200 shadow-[0_18px_46px_rgba(0,0,0,.5)] backdrop-blur-xl",
                    if let Some(fallback) = active_fallback {
                        div { class: "mb-3 rounded-lg border border-amber-500/20 bg-amber-500/10 p-2.5",
                            span { class: "block text-[12px] font-semibold text-amber-200", "Резервный режим" }
                            span { class: "mt-1 block text-[11px] leading-4 text-amber-100/70",
                                "WebTransport недоступен, используется WebSocket. Голос и трансляции могут быть менее стабильными."
                            }
                            div { class: "mt-2 flex items-center justify-between gap-3 font-mono text-[10px] text-amber-200/70",
                                span { "{fallback.diagnostic_code()}" }
                                span { "{fallback.webtransport_elapsed_ms} мс" }
                            }
                        }
                    }
                    div { class: "flex items-start justify-between gap-3",
                        div {
                            span { class: "block text-[12px] font-semibold text-zinc-100", "Пинг" }
                            span { class: "mt-1 block text-[11px] text-zinc-500", "Последняя минута" }
                        }
                        div { class: "text-right",
                            span { class: "block text-[13px] font-semibold text-zinc-100", "{ping_text}" }
                            span { class: "mt-1 block text-[11px] text-zinc-500", "пик {max_ping}" }
                        }
                    }
                    div { class: "mt-3 flex items-center justify-between gap-3 border-t border-zinc-800/80 pt-3",
                        div {
                            span { class: "block text-[12px] font-semibold text-zinc-100", "Джиттер" }
                            span { class: "mt-1 block text-[11px] text-zinc-500", "Сглаженное изменение RTT" }
                        }
                        div { class: "text-right",
                            span { class: "block text-[13px] font-semibold text-zinc-100", "{jitter_text}" }
                            span { class: "mt-1 block text-[11px] text-zinc-500", "пик {max_jitter}" }
                        }
                    }
                    div { class: "mt-3 h-[84px] rounded-lg border border-zinc-800/80 bg-zinc-900/45 p-2",
                        if quality.samples.is_empty() {
                            div { class: "grid h-full place-items-center text-[11px] text-zinc-500",
                                "Ждем первое измерение"
                            }
                        } else {
                            svg {
                                class: "h-full w-full overflow-visible",
                                view_box: "0 0 220 76",
                                preserve_aspect_ratio: "none",
                                "aria-hidden": "true",
                                path { d: "M8 18 H212 M8 38 H212 M8 58 H212", stroke: "rgb(63 63 70)", stroke_width: "1", stroke_dasharray: "3 5" }
                                polyline { points: "{points}", fill: "none", stroke: "rgb(52 211 153)", stroke_width: "2.4", stroke_linecap: "round", stroke_linejoin: "round" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn active_fallback_info(
    status: RealtimeConnectionStatus,
    fallback: Option<RealtimeFallbackInfo>,
) -> Option<RealtimeFallbackInfo> {
    matches!(
        status,
        RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebSocketFallback)
    )
    .then_some(fallback)
    .flatten()
}

fn format_ping(rtt_ms: f64) -> String {
    if rtt_ms < 10.0 {
        format!("{rtt_ms:.1} мс")
    } else {
        format!("{:.0} мс", rtt_ms.round())
    }
}

fn graph_points(samples: &[PingSample]) -> String {
    if samples.is_empty() {
        return String::new();
    }

    let first_ms = samples
        .first()
        .map(|sample| sample.received_at_ms)
        .unwrap_or_default();
    let last_ms = samples
        .last()
        .map(|sample| sample.received_at_ms)
        .unwrap_or(first_ms);
    let time_span = last_ms.saturating_sub(first_ms).max(1) as f32;
    let max_rtt = samples
        .iter()
        .map(|sample| sample.rtt_ms)
        .reduce(f64::max)
        .unwrap_or(1.0)
        .max(1.0) as f32;
    let inner_width = GRAPH_WIDTH - GRAPH_PADDING * 2.0;
    let inner_height = GRAPH_HEIGHT - GRAPH_PADDING * 2.0;

    samples
        .iter()
        .map(|sample| {
            let x = GRAPH_PADDING
                + sample.received_at_ms.saturating_sub(first_ms) as f32 / time_span * inner_width;
            let y = GRAPH_HEIGHT - GRAPH_PADDING - sample.rtt_ms as f32 / max_rtt * inner_height;
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
