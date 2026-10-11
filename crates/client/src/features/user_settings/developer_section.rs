//! Инструменты локальной проверки realtime в режиме разработчика.

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};
use dioxus::prelude::*;
use futures_util::StreamExt;

use super::styles::{input_class, primary_button_class};
use crate::features::developer_mode::{DeveloperModeHandle, MAX_PING_DELAY_MS};

/// Рендерит сетевые инструменты и сброс локального режима разработчика.
#[component]
pub(crate) fn DeveloperSettingsSection(on_reset: EventHandler<()>) -> Element {
    let mut developer_mode = use_context::<DeveloperModeHandle>();
    let realtime = use_context::<RealtimeHandle>();
    let mut status = use_signal(|| realtime.connection_status());
    use_hook(move || {
        spawn(async move {
            let mut receiver = realtime.subscribe_connection_status();
            while let Some(next) = receiver.next().await {
                status.set(next);
            }
        });
    });
    let connecting = matches!(
        status(),
        RealtimeConnectionStatus::ConnectingWebTransport
            | RealtimeConnectionStatus::ConnectingWebSocketFallback
    );
    let settings = developer_mode.current();
    let mut delay_input = use_signal(|| settings.effective_ping_delay_ms().to_string());
    let mut error = use_signal(|| None::<String>);
    let mut saved = use_signal(|| false);

    rsx! {
        div { class: "space-y-4",
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                h3 { class: "text-[16px] font-semibold tracking-[-0.03em] text-zinc-50", "Соединение" }
                label { class: "mt-4 flex cursor-pointer items-center justify-between gap-4",
                    span {
                        span { class: "block text-[14px] font-medium text-zinc-100", "Принудительно использовать WebSocket" }
                        span { class: "mt-1 block text-[12px] leading-5 text-zinc-500", "Смена транспорта переподключит соединение. Активный разговор или трансляция могут прерваться." }
                    }
                    input {
                        r#type: "checkbox",
                        checked: settings.effective_force_websocket(),
                        onchange: move |event| {
                            let mut next = developer_mode.current();
                            next.force_websocket = event.checked();
                            if let Err(message) = developer_mode.save(next) {
                                error.set(Some(message));
                            }
                        },
                    }
                }
                if connecting {
                    p { role: "status", class: "mt-3 flex items-center gap-2 text-[12px] text-zinc-400",
                        span { class: "h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-300" }
                        "Переподключаемся…"
                    }
                } else if status() == RealtimeConnectionStatus::Disconnected {
                    p { role: "status", class: "mt-3 text-[12px] text-red-200", "Соединение недоступно. Повторная попытка выполняется автоматически." }
                }
                form { class: "mt-5 space-y-3",
                    onsubmit: move |event| {
                        event.prevent_default();
                        let Ok(delay) = delay_input().parse::<u32>() else {
                            error.set(Some("Введи целое число от 0 до 10000.".to_owned()));
                            return;
                        };
                        if delay > MAX_PING_DELAY_MS {
                            error.set(Some("Задержка должна быть от 0 до 10000 мс.".to_owned()));
                            return;
                        }
                        let mut next = developer_mode.current();
                        next.ping_delay_ms = delay;
                        if let Err(message) = developer_mode.save(next) {
                            error.set(Some(message));
                            return;
                        }
                        error.set(None);
                        saved.set(true);
                    },
                    label { class: "block",
                        span { class: "mb-1.5 block text-[12px] font-medium text-zinc-300", "Добавочная задержка ping, мс" }
                        input {
                            r#type: "number",
                            min: "0", max: "10000", step: "1",
                            value: delay_input(),
                            class: input_class(),
                            oninput: move |event| {
                                delay_input.set(event.value());
                                error.set(None);
                                saved.set(false);
                            },
                        }
                    }
                    p { class: "text-[12px] leading-5 text-zinc-500", "Задерживается только измерение ping, сообщения и медиа передаются как обычно. От 1500 мс измерения могут завершаться по таймауту и вызывать переподключение." }
                    if let Some(message) = error() {
                        p { role: "alert", class: "text-[12px] text-red-200", "{message}" }
                    }
                    if saved() {
                        p { role: "status", class: "text-[12px] text-emerald-200", "Задержка сохранена." }
                    }
                    button { r#type: "submit", class: primary_button_class(), "Сохранить задержку" }
                }
            }
            div { class: "rounded-2xl border border-zinc-800 bg-zinc-950/70 p-4",
                h3 { class: "text-[16px] font-semibold text-zinc-50", "Сброс режима" }
                p { class: "mt-1 text-[12px] leading-5 text-zinc-500", "Удалит настройки Developer Mode и вернет обычное подключение. Эта вкладка исчезнет." }
                button {
                    r#type: "button",
                    class: "mt-4 rounded-xl border border-red-500/20 bg-red-500/10 px-4 py-2 text-[12px] font-medium text-red-200 transition hover:bg-red-500/15",
                    onclick: move |_| {
                        if let Err(message) = developer_mode.reset() {
                            error.set(Some(message));
                            return;
                        }
                        on_reset.call(());
                    },
                    "Сбросить Developer Mode"
                }
            }
        }
    }
}
