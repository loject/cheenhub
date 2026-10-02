//! Dioxus-провайдер realtime.

use crate::features::network::{NetworkQualityHandle, RealtimeFallbackNotice};
use dioxus::dioxus_core::spawn_forever;
use dioxus::prelude::*;

use super::activity::ActivityGate;
use super::connection_runtime::run_connection;
use super::handle::create_handle;

/// Предоставляет realtime-контекст аутентифицированным компонентам приложения.
#[component]
pub(crate) fn RealtimeProvider(children: Element) -> Element {
    let handle = use_signal(create_handle);
    let realtime = handle();
    let context_realtime = realtime.clone();
    use_context_provider(move || context_realtime.clone());
    let network_quality_state = use_signal(Default::default);
    let frequent_network_probes = use_signal(|| false);
    let network_quality = NetworkQualityHandle::new(network_quality_state, frequent_network_probes);
    use_context_provider(move || network_quality);

    let drop_realtime = realtime.clone();
    use_drop(move || {
        spawn_forever(async move {
            drop_realtime.mark_disconnected().await;
        });
    });

    use_hook(move || {
        let visibility = crate::features::runtime::lifecycle::subscribe_visibility();
        let background = realtime.subscribe_background_activity();
        spawn(async move {
            let mut gate = ActivityGate::new(visibility, background);
            let mut network_quality = network_quality;
            loop {
                gate.wait_until_allowed().await;
                info!("realtime activity allowed; starting connection runtime");
                gate.run_until_suspended(run_connection(&realtime, network_quality))
                    .await;
                info!("realtime suspended while application is hidden without active session");
                network_quality.clear();
                realtime.mark_disconnected().await;
            }
        })
    });

    rsx! {
        RealtimeFallbackNotice {}
        {children}
    }
}
