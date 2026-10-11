//! Применение локальных инструментов к контрактам realtime и сетевых измерений.

use super::DeveloperModeHandle;
use crate::features::network::NetworkQualityHandle;
use crate::features::realtime::RealtimeHandle;
use dioxus::prelude::*;

/// Синхронизирует настройки разработчика с сетью внутри realtime-провайдера.
#[component]
pub(crate) fn DeveloperNetworkBridge() -> Element {
    let developer_mode = use_context::<DeveloperModeHandle>();
    let realtime = use_context::<RealtimeHandle>();
    let quality = use_context::<NetworkQualityHandle>();
    let initial_realtime = realtime.clone();
    use_hook(move || {
        apply_settings(developer_mode, &initial_realtime, quality);
    });
    use_effect(move || {
        apply_settings(developer_mode, &realtime, quality);
    });
    rsx! {}
}

fn apply_settings(
    developer_mode: DeveloperModeHandle,
    realtime: &RealtimeHandle,
    mut quality: NetworkQualityHandle,
) {
    let settings = developer_mode.current();
    realtime.set_force_websocket(settings.effective_force_websocket());
    quality.set_simulated_ping_delay_ms(settings.effective_ping_delay_ms());
}
