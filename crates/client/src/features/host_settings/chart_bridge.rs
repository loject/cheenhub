//! Общий мост между Dioxus и Apache ECharts для графиков настроек хоста.
//!
//! Библиотека лежит локально в `public/vendor`, поэтому графики работают без
//! доступа в интернет. Мост построен на `document::eval`, который одинаково
//! доступен во всех наших целях: браузере, desktop-webview и Android WebView.
//! Отдельная платформенная реализация не требуется.
//!
//! Модуль отвечает только за жизненный цикл экземпляра графика: загрузку
//! библиотеки, создание, обновление и освобождение. Содержимое графика
//! (оси, серии, подписи) собирают вызывающие модули.

use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

/// Адрес локально вендоренной библиотеки.
const ECHARTS_URL: &str = "/vendor/echarts.min.js?v=5.5.1";

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
                if (existing.getDom() === element) {
                    existing.setOption(option, true);
                    existing.resize();
                    return true;
                }
                window.__cheenhubCharts.dispose(id);
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
            if (chart.getDom() !== document.getElementById(id)) {
                window.__cheenhubCharts.dispose(id);
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
/// `name` различает графики разных блоков в префиксе, а счётчик гарантирует
/// уникальность в пределах сессии. Идентификатор стабилен между перерисовками,
/// поэтому ECharts не пересоздаётся на каждом обновлении данных.
pub(super) fn new_chart_id(name: &str) -> String {
    let next = CHART_IDS.fetch_add(1, Ordering::Relaxed);
    format!("cheenhub-{name}-chart-{next}")
}

/// Создаёт экземпляр графика в контейнере `id` и сразу рисует `option`.
///
/// `option` — готовая JSON-конфигурация ECharts. Контейнер с таким `id`
/// должен уже существовать в разметке.
pub(super) async fn mount_chart(id: &str, option: &str) -> Result<(), String> {
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
    send_chart_payload(&eval, id, option)?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|error| format!("График не построился: {error}"))
}

/// Передаёт графику обновлённое содержимое без пересоздания экземпляра.
///
/// Если экземпляра ещё нет, мост создаёт его: быстрый первый ответ может
/// прийти раньше, чем контейнер будет отрисован.
pub(super) async fn update_chart(id: &str, option: &str) -> Result<(), String> {
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
    send_chart_payload(&eval, id, option)?;
    eval.join::<bool>()
        .await
        .map(|_| ())
        .map_err(|error| format!("График не обновился: {error}"))
}

/// Передаёт в скрипт идентификатор контейнера и конфигурацию графика.
fn send_chart_payload(eval: &document::Eval, id: &str, option: &str) -> Result<(), String> {
    eval.send(id.to_owned())
        .map_err(|_| "Не удалось подготовить график.".to_owned())?;
    eval.send(option.to_owned())
        .map_err(|_| "Не удалось подготовить данные графика.".to_owned())
}

#[cfg(test)]
mod tests;
