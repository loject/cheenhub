//! Контейнер системного графика на общем экземпляре библиотеки Apache ECharts.

use dioxus::{dioxus_core::spawn_forever, prelude::*};

use super::super::chart_bridge;

/// Отрисовывает и обновляет ECharts при изменении конфигурации.
#[component]
pub(super) fn MetricsTrendChart(
    name: &'static str,
    label: &'static str,
    class: &'static str,
    option: String,
) -> Element {
    let chart_id = use_signal(|| chart_bridge::new_chart_id(name));
    let mut chart_error = use_signal(|| None::<String>);

    // option — обычный prop, поэтому явно подписываем эффект на его изменение.
    use_effect(use_reactive!(|option| {
        let id = chart_id.peek().clone();
        spawn(async move {
            match chart_bridge::update_chart(&id, &option).await {
                Ok(()) => chart_error.set(None),
                Err(error) => {
                    warn!(%error, chart = %id, "failed to render host metrics chart");
                    chart_error.set(Some(error));
                }
            }
        });
    }));

    let dispose_id = chart_id.peek().clone();
    use_drop(move || {
        spawn_forever(async move {
            if let Err(error) = chart_bridge::dispose_chart(&dispose_id).await {
                debug!(%error, chart = %dispose_id, "failed to release host metrics chart");
            }
        });
    });

    rsx! {
        div { class: "{class}", role: "img", "aria-label": "{label}",
            div { class: "h-full w-full", id: "{chart_id}" }
            if let Some(error) = chart_error() {
                p { class: "text-[11px] text-amber-300", "{error}" }
            }
        }
    }
}
