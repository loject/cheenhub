//! Графики и форматирование значений системных метрик хоста.

use cheenhub_contracts::rest::HostMetricsSample;
use dioxus::prelude::*;
use serde_json::{Value, json};

use super::metrics_chart::MetricsTrendChart;

pub(super) fn cpu_total_chart(samples: &[HostMetricsSample]) -> Element {
    let option = chart_option(
        vec![
            line_series("Система", "#e4e4e7", samples, |sample| f64::from(sample.cpu.system_percent)),
            line_series("CheenHub", "#60a5fa", samples, |sample| f64::from(sample.cpu.cheenhub_percent)),
            line_series("База данных", "#a78bfa", samples, |sample| f64::from(sample.cpu.database_percent)),
        ],
        "%",
        Some(100.0),
        false,
    );
    rsx! {
        MetricsTrendChart {
            name: "cpu-total",
            label: "История нагрузки процессора",
            class: "mt-4 h-36 rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            option,
        }
    }
}

pub(super) fn cpu_breakdown(sample: &HostMetricsSample) -> Element {
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

pub(super) fn logical_cpu_grid(samples: &[HostMetricsSample]) -> Element {
    let core_count = samples
        .last()
        .map(|sample| sample.cpu.logical_processors_percent.len())
        .unwrap_or(0);
    rsx! {
        div { class: "mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4",
            for core_index in 0..core_count {
                div { key: "{core_index}", class: "rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
                    div { class: "flex items-center justify-between gap-3",
                        span { class: "text-[11px] font-medium text-zinc-500", "CPU {core_index}" }
                        strong { class: "tabular-nums text-[12px] font-semibold text-zinc-200", "{format_percent(samples.last().and_then(|sample| sample.cpu.logical_processors_percent.get(core_index)).copied().unwrap_or(0.0))}" }
                    }
                    MetricsTrendChart {
                        name: "cpu-core",
                        label: "История нагрузки логического ядра",
                        class: "mt-2 h-10 w-full",
                        option: chart_option(
                            vec![line_series("CPU", "#60a5fa", samples, |sample| {
                                f64::from(sample.cpu.logical_processors_percent.get(core_index).copied().unwrap_or(0.0))
                            })],
                            "%",
                            Some(100.0),
                            true,
                        ),
                    }
                }
            }
        }
    }
}

pub(super) fn memory_chart(samples: &[HostMetricsSample]) -> Element {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let max = samples.iter().map(|sample| sample.memory.total_bytes).max().unwrap_or(0) as f64 / GIB;
    let option = chart_option(
        vec![line_series("Занято", "#34d399", samples, |sample| sample.memory.used_bytes as f64 / GIB)],
        " ГБ",
        Some(max.max(1.0)),
        false,
    );
    rsx! {
        MetricsTrendChart {
            name: "memory",
            label: "История использования оперативной памяти",
            class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            option,
        }
    }
}

pub(super) fn memory_breakdown(sample: &HostMetricsSample) -> Element {
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

pub(super) fn disk_chart(samples: &[HostMetricsSample]) -> Element {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let max = samples.iter().filter_map(|sample| sample.disk.as_ref()).map(|disk| disk.total_bytes).max().unwrap_or(0) as f64 / GIB;
    let option = chart_option(
        vec![line_series("Занято", "#fbbf24", samples, |sample| {
            sample.disk.as_ref().map_or(0.0, |disk| disk.used_bytes as f64 / GIB)
        })],
        " ГБ",
        Some(max.max(1.0)),
        false,
    );
    rsx! {
        MetricsTrendChart {
            name: "disk",
            label: "История занятого места на накопителе",
            class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            option,
        }
    }
}

pub(super) fn network_chart(samples: &[HostMetricsSample]) -> Element {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * KIB;
    let peak = samples.iter().flat_map(|sample| [sample.network.sent_bytes_per_second, sample.network.received_bytes_per_second]).fold(0.0_f64, f64::max);
    let (scale, unit) = if peak >= MIB {
        (MIB, " МБ/с")
    } else if peak >= KIB {
        (KIB, " КБ/с")
    } else {
        (1.0, " Б/с")
    };
    let option = chart_option(
        vec![
            line_series("Получение", "#60a5fa", samples, |sample| sample.network.received_bytes_per_second / scale),
            line_series("Отправка", "#a78bfa", samples, |sample| sample.network.sent_bytes_per_second / scale),
        ],
        unit,
        None,
        false,
    );
    rsx! {
        MetricsTrendChart {
            name: "network",
            label: "История сетевого трафика CheenHub",
            class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            option,
        }
    }
}

pub(super) fn metric_value(
    label: &'static str,
    bytes_per_second: f64,
    color: &'static str,
) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800/80 bg-zinc-950 p-3",
            p { class: "text-[11px] text-zinc-500", "{label}" }
            p { class: "mt-1 tabular-nums text-base font-semibold {color}", "{format_rate(bytes_per_second)}" }
        }
    }
}

pub(super) fn legend_value(label: &'static str, value: String, dot_class: &'static str) -> Element {
    rsx! {
        div { class: "flex items-center justify-between gap-3",
            span { class: "flex items-center gap-2 text-zinc-500", span { class: "size-2 rounded-full {dot_class}" } "{label}" }
            strong { class: "tabular-nums font-semibold text-zinc-200", "{value}" }
        }
    }
}

/// Создаёт серию ECharts с настоящими отметками времени, а не индексами точек.
fn line_series(
    name: &str,
    color: &str,
    samples: &[HostMetricsSample],
    value: impl Fn(&HostMetricsSample) -> f64,
) -> Value {
    let data: Vec<_> = samples.iter().map(|sample| json!([sample.sampled_at_unix_ms, value(sample)])).collect();
    json!({
        "name": name,
        "type": "line",
        "showSymbol": false,
        "smooth": false,
        "sampling": "lttb",
        "lineStyle": { "width": 2, "color": color },
        "itemStyle": { "color": color },
        "data": data
    })
}

/// Общая конфигурация осей, подсказок и сетки для графиков системных метрик.
fn chart_option(series: Vec<Value>, unit: &str, max: Option<f64>, compact: bool) -> String {
    let mut y_axis = json!({
        "type": "value",
        "min": 0,
        "axisLine": { "show": false },
        "axisTick": { "show": false },
        "axisLabel": { "color": "#71717a", "fontSize": 10, "formatter": format!("{{value}}{unit}") },
        "splitLine": { "lineStyle": { "color": "rgba(255,255,255,0.06)" } }
    });
    if let Some(max) = max {
        y_axis["max"] = json!(max);
    }
    if compact {
        y_axis["show"] = json!(false);
    }
    json!({
        "animation": false,
        "grid": if compact {
            json!({ "left": 1, "right": 1, "top": 2, "bottom": 2 })
        } else {
            json!({ "left": 4, "right": 8, "top": 8, "bottom": 12, "containLabel": true })
        },
        "tooltip": {
            "trigger": "axis",
            "confine": true,
            "backgroundColor": "#09090b",
            "borderColor": "#3f3f46",
            "textStyle": { "color": "#e4e4e7", "fontSize": 11 }
        },
        "xAxis": {
            "type": "time",
            "boundaryGap": false,
            "axisLine": { "lineStyle": { "color": "#3f3f46" } },
            "axisTick": { "show": false },
            "axisLabel": { "color": "#71717a", "fontSize": 10, "hideOverlap": true },
            "splitLine": { "show": false },
            "show": !compact
        },
        "yAxis": y_axis,
        "series": series
    }).to_string()
}

pub(super) fn cpu_toggle_class(active: bool) -> &'static str {
    if active {
        "min-h-10 rounded-xl bg-zinc-800 px-3 text-[12px] font-semibold text-white shadow-[0_0_0_1px_rgba(255,255,255,0.07)] transition-[background-color,color,scale] duration-150 active:scale-[0.96]"
    } else {
        "min-h-10 rounded-xl px-3 text-[12px] font-semibold text-zinc-500 transition-[background-color,color,scale] duration-150 hover:text-white active:scale-[0.96]"
    }
}

pub(super) fn percentage_width(value: f32) -> String {
    format!("{:.2}%", value.clamp(0.0, 100.0))
}

pub(super) fn format_percent(value: f32) -> String {
    format!("{:.1}%", value.clamp(0.0, 100.0))
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const TIB: f64 = 1024.0 * 1024.0 * 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= TIB {
        format!("{:.1} ТБ", bytes as f64 / TIB)
    } else if bytes as f64 >= GIB {
        format!("{:.1} ГБ", bytes as f64 / GIB)
    } else {
        format!("{:.0} МБ", bytes as f64 / MIB)
    }
}

pub(super) fn format_rate(bytes_per_second: f64) -> String {
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
