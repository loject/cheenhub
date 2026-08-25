//! Realtime-журнал бэкенда в настройках владельца хоста.

use cheenhub_contracts::rest::{HostLogEntry, HostLogStreamMessage};
use dioxus::prelude::*;
use futures_channel::mpsc;
use futures_util::{FutureExt, StreamExt};

use crate::features::auth::api as auth_api;
use crate::features::runtime::sleep_ms;

use super::log_stream;
use super::tabs::{HostSettingsTab, host_settings_tabs};

const RECONNECT_DELAY_MS: u32 = 1_500;
const MAX_CLIENT_ENTRIES: usize = 3_000;
const MAX_RENDERED_ENTRIES: usize = 1_000;

#[derive(Clone, Debug, PartialEq, Eq)]
enum LogStreamState {
    Connecting,
    Live,
    Error(String),
}

/// Рендерит realtime-журнал бэкенда, доступный владельцу хоста.
#[component]
pub(crate) fn HostLogsPage() -> Element {
    let mut entries = use_signal(Vec::<HostLogEntry>::new);
    let mut stream_state = use_signal(|| LogStreamState::Connecting);
    let mut search = use_signal(String::new);
    let mut level_filter = use_signal(|| "ALL".to_owned());
    let mut paused = use_signal(|| false);
    let mut paused_entries = use_signal(Vec::<HostLogEntry>::new);
    let mut paused_snapshot = use_signal(|| None::<Vec<HostLogEntry>>);

    use_future(move || async move {
        loop {
            stream_state.set(LogStreamState::Connecting);

            let access_token = match auth_api::fresh_access_token().await {
                Ok(access_token) => access_token,
                Err(error) => {
                    stream_state.set(LogStreamState::Error(error));
                    sleep_ms(RECONNECT_DELAY_MS).await;
                    continue;
                }
            };

            let (sender, mut receiver) = mpsc::unbounded();
            let connection = log_stream::run(access_token, sender).fuse();
            futures_util::pin_mut!(connection);

            let mut retry_allowed = true;

            loop {
                futures_util::select! {
                    result = connection => {
                        if let Err(error) = result && !matches!(stream_state(), LogStreamState::Error(_)) {
                                stream_state.set(LogStreamState::Error(error));
                            }
                        break;
                    },
                    message = receiver.next().fuse() => {
                        let Some(message) = message else {
                            break;
                        };
                        match message {
                            HostLogStreamMessage::Snapshot { entries: snapshot } => {
                                if paused() {
                                    paused_snapshot.set(Some(snapshot));
                                    paused_entries.set(Vec::new());
                                } else {
                                    entries.set(snapshot);
                                }
                                stream_state.set(LogStreamState::Live);
                            }
                            HostLogStreamMessage::Entry { entry } => {
                                if paused() {
                                    paused_entries.with_mut(|items| {
                                        push_entry(items, entry);
                                    });
                                } else {
                                    entries.with_mut(|items| {
                                        push_entry(items, entry);
                                    });
                                }
                                stream_state.set(LogStreamState::Live);
                            }
                            HostLogStreamMessage::Error { message, retryable } => {
                                retry_allowed = retryable;
                                stream_state.set(LogStreamState::Error(message));
                            }
                        }
                    },
                }
            }

            if !retry_allowed {
                return;
            }
            sleep_ms(RECONNECT_DELAY_MS).await;
        }
    });

    let query = search().trim().to_ascii_lowercase();
    let selected_level = level_filter();
    let all_entries = entries();
    let latest_entry_id = all_entries.last().map(|entry| entry.id);
    let pending_count = paused_entries().len();

    let total_count = all_entries.len();
    let error_count = count_level(&all_entries, "ERROR");
    let warn_count = count_level(&all_entries, "WARN");
    let info_count = count_level(&all_entries, "INFO");
    let debug_count = count_level(&all_entries, "DEBUG");
    let trace_count = count_level(&all_entries, "TRACE");

    let visible_entries = all_entries
        .iter()
        .rev()
        .filter(|entry| {
            (selected_level == "ALL" || entry.level == selected_level)
                && (query.is_empty() || entry_matches(entry, &query))
        })
        .take(MAX_RENDERED_ENTRIES)
        .cloned()
        .collect::<Vec<_>>();

    let (status_class, status_label) = if paused() {
        (
            "border border-amber-400/20 bg-amber-400/10 text-amber-100",
            if pending_count > 0 {
                format!("Пауза · {pending_count} новых")
            } else {
                "Пауза".to_owned()
            },
        )
    } else {
        match stream_state() {
            LogStreamState::Connecting => (
                "border border-amber-400/20 bg-amber-400/10 text-amber-100",
                "Подключаемся".to_owned(),
            ),
            LogStreamState::Live => (
                "border border-emerald-400/20 bg-emerald-400/10 text-emerald-200",
                "Realtime".to_owned(),
            ),
            LogStreamState::Error(message) => (
                "border border-red-400/20 bg-red-400/10 text-red-100",
                message,
            ),
        }
    };

    rsx! {
        section { class: "host-logs-page host-settings-scroll min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6",
            div { class: "mx-auto w-full max-w-[1180px] pb-10",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
                    div {
                        p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
                        h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Логи бэкенда" }
                        p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500",
                            "Последние события текущего процесса CheenHub появляются здесь в реальном времени."
                        }
                    }
                    span {
                        class: "inline-flex min-h-8 max-w-full items-center gap-2 rounded-full px-3 text-[11px] font-semibold transition-[background-color,color,box-shadow,transform] duration-200 {status_class}",
                        span {
                            class: if paused() {
                                "host-log-status-dot-paused size-1.5 shrink-0 rounded-full bg-amber-300"
                            } else if matches!(stream_state(), LogStreamState::Live) {
                                "host-log-status-dot-live size-1.5 shrink-0 rounded-full bg-emerald-300"
                            } else if matches!(stream_state(), LogStreamState::Connecting) {
                                "size-1.5 shrink-0 animate-pulse rounded-full bg-amber-300"
                            } else {
                                "size-1.5 shrink-0 rounded-full bg-red-300"
                            }
                        }
                        span { class: "truncate", "{status_label}" }
                    }
                }

                {host_settings_tabs(HostSettingsTab::Logs)}

                div { class: "host-log-metrics mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6",
                    {log_metric_card("Всего", total_count, "text-white")}
                    {log_metric_card("ERROR", error_count, "text-red-300")}
                    {log_metric_card("WARN", warn_count, "text-amber-300")}
                    {log_metric_card("INFO", info_count, "text-emerald-300")}
                    {log_metric_card("DEBUG", debug_count, "text-blue-300")}
                    {log_metric_card("TRACE", trace_count, "text-violet-300")}
                }

                div { class: "host-log-toolbar mt-4 flex flex-col gap-3 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-4 shadow-[0_18px_60px_rgba(0,0,0,.18)] sm:flex-row sm:items-center sm:p-5",
                    label { class: "min-w-0 flex-1",
                        span { class: "sr-only", "Поиск по логам" }
                        input {
                            r#type: "search",
                            placeholder: "Поиск по сообщению, target или полям",
                            value: search(),
                            oninput: move |event| search.set(event.value()),
                            class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-100 outline-none transition placeholder:text-zinc-600 focus:border-accent/70 focus:ring-4 focus:ring-accent/10"
                        }
                    }
                    label { class: "sm:w-40",
                        span { class: "sr-only", "Уровень логов" }
                        select {
                            value: level_filter(),
                            onchange: move |event| level_filter.set(event.value()),
                            class: "h-11 w-full rounded-xl border border-zinc-800 bg-zinc-950 px-3 text-[13px] text-zinc-200 outline-none focus:border-accent/70 focus:ring-4 focus:ring-accent/10",
                            option { value: "ALL", "Все уровни" }
                            option { value: "ERROR", "ERROR" }
                            option { value: "WARN", "WARN" }
                            option { value: "INFO", "INFO" }
                            option { value: "DEBUG", "DEBUG" }
                            option { value: "TRACE", "TRACE" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: if paused() {
                            "inline-flex h-11 items-center justify-center rounded-xl bg-amber-400/15 px-4 text-[13px] font-semibold text-amber-200 shadow-[0_0_0_1px_rgba(251,191,36,0.2)] transition-[background-color,color,box-shadow,transform] duration-200 hover:-translate-y-px hover:bg-amber-400/20 active:translate-y-0 active:scale-[0.97]"
                        } else {
                            "inline-flex h-11 items-center justify-center rounded-xl bg-zinc-800 px-4 text-[13px] font-semibold text-zinc-300 transition-[background-color,color,transform] duration-200 hover:-translate-y-px hover:bg-zinc-700 hover:text-white active:translate-y-0 active:scale-[0.97]"
                        },
                        onclick: move |_| {
                            if paused() {
                                if let Some(snapshot) = paused_snapshot() {
                                    entries.set(snapshot);
                                }

                                let pending = paused_entries();
                                if !pending.is_empty() {
                                    entries.with_mut(|items| {
                                        for entry in pending {
                                            push_entry(items, entry);
                                        }
                                    });
                                }

                                paused_snapshot.set(None);
                                paused_entries.set(Vec::new());
                                paused.set(false);
                            } else {
                                paused.set(true);
                            }
                        },
                        if paused() {
                            span { "Продолжить" }
                            if pending_count > 0 {
                                span {
                                    key: "{pending_count}",
                                    class: "host-log-pending-badge ml-2 inline-flex min-w-5 items-center justify-center rounded-full bg-amber-300/15 px-1.5 py-0.5 text-[10px] tabular-nums text-amber-100",
                                    "{pending_count}"
                                }
                            }
                        } else {
                            span { "Пауза" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "inline-flex h-11 items-center justify-center rounded-xl bg-zinc-800 px-4 text-[13px] font-semibold text-zinc-300 transition-[background-color,color,transform] duration-200 hover:-translate-y-px hover:bg-zinc-700 hover:text-white active:translate-y-0 active:scale-[0.97]",
                        onclick: move |_| {
                            entries.set(Vec::new());
                            paused_entries.set(Vec::new());
                            paused_snapshot.set(None);
                        },
                        "Очистить экран"
                    }
                }

                div { class: "mt-3 flex flex-wrap items-center justify-between gap-2 px-1 text-[11px] text-zinc-500",
                    span { "{all_entries.len()} записей в памяти клиента" }
                    span {
                        if paused() {
                            if pending_count > 0 {
                                "Поток на паузе · ожидают {pending_count} новых записей"
                            } else {
                                "Поток на паузе"
                            }
                        } else {
                            "Новые события отображаются сверху"
                        }
                    }
                }

                section {
                    class: "host-log-console mt-3 min-h-80 overflow-hidden rounded-[20px] border border-zinc-800 bg-zinc-950/70 shadow-[0_18px_60px_rgba(0,0,0,.18)]",
                    "aria-label": "Журнал бэкенда",

                    if visible_entries.is_empty() {
                        div { class: "host-log-empty-state grid min-h-80 place-items-center px-6 text-center",
                            div {
                                p { class: "text-sm font-semibold text-zinc-300",
                                    if all_entries.is_empty() {
                                        "Ожидаем события"
                                    } else {
                                        "Ничего не найдено"
                                    }
                                }
                                p { class: "mt-1 text-[12px] leading-5 text-zinc-600",
                                    if all_entries.is_empty() {
                                        "Новые записи появятся здесь автоматически."
                                    } else {
                                        "Измени уровень или поисковый запрос."
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "divide-y divide-zinc-900",
                            for entry in visible_entries {
                                div {
                                    key: "{entry.id}",
                                    class: if latest_entry_id == Some(entry.id) {
                                        "host-log-entry-new grid gap-1 px-4 py-3 font-mono text-[12px] leading-5 transition-colors duration-150 hover:bg-zinc-900/45 lg:grid-cols-[105px_64px_180px_minmax(0,1fr)] lg:gap-3"
                                    } else {
                                        "grid gap-1 px-4 py-3 font-mono text-[12px] leading-5 transition-colors duration-150 hover:bg-zinc-900/45 lg:grid-cols-[105px_64px_180px_minmax(0,1fr)] lg:gap-3"
                                    },
                                    span { class: "tabular-nums text-zinc-600", "{short_timestamp(&entry.timestamp)}" }
                                    span { class: "font-semibold {level_class(&entry.level)}", "{entry.level}" }
                                    span {
                                        class: "truncate text-zinc-500",
                                        title: "{entry.target}",
                                        "{target_label(&entry.target)}"
                                    }
                                    div { class: "min-w-0",
                                        p { class: "break-words text-zinc-200", "{entry.message}" }
                                        if !entry.fields.is_empty() {
                                            p { class: "mt-0.5 break-words text-zinc-600", "{entry.fields.join(\"  \")}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if all_entries.len() > MAX_RENDERED_ENTRIES {
                    p { class: "mt-3 px-1 text-[11px] text-zinc-600",
                        "Для производительности одновременно отображается не больше {MAX_RENDERED_ENTRIES} записей."
                    }
                }
            }
        }
    }
}

fn push_entry(items: &mut Vec<HostLogEntry>, entry: HostLogEntry) {
    if items
        .last()
        .map(|last| last.id >= entry.id)
        .unwrap_or(false)
    {
        return;
    }

    items.push(entry);

    let overflow = items.len().saturating_sub(MAX_CLIENT_ENTRIES);
    if overflow > 0 {
        items.drain(0..overflow);
    }
}

fn count_level(entries: &[HostLogEntry], level: &str) -> usize {
    entries.iter().filter(|entry| entry.level == level).count()
}

fn log_metric_card(label: &'static str, value: usize, value_class: &'static str) -> Element {
    rsx! {
        div { class: "host-log-metric-card rounded-[18px] border border-zinc-800 bg-zinc-950/70 p-4 transition-[transform,background-color,box-shadow] duration-200 hover:-translate-y-0.5 hover:bg-zinc-900",
            p { class: "text-[10px] font-semibold uppercase tracking-[0.16em] text-zinc-500",
                "{label}"
            }
            strong {
                key: "{label}-{value}",
                class: "host-log-metric-value mt-1 block tabular-nums text-2xl font-semibold tracking-[-0.04em] {value_class}",
                "{value}"
            }
            p { class: "mt-1 text-[10px] text-zinc-600", "в текущем буфере" }
        }
    }
}

fn entry_matches(entry: &HostLogEntry, query: &str) -> bool {
    entry.message.to_ascii_lowercase().contains(query)
        || entry.target.to_ascii_lowercase().contains(query)
        || entry
            .fields
            .iter()
            .any(|field| field.to_ascii_lowercase().contains(query))
}

fn short_timestamp(timestamp: &str) -> String {
    timestamp
        .split_once('T')
        .map(|(_, time)| time.trim_end_matches('Z').to_owned())
        .unwrap_or_else(|| timestamp.to_owned())
}

fn target_label(target: &str) -> &str {
    target.rsplit("::").next().unwrap_or(target)
}

fn level_class(level: &str) -> &'static str {
    match level {
        "ERROR" => "text-red-300",
        "WARN" => "text-amber-300",
        "INFO" => "text-emerald-300",
        "DEBUG" => "text-blue-300",
        "TRACE" => "text-violet-300",
        _ => "text-zinc-300",
    }
}
