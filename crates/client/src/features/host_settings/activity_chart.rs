//! График активности голосового чата на Apache ECharts.
//!
//! Библиотека лежит локально в `public/vendor`, поэтому график работает без
//! доступа в интернет. Мост между Dioxus и ECharts построен на `document::eval`,
//! который одинаково доступен во всех наших целях: браузере, desktop-webview и
//! Android WebView. Отдельная платформенная реализация не требуется.

use std::sync::atomic::{AtomicU32, Ordering};

use cheenhub_contracts::rest::HostVoiceActivitySample;
use dioxus::prelude::*;

/// Адрес локально вендоренной библиотеки.
const ECHARTS_URL: &str = "/vendor/echarts.min.js?v=5.5.1";
/// Глубина показываемой истории в миллисекундах: 24 часа.
const HISTORY_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;
/// Пропуск между измерениями, который считается простоем сервера.
const SAMPLE_GAP_MS: i64 = 30_000;

static CHART_IDS: AtomicU32 = AtomicU32::new(0);

/// Мост между Dioxus и ECharts: загрузка библиотеки, экземпляры и ресайз.
///
/// Объект кладётся в `window` один раз на всё приложение и переиспользуется
/// всеми графиками; повторные вызовы `document::eval` просто берут тот же объект.
const BRIDGE: &str = r#"(function bootstrap() {
    if (window.__cheenhubCharts) {
        return;
    }
    const registry = new Map();
    let loading = null;

    const load = () => {
        if (window.echarts) {
            return Promise.resolve(window.echarts);
        }
        if (loading) {
            return loading;
        }
        loading = new Promise((resolve, reject) => {
            const script = document.createElement('script');
            script.src = '__ECHARTS_URL__';
            script.async = true;
            script.onload = () => window.echarts
                ? resolve(window.echarts)
                : reject(new Error('echarts asset loaded without global'));
            script.onerror = () => reject(new Error('failed to load echarts asset'));
            document.head.appendChild(script);
        });
        return loading;
    };

    window.__cheenhubCharts = {
        async mount(id, option) {
            const echarts = await load();
            const element = document.getElementById(id);
            if (!element) {
                throw new Error('chart container is missing');
            }
            const existing = registry.get(id);
            if (existing) {
                existing.setOption(option, true);
                existing.resize();
                return true;
            }
            const chart = echarts.init(element, null, { renderer: 'canvas' });
            chart.setOption(option, true);
            if (typeof ResizeObserver !== 'undefined') {
                const observer = new ResizeObserver(() => chart.resize());
                observer.observe(element);
                chart.__cheenhubResizeObserver = observer;
            }
            registry.set(id, chart);
            return true;
        },
        update(id, option) {
            const chart = registry.get(id);
            if (!chart) {
                return false;
            }
            chart.setOption(option, { replaceMerge: ['series'] });
            chart.resize();
            return true;
        },
        dispose(id) {
            const chart = registry.get(id);
            if (!chart) {
                return false;
            }
            if (chart.__cheenhubResizeObserver) {
                chart.__cheenhubResizeObserver.disconnect();
            }
            chart.dispose();
            registry.delete(id);
            return true;
        }
    };
})();"#;

/// Подставляет адрес библиотеки в подготовленный скрипт моста.
fn bridge_script() -> String {
    BRIDGE.replace("__ECHARTS_URL__", ECHARTS_URL)
}

/// Возвращает уникальный идентификатор контейнера графика.
///
/// Идентификатор стабилен между перерисовками, поэтому ECharts не пересоздаётся
/// на каждом обновлении истории, а только получает новые данные.
pub(super) fn new_chart_id() -> String {
    let next = CHART_IDS.fetch_add(1, Ordering::Relaxed);
    format!("cheenhub-activity-chart-{next}")
}

/// Создаёт экземпляр графика и сразу рисует первые данные.
pub(super) async fn mount_chart(
    id: &str,
    samples: &[HostVoiceActivitySample],
) -> Result<(), String> {
    let eval = document::eval(&format!(
        r#"
        const id = await dioxus.recv();
        const option = JSON.parse(await dioxus.recv());
        {bridge}
        await window.__cheenhubCharts.mount(id, option);
        return true;
        "#,
        bridge = bridge_script()
    ));
    eval.send(id.to_owned())
        .map_err(|_| "Не удалось подготовить график.".to_owned())?;
    eval.send(chart_option(samples))
        .map_err(|_| "Не удалось подготовить данные графика.".to_owned())?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|error| format!("График активности не построился: {error}"))
}

/// Передаёт графику обновлённые измерения без пересоздания экземпляра.
pub(super) async fn update_chart(
    id: &str,
    samples: &[HostVoiceActivitySample],
) -> Result<(), String> {
    let eval = document::eval(&format!(
        r#"
        const id = await dioxus.recv();
        const option = JSON.parse(await dioxus.recv());
        {bridge}
        if (!window.__cheenhubCharts.update(id, option)) {{
            await window.__cheenhubCharts.mount(id, option);
        }}
        return true;
        "#,
        bridge = bridge_script()
    ));
    eval.send(id.to_owned())
        .map_err(|_| "Не удалось подготовить обновление графика.".to_owned())?;
    eval.send(chart_option(samples))
        .map_err(|_| "Не удалось подготовить данные графика.".to_owned())?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|error| format!("График активности не обновился: {error}"))
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
