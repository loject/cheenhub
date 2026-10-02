//! Графики и форматирование значений системных метрик хоста.

use cheenhub_contracts::rest::HostMetricsSample;
use dioxus::prelude::*;

pub(super) fn cpu_total_chart(samples: &[HostMetricsSample]) -> Element {
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

pub(super) fn memory_chart(samples: &[HostMetricsSample]) -> Element {
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
    let max = samples
        .last()
        .and_then(|sample| sample.disk.as_ref())
        .map(|disk| disk.total_bytes as f64)
        .unwrap_or(1.0)
        .max(1.0);
    let used = points(samples, max, |sample| {
        sample
            .disk
            .as_ref()
            .expect("filtered samples contain disk metrics")
            .used_bytes as f64
    });
    rsx! {
        svg { class: "mt-5 h-32 w-full rounded-xl border border-zinc-800/80 bg-zinc-950 p-3", view_box: "0 0 100 40", preserve_aspect_ratio: "none", role: "img", "aria-label": "История занятого места на накопителе",
            polyline { points: "{used}", fill: "none", stroke: "#fbbf24", stroke_width: "1.25", vector_effect: "non-scaling-stroke" }
        }
    }
}

pub(super) fn network_chart(samples: &[HostMetricsSample]) -> Element {
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

pub(super) fn points(
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

pub(super) fn core_points(samples: &[HostMetricsSample], core_index: usize) -> String {
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
