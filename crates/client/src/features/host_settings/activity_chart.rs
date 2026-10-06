//! График активности голосового чата на Apache ECharts.
//!
//! Модуль собирает только содержимое графика: временную шкалу, серии и шкалу
//! значений. Жизненный цикл экземпляра ECharts берёт на себя общий
//! [`super::chart_bridge`], поэтому оба графика дашборда используют один
//! загруженный экземпляр библиотеки.

use cheenhub_contracts::rest::HostVoiceActivitySample;
use dioxus::prelude::*;

use super::chart_bridge;

/// Глубина показываемой истории в миллисекундах: 24 часа.
const HISTORY_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
/// Пропуск между измерениями, который считается простоем сервера.
const SAMPLE_GAP_MS: i64 = 30_000;

/// Короткое имя графика в идентификаторе контейнера.
const CHART_NAME: &str = "activity";

/// Возвращает уникальный идентификатор контейнера графика активности.
pub(super) fn new_chart_id() -> String {
    chart_bridge::new_chart_id(CHART_NAME)
}

/// Создаёт экземпляр графика активности и сразу рисует первые данные.
pub(super) async fn mount_chart(
    id: &str,
    samples: &[HostVoiceActivitySample],
) -> Result<(), String> {
    chart_bridge::mount_chart(id, &chart_option(samples)).await
}

/// Передаёт графику активности обновлённые измерения.
pub(super) async fn update_chart(
    id: &str,
    samples: &[HostVoiceActivitySample],
) -> Result<(), String> {
    chart_bridge::update_chart(id, &chart_option(samples)).await
}

/// Удаляет точки старше 24 часов, чтобы график соответствовал заявленному окну.
pub(super) fn prune_to_window(
    samples: Vec<HostVoiceActivitySample>,
) -> Vec<HostVoiceActivitySample> {
    let newest = samples
        .iter()
        .map(|sample| sample.sampled_at_unix_ms)
        .max()
        .unwrap_or(0);
    let cutoff = newest - HISTORY_WINDOW_MS;
    samples
        .into_iter()
        .filter(|sample| sample.sampled_at_unix_ms > cutoff)
        .collect()
}

/// Описание gap-разрывов для серии: пропуск измерений обрывает линию.
///
/// ECharts разрывает линию на `null`-значении, поэтому после простоя добавляется
/// пустая точка: пропуск сервера виден как разрыв, а не как провал до нуля.
fn series_points(
    samples: &[HostVoiceActivitySample],
    value: impl Fn(&HostVoiceActivitySample) -> u32,
) -> Vec<serde_json::Value> {
    let mut points: Vec<serde_json::Value> = Vec::with_capacity(samples.len());
    let mut previous_time: Option<i64> = None;

    for sample in samples {
        if previous_time.is_some_and(|time| sample.sampled_at_unix_ms - time > SAMPLE_GAP_MS) {
            points.push(serde_json::json!([
                previous_time.expect("gap follows a sample") + 1,
                null
            ]));
        }
        points.push(serde_json::json!([
            sample.sampled_at_unix_ms,
            value(sample)
        ]));
        previous_time = Some(sample.sampled_at_unix_ms);
    }
    points
}

/// Верхняя граница шкалы значений с округлением вверх до ровного числа.
fn nice_max(maximum: u32) -> u32 {
    match maximum {
        0..=4 => 4,
        5..=10 => (maximum / 2 + 1) * 2,
        _ => (maximum / 5 + 1) * 5,
    }
}

/// Строит конфигурацию графика для ECharts.
fn chart_option(samples: &[HostVoiceActivitySample]) -> String {
    let voice_max = samples
        .iter()
        .map(|sample| sample.voice_connections)
        .max()
        .unwrap_or(0);
    let video_max = samples
        .iter()
        .map(|sample| sample.video_sources)
        .max()
        .unwrap_or(0);
    let option = serde_json::json!({
        "animationDuration": 350,
        "animationEasing": "cubicOut",
        "grid": { "left": 8, "right": 16, "top": 16, "bottom": 24, "containLabel": true },
        "tooltip": {
            "trigger": "axis",
            "confine": true,
            "backgroundColor": "#09090b",
            "borderColor": "#3f3f46",
            "textStyle": { "color": "#e4e4e7", "fontSize": 12 }
        },
        "xAxis": {
            "type": "time",
            "axisLine": { "lineStyle": { "color": "#3f3f46" } },
            "axisTick": { "show": false },
            "axisLabel": { "color": "#71717a", "fontSize": 11, "hideOverlap": true },
            "splitLine": { "show": false }
        },
        "yAxis": {
            "type": "value",
            "min": 0,
            "max": nice_max(voice_max.max(video_max)),
            "splitNumber": 4,
            "axisLabel": { "color": "#71717a", "fontSize": 11 },
            "axisLine": { "show": false },
            "axisTick": { "show": false },
            "splitLine": { "lineStyle": { "color": "rgba(255,255,255,0.06)" } }
        },
        "series": [
            {
                "name": "Подключения",
                "type": "line",
                "smooth": 0.25,
                "showSymbol": false,
                "sampling": "lttb",
                "connectNulls": false,
                "lineStyle": { "width": 2, "color": "#60a5fa" },
                "itemStyle": { "color": "#60a5fa" },
                "areaStyle": {
                    "color": {
                        "type": "linear", "x": 0, "y": 0, "x2": 0, "y2": 1,
                        "colorStops": [
                            { "offset": 0, "color": "rgba(96,165,250,0.38)" },
                            { "offset": 1, "color": "rgba(96,165,250,0.02)" }
                        ]
                    }
                },
                "data": series_points(samples, |sample| sample.voice_connections)
            },
            {
                "name": "Видеоисточники",
                "type": "line",
                "smooth": 0.25,
                "showSymbol": false,
                "sampling": "lttb",
                "connectNulls": false,
                "lineStyle": { "width": 2, "type": "dashed", "color": "#c084fc" },
                "itemStyle": { "color": "#c084fc" },
                "areaStyle": {
                    "color": {
                        "type": "linear", "x": 0, "y": 0, "x2": 0, "y2": 1,
                        "colorStops": [
                            { "offset": 0, "color": "rgba(192,132,252,0.32)" },
                            { "offset": 1, "color": "rgba(192,132,252,0.02)" }
                        ]
                    }
                },
                "data": series_points(samples, |sample| sample.video_sources)
            }
        ]
    });

    option.to_string()
}

#[cfg(test)]
mod tests;
