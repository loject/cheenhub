//! Панель сводной статистики CheenHub на дашборде владельца хоста.
//!
//! Блок показывает размер установки (пользователи, серверы, комнаты, сообщения)
//! и график сообщений в минуту за сутки. Он загружается независимо от системных
//! метрик, поэтому недоступность одного источника не скрывает остальные блоки.

use cheenhub_contracts::rest::{HostMessagesPerMinuteSample, HostStatsResponse};
use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

use super::api;
use super::messages_chart;

/// Интервал обновления статистики.
///
/// Счётчики меняются медленно, а график агрегирует значения по минутам,
/// поэтому обновлять его чаще, чем раз в полминуты, нет смысла.
const REFRESH_INTERVAL_MS: u32 = 30_000;

/// Рендерит карточки размера установки и график сообщений за сутки.
#[component]
pub(crate) fn HostStatsPanel() -> Element {
    let mut stats = use_resource(api::load_stats);
    let chart_error = use_signal(|| None::<String>);
    let chart_id = use_signal(messages_chart::new_chart_id);

    let retry = Callback::new(move |()| stats.restart());

    // График монтируется один раз: новые данные приходят обновлением опции.
    // Значение читается синхронно, поэтому эффект срабатывает и на первом
    // ответе, и на каждом следующем обновлении статистики.
    use_effect(move || {
        let id = chart_id.read().clone();
        let data = chart_data(&stats.read());
        if data.is_empty() {
            return;
        }
        let mut chart_error = chart_error;
        spawn(async move {
            if let Err(error) = messages_chart::mount_chart(&id, &data).await {
                warn!(%error, "failed to render host messages chart");
                chart_error.set(Some(error));
            } else {
                debug!(chart_id = %id, "host messages chart rendered");
            }
        });
    });

    use_future(move || async move {
        loop {
            sleep_ms(REFRESH_INTERVAL_MS).await;
            let id = chart_id.read().clone();
            let response = api::load_stats().await;
            let data = match &response {
                Ok(response) => messages_chart::prune_to_window(chart_data_from(response)),
                Err(error) => {
                    warn!(message = %error.message(), "failed to refresh host statistics");
                    messages_chart::MessagesChartData {
                        window_end_unix_ms: 0,
                        room_samples: Vec::new(),
                        direct_samples: Vec::new(),
                    }
                }
            };
            if !data.is_empty()
                && let Err(error) = messages_chart::update_chart(&id, &data).await
            {
                warn!(%error, "failed to update host messages chart");
            }
            match response {
                Ok(response) => {
                    info!(
                        users_total = response.users_total,
                        servers_total = response.servers_total,
                        rooms_total = response.rooms_total,
                        room_messages_total = response.room_messages_total,
                        direct_messages_total = response.direct_messages_total,
                        "host statistics refreshed"
                    );
                    stats.set(Some(Ok(response)));
                }
                Err(error) => stats.set(Some(Err(error))),
            }
        }
    });

    let result = stats.read().clone();

    rsx! {
        section { class: "mt-6",
            div {
                h2 { class: "text-[17px] font-semibold tracking-[-0.02em] text-zinc-50", "Статистика CheenHub" }
                p { class: "mt-1 text-pretty text-[13px] leading-5 text-zinc-500", "Размер установки и сообщения в комнатах за последние сутки." }
            }
            {stats_body(result, chart_id.read().clone(), chart_error(), retry)}
        }
    }
}

/// Показывает карточки счётчиков и график сообщений для текущего состояния.
fn stats_body(
    result: Option<Result<HostStatsResponse, api::HostSettingsApiError>>,
    chart_id: String,
    chart_error: Option<String>,
    retry: Callback<()>,
) -> Element {
    match result {
        None => rsx! {
            div { class: "mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
                div { class: "h-24 animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70" }
                div { class: "h-24 animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70" }
                div { class: "h-24 animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70" }
                div { class: "h-24 animate-pulse rounded-[18px] border border-zinc-800 bg-zinc-950/70" }
            }
            div { class: "mt-3 h-72 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
        },
        Some(Err(error)) => rsx! {
            div { class: "mt-4 rounded-[20px] border border-red-500/20 bg-red-500/10 p-5",
                h3 { class: "text-[15px] font-semibold text-red-100", "Не удалось загрузить статистику" }
                p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-red-100/80", "{error.message()}" }
                p { class: "mt-1.5 text-pretty text-[12px] leading-5 text-zinc-400", "Системные метрики и активность голосового чата обновляются независимо." }
                button {
                    r#type: "button",
                    class: "mt-4 inline-flex min-h-11 items-center justify-center rounded-xl bg-red-400 px-4 text-[13px] font-semibold text-zinc-950 transition-[background-color,scale] duration-150 hover:bg-red-300 active:scale-[0.96]",
                    onclick: move |_| retry(()),
                    "Повторить"
                }
            }
        },
        Some(Ok(stats)) => rsx! {
            {counters(&stats)}
            {chart_card(&stats, chart_id, chart_error)}
        },
    }
}

/// Карточки ключевых счётчиков установки.
fn counters(stats: &HostStatsResponse) -> Element {
    let messages_total = stats.room_messages_total + stats.direct_messages_total;
    rsx! {
        div { class: "mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
            {counter_card("Пользователей", stats.users_total, "зарегистрировано на хосте", "text-sky-300")}
            {counter_card("Серверов", stats.servers_total, "создано на хосте", "text-violet-300")}
            {counter_card("Комнат", stats.rooms_total, "во всех серверах", "text-amber-300")}
            {counter_card(
                "Сообщений",
                messages_total,
                direct_hint(stats.direct_messages_total),
                "text-emerald-300",
            )}
        }
    }
}

/// Подсказка карточки сообщений: сколько из них пришло в личные диалоги.
fn direct_hint(direct_messages_total: u64) -> String {
    format!("из них {direct_messages_total} личных")
}

/// Карточка одного счётчика с подсказкой о том, что именно считается.
fn counter_card(
    label: &'static str,
    value: u64,
    hint: impl Into<String>,
    color: &'static str,
) -> Element {
    let hint = hint.into();
    rsx! {
        div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 px-4 py-4 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            p { class: "text-[12px] font-medium text-zinc-500", "{label}" }
            strong { class: "mt-1 block tabular-nums text-2xl font-semibold tracking-[-0.03em] {color}", "{format_count(value)}" }
            p { class: "mt-1 text-[11px] text-zinc-600", "{hint}" }
        }
    }
}

/// Карточка графика сообщений с состояниями «нет данных» и «график недоступен».
fn chart_card(stats: &HostStatsResponse, chart_id: String, chart_error: Option<String>) -> Element {
    if let Some(message) = chart_error {
        return rsx! {
            div { class: "mt-3 rounded-[20px] border border-amber-400/20 bg-amber-400/10 p-5",
                h3 { class: "text-[15px] font-semibold text-amber-100", "График сообщений временно недоступен" }
                p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-amber-100/80", "{message}" }
                p { class: "mt-1.5 text-pretty text-[12px] leading-5 text-zinc-400", "Счётчики выше продолжают обновляться." }
            }
        };
    }

    let data = chart_data_from(stats);
    if data.is_empty() {
        return rsx! {
            div { class: "mt-3 rounded-[20px] border border-zinc-800 bg-zinc-950/60 p-6 text-center",
                h3 { class: "text-[15px] font-semibold text-zinc-200", "За сутки сообщений ещё не было" }
                p { class: "mx-auto mt-1.5 max-w-md text-pretty text-[13px] leading-5 text-zinc-500", "График наполнится, как только в комнатах или личных диалогах появятся сообщения." }
            }
        };
    }

    rsx! {
        div { class: "mt-3 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-4 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            div { class: "flex flex-wrap items-center justify-between gap-3",
                div {
                    h3 { class: "text-[13px] font-semibold text-zinc-200", "Сообщений в минуту за 24 часа" }
                    p { class: "mt-0.5 text-[12px] text-zinc-500", "Минуты без сообщений на шкале не отмечаются." }
                }
                div { class: "flex flex-wrap items-center gap-x-5 gap-y-1",
                    {series_badge("В комнатах", "bg-emerald-300", &data.room_samples)}
                    {series_badge("Личные", "bg-sky-300", &data.direct_samples)}
                }
            }
            div { class: "mt-3 h-56 w-full", id: "{chart_id}" }
        }
    }
}

/// Подпись серии с текущим и пиковым значением за сутки.
fn series_badge(
    label: &'static str,
    dot_class: &'static str,
    samples: &[HostMessagesPerMinuteSample],
) -> Element {
    rsx! {
        div { class: "flex items-center gap-2",
            span { class: "size-2.5 rounded-full {dot_class}" }
            span { class: "text-[11px] text-zinc-500", "{label}" }
            strong { class: "tabular-nums text-[13px] font-semibold text-zinc-100", "{last_minute_value(samples)}" }
            span { class: "text-[11px] text-zinc-600", "пик {peak_minute_value(samples)}" }
        }
    }
}

/// Собирает данные графика из ответа статистики.
///
/// Минуты без сообщений сервер не присылает, поэтому каждая серия содержит
/// только минуты с реальными сообщениями.
fn chart_data_from(stats: &HostStatsResponse) -> messages_chart::MessagesChartData {
    messages_chart::MessagesChartData {
        window_end_unix_ms: stats.messages_window_end_unix_ms,
        room_samples: stats.room_messages_per_minute.clone(),
        direct_samples: stats.direct_messages_per_minute.clone(),
    }
}

/// Отдаёт данные графика для текущего состояния ресурса; загрузка не рисуется.
fn chart_data(
    state: &Option<Result<HostStatsResponse, api::HostSettingsApiError>>,
) -> messages_chart::MessagesChartData {
    match state {
        Some(Ok(stats)) => chart_data_from(stats),
        _ => messages_chart::MessagesChartData {
            window_end_unix_ms: 0,
            room_samples: Vec::new(),
            direct_samples: Vec::new(),
        },
    }
}

/// Сообщения в последней минуте с данными.
fn last_minute_value(samples: &[HostMessagesPerMinuteSample]) -> u64 {
    samples.last().map(|sample| sample.messages).unwrap_or(0)
}

/// Максимальное число сообщений в минуту за сутки.
fn peak_minute_value(samples: &[HostMessagesPerMinuteSample]) -> u64 {
    samples
        .iter()
        .map(|sample| sample.messages)
        .max()
        .unwrap_or(0)
}

/// Форматирует большие счётчики с разделителем разрядов.
///
/// Числа короче пяти знаков показываются полностью: на небольшой установке
/// точное значение без разделителей читается быстрее.
pub(super) fn format_count(value: u64) -> String {
    let digits = value.to_string();
    if digits.len() <= 4 {
        return digits;
    }

    digits
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
