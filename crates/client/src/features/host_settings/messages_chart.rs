//! График количества сообщений в минуту за последние сутки.
//!
//! Модуль строит содержимое графика: временную шкалу суток, две серии —
//! сообщения комнат и личные сообщения — и шкалу значений. Экземпляр ECharts
//! ведёт общий [`super::chart_bridge`].

use cheenhub_contracts::rest::HostMessagesPerMinuteSample;
use dioxus::prelude::*;

use super::chart_bridge;

/// Короткое имя графика в идентификаторе контейнера.
const CHART_NAME: &str = "messages";

/// Глубина окна графика в миллисекундах: 24 часа.
const HISTORY_WINDOW_MS: i64 = 24 * 60 * MINUTE_MS;

/// Длина минуты в миллисекундах: график агрегирует значения по границам минут.
const MINUTE_MS: i64 = 60_000;

/// Возвращает уникальный идентификатор контейнера графика сообщений.
pub(super) fn new_chart_id() -> String {
    chart_bridge::new_chart_id(CHART_NAME)
}

/// Данные графика: сообщения комнат и личные сообщения по минутам.
pub(super) struct MessagesChartData {
    /// Конец суточного окна по часам сервера в миллисекундах Unix.
    pub(super) window_end_unix_ms: i64,
    /// Сообщения комнат по минутам.
    pub(super) room_samples: Vec<HostMessagesPerMinuteSample>,
    /// Личные сообщения по минутам.
    pub(super) direct_samples: Vec<HostMessagesPerMinuteSample>,
}

impl MessagesChartData {
    /// Возвращает максимум среди обеих серий для шкалы значений.
    fn peak(&self) -> u64 {
        self.room_samples
            .iter()
            .chain(self.direct_samples.iter())
            .map(|sample| sample.messages)
            .max()
            .unwrap_or(0)
    }

    /// Сообщает, есть ли хотя бы одна минута с данными.
    pub(super) fn is_empty(&self) -> bool {
        self.room_samples.is_empty() && self.direct_samples.is_empty()
    }
}

/// Создаёт экземпляр графика сообщений и сразу рисует первые данные.
pub(super) async fn mount_chart(id: &str, data: &MessagesChartData) -> Result<(), String> {
    chart_bridge::mount_chart(id, &chart_option(data)).await
}

/// Передаёт графику обновлённые данные без пересоздания экземпляра.
pub(super) async fn update_chart(id: &str, data: &MessagesChartData) -> Result<(), String> {
    chart_bridge::update_chart(id, &chart_option(data)).await
}

/// Обрезает обе серии до общего окна суток.
///
/// Обе серии используют границы серверного снимка. Начальная минута может
/// содержать сообщения после границы окна, поэтому её агрегат сохраняется.
pub(super) fn prune_to_window(mut data: MessagesChartData) -> MessagesChartData {
    let cutoff = (data.window_end_unix_ms - HISTORY_WINDOW_MS).div_euclid(MINUTE_MS) * MINUTE_MS;

    data.room_samples.retain(|sample| {
        sample.minute_unix_ms >= cutoff && sample.minute_unix_ms <= data.window_end_unix_ms
    });
    data.direct_samples.retain(|sample| {
        sample.minute_unix_ms >= cutoff && sample.minute_unix_ms <= data.window_end_unix_ms
    });
    data
}

/// Строит конфигурацию графика для ECharts.
///
/// Комнаты и личные сообщения рисуются отдельными сериями с одинаковой формой
/// столбца, поэтому их доли видно рядом, а не смешанными в одном столбце.
pub(super) fn chart_option(data: &MessagesChartData) -> String {
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
            "min": data.window_end_unix_ms - HISTORY_WINDOW_MS,
            "max": data.window_end_unix_ms,
            "axisLine": { "lineStyle": { "color": "#3f3f46" } },
            "axisTick": { "show": false },
            "axisLabel": { "color": "#71717a", "fontSize": 11, "hideOverlap": true },
            "splitLine": { "show": false }
        },
        "yAxis": {
            "type": "value",
            "min": 0,
            "max": nice_max(data.peak()),
            "splitNumber": 4,
            "axisLabel": { "color": "#71717a", "fontSize": 11 },
            "axisLine": { "show": false },
            "axisTick": { "show": false },
            "splitLine": { "lineStyle": { "color": "rgba(255,255,255,0.06)" } }
        },
        "series": [
            {
                "name": "В комнатах",
                "type": "bar",
                "barMaxWidth": 8,
                "itemStyle": {
                    "color": "#34d399",
                    "borderRadius": [2, 2, 0, 0]
                },
                "emphasis": { "itemStyle": { "color": "#6ee7b7" } },
                "data": series_points(&data.room_samples, data.window_end_unix_ms - HISTORY_WINDOW_MS)
            },
            {
                "name": "Личные",
                "type": "bar",
                "barMaxWidth": 8,
                "itemStyle": {
                    "color": "#38bdf8",
                    "borderRadius": [2, 2, 0, 0]
                },
                "emphasis": { "itemStyle": { "color": "#7dd3fc" } },
                "data": series_points(&data.direct_samples, data.window_end_unix_ms - HISTORY_WINDOW_MS)
            }
        ]
    });

    option.to_string()
}

/// Столбцы серии: минута и число сообщений в ней.
///
/// Столбцы выравниваются по началу минуты. Первая неполная минута рисуется
/// на границе окна, чтобы её сообщения не скрывались за пределами оси.
fn series_points(
    samples: &[HostMessagesPerMinuteSample],
    window_start: i64,
) -> Vec<serde_json::Value> {
    samples
        .iter()
        .map(|sample| serde_json::json!([sample.minute_unix_ms.max(window_start), sample.messages]))
        .collect()
}

/// Верхняя граница шкалы значений с округлением вверх до ровного числа.
fn nice_max(maximum: u64) -> u64 {
    match maximum {
        0..=4 => 4,
        5..=10 => (maximum / 2 + 1) * 2,
        _ => (maximum / 5 + 1) * 5,
    }
}

#[cfg(test)]
mod tests;
