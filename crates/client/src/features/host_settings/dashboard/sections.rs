//! Секции системных метрик хоста с независимой деградацией без данных.
//!
//! Каждая секция умеет показать собственное состояние «нет данных», поэтому
//! недоступность сборщика метрик не скрывает остальные части дашборда.

use cheenhub_contracts::rest::HostMetricsSample;
use dioxus::prelude::*;

use super::charts::{
    cpu_breakdown, cpu_toggle_class, cpu_total_chart, disk_chart, format_bytes, format_percent,
    format_rate, logical_cpu_grid, memory_breakdown, memory_chart, metric_value, network_chart,
    percentage_width,
};
use super::states::section_without_data;
use super::{CpuView, summary_card};

/// Рендерит сводные карточки и секции нагрузки хоста.
///
/// Пустой `samples` не является ошибкой страницы: вместо графиков каждая секция
/// показывает своё состояние отсутствия данных.
pub(super) fn metrics_sections(
    samples: &[HostMetricsSample],
    cpu_view: Signal<CpuView>,
) -> Element {
    let latest = samples.last();
    let disk_samples: Vec<HostMetricsSample> = samples
        .iter()
        .filter(|sample| sample.disk.is_some())
        .cloned()
        .collect();

    rsx! {
        div { class: "mt-6 grid gap-3 sm:grid-cols-2 xl:grid-cols-5",
            {summary_card("Процессор", cpu_value(latest), "общая нагрузка", "text-white")}
            {summary_card("Оперативная память", memory_value(latest), memory_hint(latest), "text-emerald-300")}
            {summary_card("Накопитель", disk_value(latest), disk_hint(latest), "text-amber-300")}
            {summary_card("Получение", network_value(latest, |sample| sample.network.received_bytes_per_second), "сеть CheenHub", "text-blue-300")}
            {summary_card("Отправка", network_value(latest, |sample| sample.network.sent_bytes_per_second), "сеть CheenHub", "text-violet-300")}
        }

        {cpu_section(latest, samples, cpu_view)}

        div { class: "mt-5 grid gap-5 lg:grid-cols-2",
            {memory_section(latest, samples)}
            {disk_section(latest, &disk_samples)}
        }

        {network_section(latest, samples)}
    }
}

fn cpu_section(
    latest: Option<&HostMetricsSample>,
    samples: &[HostMetricsSample],
    cpu_view: Signal<CpuView>,
) -> Element {
    let mut cpu_view = cpu_view;
    let Some(latest) = latest else {
        return rsx! {
            section { class: "mt-4 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Процессор" }
                {section_without_data("Загрузка процессора появится, когда сборщик метрик начнёт передавать измерения.")}
            }
        };
    };
    let total_button = cpu_toggle_class(cpu_view() == CpuView::Total);
    let logical_button = cpu_toggle_class(cpu_view() == CpuView::Logical);

    rsx! {
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
                {cpu_total_chart(samples)}
                {cpu_breakdown(latest)}
            } else {
                {logical_cpu_grid(samples)}
            }
        }
    }
}

fn memory_section(latest: Option<&HostMetricsSample>, samples: &[HostMetricsSample]) -> Element {
    let Some(latest) = latest else {
        return rsx! {
            section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Оперативная память" }
                {section_without_data("Данные об оперативной памяти появятся после первого измерения.")}
            }
        };
    };

    rsx! {
        section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Оперативная память" }
            div { class: "mt-1 flex items-baseline gap-2",
                strong { class: "tabular-nums text-2xl font-semibold tracking-[-0.04em] text-white", "{format_bytes(latest.memory.used_bytes)}" }
                span { class: "text-sm text-zinc-500", "из {format_bytes(latest.memory.total_bytes)}" }
            }
            {memory_chart(samples)}
            {memory_breakdown(latest)}
        }
    }
}

fn disk_section(latest: Option<&HostMetricsSample>, disk_samples: &[HostMetricsSample]) -> Element {
    let Some(disk) = latest.and_then(|sample| sample.disk.as_ref()) else {
        return rsx! {
            section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Накопитель" }
                {section_without_data("Данные о накопителе временно недоступны.")}
            }
        };
    };
    let used_percent = used_share(disk.used_bytes, disk.total_bytes);

    rsx! {
        section { class: "rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Накопитель" }
            div { class: "mt-1 flex items-baseline gap-2",
                strong { class: "tabular-nums text-2xl font-semibold tracking-[-0.04em] text-white", "{format_bytes(disk.used_bytes)}" }
                span { class: "text-sm text-zinc-500", "из {format_bytes(disk.total_bytes)}" }
            }
            div { class: "mt-4 flex h-2 overflow-hidden rounded-full bg-zinc-800",
                div { class: "bg-amber-400", style: "width: {percentage_width(used_percent)}" }
            }
            {disk_chart(disk_samples)}
        }
    }
}

fn network_section(latest: Option<&HostMetricsSample>, samples: &[HostMetricsSample]) -> Element {
    let Some(latest) = latest else {
        return rsx! {
            section { class: "mt-5 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5",
                p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Сеть CheenHub" }
                {section_without_data("Счётчики сетевого трафика появятся после первого измерения.")}
            }
        };
    };

    rsx! {
        section { class: "mt-5 rounded-[20px] border border-zinc-800 bg-zinc-950/70 p-5 shadow-[0_18px_60px_rgba(0,0,0,.22)]",
            p { class: "text-[11px] font-semibold uppercase tracking-[0.16em] text-zinc-500", "Сеть CheenHub" }
            div { class: "mt-2 grid grid-cols-2 gap-3",
                {metric_value("Получение", latest.network.received_bytes_per_second, "text-blue-300")}
                {metric_value("Отправка", latest.network.sent_bytes_per_second, "text-violet-300")}
            }
            {network_chart(samples)}
        }
    }
}

fn cpu_value(latest: Option<&HostMetricsSample>) -> String {
    latest.map_or_else(
        || "—".to_owned(),
        |sample| format_percent(sample.cpu.system_percent),
    )
}

fn network_value(
    latest: Option<&HostMetricsSample>,
    value: impl Fn(&HostMetricsSample) -> f64,
) -> String {
    latest.map_or_else(|| "—".to_owned(), |sample| format_rate(value(sample)))
}

fn memory_value(latest: Option<&HostMetricsSample>) -> String {
    latest.map_or_else(
        || "—".to_owned(),
        |sample| {
            format_percent(used_share(
                sample.memory.used_bytes,
                sample.memory.total_bytes,
            ))
        },
    )
}

fn memory_hint(latest: Option<&HostMetricsSample>) -> String {
    latest.map_or_else(
        || "нет данных".to_owned(),
        |sample| {
            format!(
                "{} из {}",
                format_bytes(sample.memory.used_bytes),
                format_bytes(sample.memory.total_bytes)
            )
        },
    )
}

fn disk_value(latest: Option<&HostMetricsSample>) -> String {
    latest.and_then(|sample| sample.disk.as_ref()).map_or_else(
        || "—".to_owned(),
        |disk| format_percent(used_share(disk.used_bytes, disk.total_bytes)),
    )
}

fn disk_hint(latest: Option<&HostMetricsSample>) -> String {
    latest.and_then(|sample| sample.disk.as_ref()).map_or_else(
        || "нет данных".to_owned(),
        |disk| {
            format!(
                "{} из {}",
                format_bytes(disk.used_bytes),
                format_bytes(disk.total_bytes)
            )
        },
    )
}

/// Доля занятого объёма в процентах; нулевой объём не даёт деления на ноль.
fn used_share(used: u64, total: u64) -> f32 {
    if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64 * 100.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{cpu_value, disk_hint, disk_value, memory_hint, memory_value, network_value};

    #[test]
    fn summary_cards_degrade_to_placeholders_without_samples() {
        assert_eq!(cpu_value(None), "—");
        assert_eq!(memory_value(None), "—");
        assert_eq!(disk_value(None), "—");
        assert_eq!(memory_hint(None), "нет данных");
        assert_eq!(disk_hint(None), "нет данных");
        assert_eq!(
            network_value(None, |sample| sample.network.sent_bytes_per_second),
            "—"
        );
    }
}
