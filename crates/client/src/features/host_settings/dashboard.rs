//! Дашборд нагрузки хоста для владельца установки.

use cheenhub_contracts::rest::{HostMetricsResponse, HostMetricsSample};
use dioxus::prelude::*;

use crate::features::runtime::sleep_ms;

use super::api;
use super::tabs::{HostSettingsTab, host_settings_tabs};

const REFRESH_INTERVAL_MS: u32 = 2_000;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum CpuView {
    #[default]
    Total,
    Logical,
}

/// Рендерит оперативный дашборд нагрузки хоста.
#[component]
pub(crate) fn HostDashboardPage() -> Element {
    let cpu_view = use_signal(CpuView::default);
    let mut metrics_resource = use_resource(api::load_metrics);
    let metrics_result = metrics_resource.read().clone();

    use_future(move || async move {
        loop {
            sleep_ms(REFRESH_INTERVAL_MS).await;
            metrics_resource.restart();
        }
    });

    rsx! {
        section { class: "host-settings-scroll min-w-0 flex-1 overflow-y-auto bg-zinc-950/35 px-4 py-6 sm:px-6",
            div { class: "mx-auto w-full max-w-[1180px] pb-10",
                div { class: "flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between",
                    div {
                        p { class: "text-[11px] font-medium uppercase tracking-[0.20em] text-zinc-600", "Настройки хоста" }
                        h1 { class: "mt-1 text-balance text-[22px] font-semibold tracking-[-0.04em] text-zinc-50", "Нагрузка на систему" }
                        p { class: "mt-1.5 max-w-2xl text-pretty text-[13px] leading-5 text-zinc-500", "Следи за ресурсами CheenHub, базы данных и остальной системы в реальном времени." }
                    }
                    if let Some(Ok(metrics)) = &metrics_result {
                        span {
                            class: if metrics.available { "inline-flex min-h-8 w-fit items-center gap-2 rounded-xl border border-emerald-400/20 bg-emerald-400/10 px-3 text-[11px] font-medium text-emerald-200" } else { "inline-flex min-h-8 w-fit items-center gap-2 rounded-xl border border-amber-400/20 bg-amber-400/10 px-3 text-[11px] font-medium text-amber-100" },
                            span { class: if metrics.available { "size-1.5 rounded-full bg-emerald-300" } else { "size-1.5 rounded-full bg-amber-300" } }
                            if metrics.available { "Обновляется" } else { "Нет свежих данных" }
                        }
                    }
                }
                {host_settings_tabs(HostSettingsTab::Dashboard)}

                match metrics_result {
                    None => dashboard_loader(),
                    Some(Err(error)) => rsx! {
                        div { class: "mt-6 rounded-[20px] border border-red-500/20 bg-red-500/10 p-5",
                            h2 { class: "text-lg font-semibold text-red-100", "Не удалось загрузить показатели" }
                            p { class: "mt-2 text-pretty text-sm leading-6 text-red-100/75", "{error.message()}" }
                            button {
                                r#type: "button",
                                class: "mt-4 inline-flex min-h-11 items-center justify-center rounded-xl bg-red-400 px-4 text-[13px] font-semibold text-zinc-950 transition-[background-color,scale] duration-150 hover:bg-red-300 active:scale-[0.96]",
                                onclick: move |_| metrics_resource.restart(),
                                "Повторить"
                            }
                        }
                    },
                    Some(Ok(metrics)) if metrics.samples.is_empty() => unavailable_state(),
                    Some(Ok(metrics)) => dashboard_content(metrics, cpu_view),
                }
            }
        }
    }
}

fn dashboard_loader() -> Element {
    rsx! {
        div { class: "mt-6 grid gap-5",
            div { class: "h-72 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/70" }
            div { class: "grid gap-5 lg:grid-cols-2",
                div { class: "h-64 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
                div { class: "h-64 animate-pulse rounded-[20px] border border-zinc-800 bg-zinc-950/60" }
            }
        }
    }
}

fn unavailable_state() -> Element {
    rsx! {
        div { class: "mt-6 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-6 text-center shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            div { class: "mx-auto flex size-12 items-center justify-center rounded-2xl bg-amber-400/10 text-amber-200",
                svg { class: "size-6", fill: "none", stroke: "currentColor", stroke_width: "1.8", view_box: "0 0 24 24", "aria-hidden": "true",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 13h4l2-7 4 12 2-5h4M5 21h14a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2Z" }
                }
            }
            h2 { class: "mt-4 text-balance text-xl font-semibold text-white", "Ожидаем данные о нагрузке" }
            p { class: "mx-auto mt-2 max-w-lg text-pretty text-sm leading-6 text-zinc-400", "Сборщик подключается к сервисам. Показатели появятся здесь автоматически." }
        }
    }
}

fn dashboard_content(metrics: HostMetricsResponse, mut cpu_view: Signal<CpuView>) -> Element {
    let samples = metrics.samples;
    let latest = samples
        .last()
        .expect("non-empty metrics history was checked");
    let total_button = cpu_toggle_class(cpu_view() == CpuView::Total);
    let logical_button = cpu_toggle_class(cpu_view() == CpuView::Logical);
    let memory_percent = if latest.memory.total_bytes == 0 {
        0.0
    } else {
        (latest.memory.used_bytes as f64 / latest.memory.total_bytes as f64 * 100.0) as f32
    };

    rsx! {
        if !metrics.available {
            div { class: "mt-6 rounded-2xl bg-amber-400/10 px-4 py-3 text-[13px] text-amber-100 shadow-[0_0_0_1px_rgba(251,191,36,0.2)]", role: "status",
                "Показываем последние доступные данные. Новые измерения временно не поступают."
            }
        }
        div { class: "mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
            {summary_card(
                "Процессор",
                format_percent(latest.cpu.system_percent),
                "общая нагрузка",
                "text-white",
            )}
            {summary_card(
                "Оперативная память",
                format_percent(memory_percent),
                format!(
                    "{} из {}",
                    format_bytes(latest.memory.used_bytes),
                    format_bytes(latest.memory.total_bytes),
                ),
                "text-emerald-300",
            )}
            {summary_card(
                "Получение",
                format_rate(latest.network.received_bytes_per_second),
                "сеть CheenHub",
                "text-blue-300",
            )}
            {summary_card(
                "Отправка",
                format_rate(latest.network.sent_bytes_per_second),
                "сеть CheenHub",
                "text-violet-300",
            )}
        }

        section { class: "mt-4 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            div { class: "flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between",
                div {
                    p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Процессор" }
                    div { class: "mt-1 flex items-baseline gap-2",
                        strong { class: "tabular-nums text-3xl font-semibold tracking-[-0.04em] text-white", "{format_percent(latest.cpu.system_percent)}" }
                        span { class: "text-sm text-zinc-500", "общая нагрузка" }
                    }
                }
                div { class: "grid grid-cols-2 rounded-2xl bg-zinc-950/80 p-1.5 shadow-[0_0_0_1px_rgba(255,255,255,0.06)]",
                    button { r#type: "button", class: "{total_button}", onclick: move |_| cpu_view.set(CpuView::Total), "Система" }
                    button { r#type: "button", class: "{logical_button}", onclick: move |_| cpu_view.set(CpuView::Logical), "По ядрам" }
                }
            }
            if cpu_view() == CpuView::Total {
                {cpu_total_chart(&samples)}
                {cpu_breakdown(latest)}
            } else {
                {logical_cpu_grid(&samples)}
            }
        }

        div { class: "mt-5 grid gap-5 lg:grid-cols-2",
            section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Оперативная память" }
                div { class: "mt-1 flex items-baseline gap-2",
                    strong { class: "tabular-nums text-2xl font-semibold tracking-[-0.04em] text-white", "{format_bytes(latest.memory.used_bytes)}" }
                    span { class: "text-sm text-zinc-500", "из {format_bytes(latest.memory.total_bytes)}" }
                }
                {memory_chart(&samples)}
                {memory_breakdown(latest)}
            }
            section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Сеть CheenHub" }
                div { class: "mt-2 grid grid-cols-2 gap-3",
                    {metric_value("Получение", latest.network.received_bytes_per_second, "text-blue-300")}
                    {metric_value("Отправка", latest.network.sent_bytes_per_second, "text-violet-300")}
                }
                {network_chart(&samples)}
            }
        }
    }
}

fn cpu_total_chart(samples: &[HostMetricsSample]) -> Element {
    let system = points(samples, 100.0, |sample| {
        f64::from(sample.cpu.system_percent)
    });
    let cheenhub = points(samples, 100.0, |sample| {
        f64::from(sample.cpu.cheenhub_percent)
    });
    let database = points(samples, 100.0, |sample| {
        f64::from(sample.cpu.database_percent)
    });
    rsx! {
        div { class: "mt-4 h-36 rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            svg { class: "h-full w-full", view_box: "0 0 100 40", preserve_aspect_ratio: "none", role: "img", "aria-label": "История нагрузки процессора",
                line { x1: "0", y1: "20", x2: "100", y2: "20", stroke: "rgba(255,255,255,0.06)", stroke_width: "0.35" }
                polyline { points: "{system}", fill: "none", stroke: "#e4e4e7", stroke_width: "1.25", vector_effect: "non-scaling-stroke" }
                polyline { points: "{cheenhub}", fill: "none", stroke: "#60a5fa", stroke_width: "1.15", vector_effect: "non-scaling-stroke" }
                polyline { points: "{database}", fill: "none", stroke: "#a78bfa", stroke_width: "1.15", vector_effect: "non-scaling-stroke" }
            }
        }
    }
}

fn cpu_breakdown(sample: &HostMetricsSample) -> Element {
    rsx! {
        div { class: "mt-4",
            div { class: "flex h-2 overflow-hidden rounded-full bg-zinc-800",
                div { class: "bg-blue-400", style: "width: {percentage_width(sample.cpu.cheenhub_percent)}" }
                div { class: "bg-violet-400", style: "width: {percentage_width(sample.cpu.database_percent)}" }
                div { class: "bg-zinc-500", style: "width: {percentage_width(sample.cpu.other_percent)}" }
            }
            div { class: "mt-3 grid gap-2 text-[12px] sm:grid-cols-3",
                {legend_value("CheenHub", format_percent(sample.cpu.cheenhub_percent), "bg-blue-400")}
                {legend_value("База данных", format_percent(sample.cpu.database_percent), "bg-violet-400")}
                {legend_value("Остальная система", format_percent(sample.cpu.other_percent), "bg-zinc-500")}
            }
        }
    }
}

fn logical_cpu_grid(samples: &[HostMetricsSample]) -> Element {
    let core_count = samples
        .last()
        .map(|sample| sample.cpu.logical_processors_percent.len())
        .unwrap_or(0);
    rsx! {
        div { class: "mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
            for core_index in 0..core_count {
                div { class: "rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
                    div { class: "flex items-center justify-between gap-3",
                        span { class: "text-[11px] font-medium text-zinc-500", "CPU {core_index}" }
                        strong { class: "tabular-nums text-[12px] font-semibold text-zinc-200", "{format_percent(samples.last().and_then(|sample| sample.cpu.logical_processors_percent.get(core_index)).copied().unwrap_or(0.0))}" }
                    }
                    svg { class: "mt-2 h-10 w-full", view_box: "0 0 100 24", preserve_aspect_ratio: "none", "aria-hidden": "true",
                        polyline { points: "{core_points(samples, core_index)}", fill: "none", stroke: "#60a5fa", stroke_width: "1.1", vector_effect: "non-scaling-stroke" }
                    }
                }
            }
        }
    }
}

fn memory_chart(samples: &[HostMetricsSample]) -> Element {
    let max = samples
        .last()
        .map(|sample| sample.memory.total_bytes as f64)
        .unwrap_or(1.0)
        .max(1.0);
    let used = points(samples, max, |sample| sample.memory.used_bytes as f64);
    rsx! {
        svg { class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3", view_box: "0 0 100 40", preserve_aspect_ratio: "none", role: "img", "aria-label": "История использования оперативной памяти",
            polyline { points: "{used}", fill: "none", stroke: "#34d399", stroke_width: "1.25", vector_effect: "non-scaling-stroke" }
        }
    }
}

fn memory_breakdown(sample: &HostMetricsSample) -> Element {
    let total = sample.memory.total_bytes.max(1) as f64;
    rsx! {
        div { class: "mt-4 grid gap-2 text-[12px]",
            {legend_value("CheenHub", format_bytes(sample.memory.cheenhub_bytes), "bg-blue-400")}
            {legend_value("База данных", format_bytes(sample.memory.database_bytes), "bg-violet-400")}
            {legend_value("Остальная система", format_bytes(sample.memory.other_bytes), "bg-zinc-500")}
            div { class: "mt-1 flex h-2 overflow-hidden rounded-full bg-zinc-800",
                div { class: "bg-blue-400", style: "width: {percentage_width((sample.memory.cheenhub_bytes as f64 / total * 100.0) as f32)}" }
                div { class: "bg-violet-400", style: "width: {percentage_width((sample.memory.database_bytes as f64 / total * 100.0) as f32)}" }
                div { class: "bg-zinc-500", style: "width: {percentage_width((sample.memory.other_bytes as f64 / total * 100.0) as f32)}" }
            }
        }
    }
}

fn network_chart(samples: &[HostMetricsSample]) -> Element {
    let max = samples
        .iter()
        .flat_map(|sample| {
            [
                sample.network.sent_bytes_per_second,
                sample.network.received_bytes_per_second,
            ]
        })
        .fold(1.0_f64, f64::max);
    let received = points(samples, max, |sample| {
        sample.network.received_bytes_per_second
    });
    let sent = points(samples, max, |sample| sample.network.sent_bytes_per_second);
    rsx! {
        svg { class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3", view_box: "0 0 100 40", preserve_aspect_ratio: "none", role: "img", "aria-label": "История сетевого трафика CheenHub",
            polyline { points: "{received}", fill: "none", stroke: "#60a5fa", stroke_width: "1.25", vector_effect: "non-scaling-stroke" }
            polyline { points: "{sent}", fill: "none", stroke: "#a78bfa", stroke_width: "1.25", vector_effect: "non-scaling-stroke" }
        }
    }
}

fn summary_card(
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

fn metric_value(label: &'static str, bytes_per_second: f64, color: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            p { class: "text-[11px] text-zinc-500", "{label}" }
            p { class: "mt-1 tabular-nums text-base font-semibold {color}", "{format_rate(bytes_per_second)}" }
        }
    }
}

fn legend_value(label: &'static str, value: String, dot_class: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center justify-between gap-3",
            span { class: "flex items-center gap-2 text-zinc-500", span { class: "size-2 rounded-full {dot_class}" } "{label}" }
            strong { class: "tabular-nums font-semibold text-zinc-200", "{value}" }
        }
    }
}

fn points(
    samples: &[HostMetricsSample],
    max_value: f64,
    value: impl Fn(&HostMetricsSample) -> f64,
) -> String {
    let denominator = samples.len().saturating_sub(1).max(1) as f64;
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let x = index as f64 / denominator * 100.0;
            let y = 40.0 - (value(sample) / max_value.max(0.001)).clamp(0.0, 1.0) * 40.0;
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn core_points(samples: &[HostMetricsSample], core_index: usize) -> String {
    let denominator = samples.len().saturating_sub(1).max(1) as f64;
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let x = index as f64 / denominator * 100.0;
            let value = sample
                .cpu
                .logical_processors_percent
                .get(core_index)
                .copied()
                .unwrap_or(0.0);
            let y = 24.0 - f64::from(value.clamp(0.0, 100.0)) / 100.0 * 24.0;
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn cpu_toggle_class(active: bool) -> &'static str {
    if active {
        "min-h-10 rounded-xl bg-zinc-800 px-3 text-[12px] font-semibold text-white shadow-[0_0_0_1px_rgba(255,255,255,0.07)] transition-[background-color,color,scale] duration-150 active:scale-[0.96]"
    } else {
        "min-h-10 rounded-xl px-3 text-[12px] font-semibold text-zinc-500 transition-[background-color,color,scale] duration-150 hover:text-white active:scale-[0.96]"
    }
}

fn percentage_width(value: f32) -> String {
    format!("{:.2}%", value.clamp(0.0, 100.0))
}

fn format_percent(value: f32) -> String {
    format!("{:.1}%", value.clamp(0.0, 100.0))
}

fn format_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= GIB {
        format!("{:.1} ГБ", bytes as f64 / GIB)
    } else {
        format!("{:.0} МБ", bytes as f64 / MIB)
    }
}

fn format_rate(bytes_per_second: f64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes_per_second >= MIB {
        format!("{:.1} МБ/с", bytes_per_second / MIB)
    } else if bytes_per_second >= KIB {
        format!("{:.1} КБ/с", bytes_per_second / KIB)
    } else {
        format!("{:.0} Б/с", bytes_per_second.max(0.0))
    }
}
