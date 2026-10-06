//! Панель активности голосового чата на дашборде владельца хоста.
//!
//! Блок работает независимо от системных метрик: недоступность CPU, памяти или
//! metrics proxy не влияет на текущие счётчики и историю голосовых подключений.

use cheenhub_contracts::rest::{HostVoiceActivityResponse, HostVoiceActivitySample};
use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

use super::activity_chart;
use super::api;

/// Интервал обновления текущих счётчиков активности.
const COUNTER_REFRESH_INTERVAL_MS: u32 = 2_000;
/// Интервал догрузки новых точек истории активности.
const HISTORY_REFRESH_INTERVAL_MS: u32 = 10_000;

/// Состояние текущих счётчиков активности.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CounterState {
    Loading,
    Failed,
    Ready(HostVoiceActivityResponse),
}

/// Состояние истории активности: недоступность хранилища не должна скрывать график,
/// поэтому она выражается отдельным вариантом, а не ошибкой.
#[derive(Clone, PartialEq, Eq)]
enum HistoryState {
    Loading,
    Failed(String),
    Ready {
        available: bool,
        samples: Vec<HostVoiceActivitySample>,
    },
}

/// Рендерит карточки текущей активности и график голосовых подключений за 24 часа.
#[component]
pub(crate) fn HostActivityPanel() -> Element {
    let mut counter = use_signal(|| CounterState::Loading);
    let mut history = use_signal(|| HistoryState::Loading);
    let chart_error = use_signal(|| None::<String>);
    let chart_id = use_signal(activity_chart::new_chart_id);

    // График монтируется один раз: экземпляр ECharts переиспользуется, а новые
    // измерения приходят через обновление опции ниже.
    use_effect(move || {
        let id = chart_id.read().clone();
        let mut chart_error = chart_error;
        spawn(async move {
            let samples = history_samples(&history.read());
            if samples.len() < 2 {
                return;
            }
            if let Err(error) = activity_chart::mount_chart(&id, &samples).await {
                warn!(%error, "failed to render host voice activity chart");
                chart_error.set(Some(error));
            }
        });
    });

    use_future(move || async move {
        loop {
            sleep_ms(COUNTER_REFRESH_INTERVAL_MS).await;
            match api::load_activity().await {
                Ok(activity) => {
                    info!(
                        voice_connections = activity.voice_connections,
                        video_sources = activity.video_sources,
                        "host voice activity counters updated"
                    );
                    counter.set(CounterState::Ready(activity));
                }
                Err(error) => {
                    warn!(message = %error.message(), "failed to refresh host voice activity");
                    counter.set(CounterState::Failed);
                }
            }
        }
    });

    use_future(move || async move {
        loop {
            sleep_ms(HISTORY_REFRESH_INTERVAL_MS).await;
            // Запрос идёт от отметки последней точки, поэтому клиент не забирает
            // все измерения суток каждые 10 секунд.
            let after = match &*history.read() {
                HistoryState::Ready { samples, .. } => {
                    samples.last().map(|sample| sample.sampled_at_unix_ms)
                }
                _ => None,
            };
            match api::load_activity_history(after).await {
                Ok(response) => {
                    let merged = merge_history(&history.read(), response);
                    let samples = history_samples(&merged);
                    if !samples.is_empty()
                        && let Err(error) =
                            activity_chart::update_chart(&chart_id.read().clone(), &samples).await
                    {
                        warn!(%error, "failed to update host voice activity chart");
                    }
                    history.set(merged);
                }
                Err(error) => {
                    warn!(message = %error.message(), "failed to refresh host voice activity history");
                    history.set(HistoryState::Failed(error.message().to_owned()));
                }
            }
        }
    });

    rsx! {
        section { class: "mt-6",
            div {
                h2 { class: "text-[17px] font-semibold tracking-[-0.02em] text-zinc-50", "Активность голосового чата" }
                p { class: "mt-1 text-pretty text-[13px] leading-5 text-zinc-500", "Текущие подключения и видеоисточники, а также история голосовых подключений за сутки." }
            }
            div { class: "mt-4 grid gap-3 sm:grid-cols-2",
                {activity_card("Подключения к голосу", voice_connections_value(counter()))}
                {activity_card("Видеоисточники", video_sources_value(counter()))}
            }
            {chart_card(history(), chart_id.read().clone(), chart_error())}
        }
    }
}

/// Добавляет новые точки к уже загруженной истории и обрезает окно в 24 часа.
fn merge_history(
    current: &HistoryState,
    response: cheenhub_contracts::rest::HostVoiceActivityHistoryResponse,
) -> HistoryState {
    let samples = match current {
        HistoryState::Ready {
            available: true,
            samples: previous,
        } => {
            let mut previous = previous.clone();
            previous.extend(response.samples);
            previous
        }
        _ => response.samples,
    };
    HistoryState::Ready {
        available: response.available,
        samples: activity_chart::prune_to_window(samples),
    }
}
fn activity_card(label: &'static str, value: Element) -> Element {
    rsx! {
        div { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-4 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            p { class: "text-[12px] font-medium text-zinc-500", "{label}" }
            div { class: "mt-2 min-h-9", {value} }
        }
    }
}

fn voice_connections_value(state: CounterState) -> Element {
    match state {
        CounterState::Loading => counter_placeholder("Загружаем текущую активность..."),
        CounterState::Failed => counter_placeholder("Показатели временно недоступны"),
        CounterState::Ready(activity) => counter_value(activity.voice_connections),
    }
}

fn video_sources_value(state: CounterState) -> Element {
    match state {
        CounterState::Loading => counter_placeholder("Загружаем текущую активность..."),
        CounterState::Failed => counter_placeholder("Показатели временно недоступны"),
        CounterState::Ready(activity) => counter_value(activity.video_sources),
    }
}

fn counter_placeholder(message: &'static str) -> Element {
    rsx! {
        span { class: "flex h-9 items-center gap-2 text-[13px] text-zinc-500",
            span { class: "size-3.5 animate-pulse rounded-full bg-zinc-700" }
            "{message}"
        }
    }
}

fn counter_value(value: u32) -> Element {
    rsx! {
        span { class: "tabular-nums text-[26px] font-semibold tracking-[-0.03em] text-zinc-50", "{value}" }
    }
}

/// Отдаёт измерения истории в график; недоступное состояние не рисуется.
fn history_samples(state: &HistoryState) -> Vec<HostVoiceActivitySample> {
    match state {
        HistoryState::Ready { samples, .. } => samples.clone(),
        _ => Vec::new(),
    }
}

/// Карточка графика: контейнер для ECharts плюс состояние без данных.
fn chart_card(state: HistoryState, chart_id: String, chart_error: Option<String>) -> Element {
    if let Some(message) = chart_error {
        return chart_error_state(&message);
    }
    match state {
        HistoryState::Loading => rsx! {
            div { class: "mt-3 h-56 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
        },
        HistoryState::Failed(message) => chart_error_state(&message),
        HistoryState::Ready {
            available: false,
            ref samples,
        } if samples.is_empty() => history_unavailable(),
        HistoryState::Ready {
            available: true,
            ref samples,
        } if samples.len() < 2 => empty_history(),
        HistoryState::Ready { .. } => rsx! {
            div { class: "mt-3 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-4 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                {chart_header(state)}
                div { class: "mt-3 h-56 w-full", id: "{chart_id}" }
            }
        },
    }
}

/// Заголовок карточки с текущими значениями и пиком за сутки.
fn chart_header(state: HistoryState) -> Element {
    let samples = history_samples(&state);
    let last = samples.last();
    let peak =
        |value: fn(&HostVoiceActivitySample) -> u32| samples.iter().map(value).max().unwrap_or(0);
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3",
            div {
                h3 { class: "text-[13px] font-semibold text-zinc-200", "Активность за 24 часа" }
                p { class: "mt-0.5 text-[12px] text-zinc-500", "Измерение каждые 10 секунд, разрывы означают время простоя сервера." }
            }
            div { class: "flex flex-wrap items-center gap-x-5 gap-y-1",
                {series_badge("Подключения", last.map(|sample| sample.voice_connections), peak(|sample| sample.voice_connections), "#60a5fa", false)}
                {series_badge("Видеоисточники", last.map(|sample| sample.video_sources), peak(|sample| sample.video_sources), "#c084fc", true)}
            }
        }
    }
}

/// Подпись серии с текущим и пиковым значением за сутки.
fn series_badge(
    label: &'static str,
    current: Option<u32>,
    peak: u32,
    color: &'static str,
    dashed: bool,
) -> Element {
    let marker_class = if dashed {
        "h-0.5 w-4 border-t-2 border-dashed border-t-violet-300"
    } else {
        "size-2.5 rounded-full"
    };
    rsx! {
        div { class: "flex items-center gap-2",
            span { class: "{marker_class}", style: if dashed { "border-top-color:{color};" } else { "background:{color};" } }
            span { class: "text-[11px] text-zinc-500", "{label}" }
            strong { class: "tabular-nums text-[13px] font-semibold text-zinc-100", "{current.unwrap_or(0)}" }
            span { class: "text-[11px] text-zinc-600", "пик {peak}" }
        }
    }
}

/// Состояние, когда график построить не удалось, но числа остаются актуальными.
fn chart_error_state(message: &str) -> Element {
    rsx! {
        div { class: "mt-3 rounded-[20px] border border-amber-400/20 bg-amber-400/10 p-5",
            h3 { class: "text-[15px] font-semibold text-amber-100", "График временно недоступен" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-amber-100/80", "{message}" }
            p { class: "mt-1.5 text-pretty text-[12px] leading-5 text-zinc-400", "Текущие показатели активности выше продолжают обновляться." }
        }
    }
}

fn history_unavailable() -> Element {
    rsx! {
        div { class: "mt-3 rounded-[20px] border border-amber-400/20 bg-amber-400/10 p-5",
            h3 { class: "text-[15px] font-semibold text-amber-100", "История пока недоступна" }
            p { class: "mt-1.5 text-pretty text-[13px] leading-5 text-amber-100/80", "Сервер не смог прочитать накопленные измерения. Новые данные появятся после восстановления хранилища." }
        }
    }
}

fn empty_history() -> Element {
    rsx! {
        div { class: "mt-3 rounded-[20px] border border-zinc-800 bg-zinc-950/60 p-6 text-center",
            h3 { class: "text-[15px] font-semibold text-zinc-200", "История ещё не накопилась" }
            p { class: "mx-auto mt-1.5 max-w-md text-pretty text-[13px] leading-5 text-zinc-500", "Первое измерение появляется сразу после запуска сервера, дальше график дополняется каждые 10 секунд." }
        }
    }
}
