//! Дашборд нагрузки хоста для владельца установки.
//!
//! Страница загружается всегда: системные метрики, история нагрузки и активность
//! голосового чата запрашиваются независимо, поэтому недоступность одного
//! источника оставляет рабочими остальные блоки.

use cheenhub_contracts::rest::HostMetricsResponse;
use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

use super::activity_panel::HostActivityPanel;
use super::api::{self, HostSettingsApiError};
use super::tabs::{HostSettingsTab, host_settings_tabs};

mod charts;
mod sections;
mod states;

/// Интервал обновления системных метрик хоста.
const REFRESH_INTERVAL_MS: u32 = 2_000;

/// Выбранный вид процессора в секции нагрузки.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum CpuView {
    /// Сводная загрузка и разбивка по процессам.
    #[default]
    Total,
    /// Отдельный график по каждому логическому ядру.
    Logical,
}

/// Рендерит оперативный дашборд нагрузки хоста.
#[component]
pub(crate) fn HostDashboardPage() -> Element {
    let cpu_view = use_signal(CpuView::default);
    let mut metrics_resource = use_resource(api::load_metrics);
    let metrics_result = metrics_resource.read().clone();

    let retry_metrics = Callback::new(move |()| metrics_resource.restart());

    use_future(move || async move {
        loop {
            sleep_ms(REFRESH_INTERVAL_MS).await;
            metrics_resource.restart();
        }
    });

    rsx! {
        section { class: "host-settings-scroll min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6",
            div { class: "mx-auto w-full max-w-[1180px] pb-10",
                {header_block(metrics_result.as_ref().and_then(|result| result.as_ref().ok()))}
                {host_settings_tabs(HostSettingsTab::Dashboard)}

                {HostActivityPanel()}

                {metrics_body(metrics_result.clone(), cpu_view, retry_metrics)}
            }
        }
    }
}

/// Шапка страницы со статусом источника системных метрик.
fn header_block(metrics: Option<&HostMetricsResponse>) -> Element {
    rsx! {
        div { class: "flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
            div {
                p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
                h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Нагрузка на систему" }
                p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500", "Следи за ресурсами CheenHub, базы данных и остальной системы в реальном времени." }
            }
            match metrics {
                Some(metrics) => rsx! {
                    span {
                        class: if metrics.available { "inline-flex min-h-8 w-fit items-center gap-2 rounded-xl border border-emerald-400/20 bg-emerald-400/10 px-3 text-[11px] font-medium text-emerald-200" } else { "inline-flex min-h-8 w-fit items-center gap-2 rounded-xl border border-amber-400/20 bg-amber-400/10 px-3 text-[11px] font-medium text-amber-100" },
                        span { class: if metrics.available { "size-1.5 rounded-full bg-emerald-300" } else { "size-1.5 rounded-full bg-amber-300" } }
                        if metrics.available { "Обновляется" } else { "Нет свежих данных" }
                    }
                },
                None => rsx! {
                    span { class: "inline-flex min-h-8 w-fit items-center gap-2 rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 text-[11px] font-medium text-zinc-400",
                        span { class: "size-1.5 rounded-full bg-zinc-600" }
                        "Подключение"
                    }
                },
            }
        }
    }
}

/// Показывает системные метрики, деградируя по секциям при отсутствии данных.
fn metrics_body(
    metrics_result: Option<Result<HostMetricsResponse, HostSettingsApiError>>,
    cpu_view: Signal<CpuView>,
    restart: Callback<()>,
) -> Element {
    let Some(metrics_result) = metrics_result else {
        return states::metrics_loader();
    };

    match metrics_result {
        Ok(metrics) => {
            let samples = metrics.samples;
            // Ответ без измерений или с устаревшими данными не скрывает секции:
            // они показывают собственное состояние отсутствия данных.
            let show_stale_notice = !metrics.available && !samples.is_empty();
            rsx! {
                if show_stale_notice {
                    {states::stale_metrics_notice()}
                }
                {sections::metrics_sections(&samples, cpu_view)}
            }
        }
        Err(error) => rsx! {
            {states::metrics_error(error.message().to_owned(), restart)}
            {sections::metrics_sections(&[], cpu_view)}
        },
    }
}

/// Карточка сводного показателя системы.
pub(super) fn summary_card(
    label: &'static str,
    value: String,
    hint: impl Into<String>,
    color: &'static str,
) -> Element {
    let hint = hint.into();

    rsx! {
        div { class: "rounded-[18px] border border-zinc-800 bg-zinc-950/70 px-4 py-4",
            p { class: "text-[10px] font-semibold uppercase tracking-[0.14em] text-zinc-500",
                "{label}"
            }
            p { class: "mt-1 tabular-nums text-xl font-semibold tracking-[-0.03em] {color}",
                "{value}"
            }
            p { class: "mt-1 text-[11px] text-zinc-600",
                "{hint}"
            }
        }
    }
}
