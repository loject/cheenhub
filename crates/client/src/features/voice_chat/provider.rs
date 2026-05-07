//! Voice connection context provider.

use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::{RealtimeConnectionStatus, RealtimeHandle};

use super::realtime;
use super::state::{VoiceConnectionHandle, VoiceConnectionState};

/// Provides voice connection state to authenticated app components.
#[component]
pub(crate) fn VoiceConnectionProvider(current_user: AuthUser, children: Element) -> Element {
    let realtime = use_context::<RealtimeHandle>();
    let state = use_signal(|| VoiceConnectionState::Disconnected);
    let handle = VoiceConnectionHandle::new(state, realtime.clone(), current_user);
    let context_handle = handle.clone();
    use_context_provider(move || context_handle.clone());

    let snapshot_realtime = realtime.clone();
    use_hook(move || {
        spawn(async move {
            let mut snapshots = realtime::subscribe_voice_chat(&snapshot_realtime);
            while let Some(snapshot) = snapshots.next().await {
                handle.apply_snapshot(snapshot);
            }
        })
    });
    let status_realtime = realtime.clone();
    use_hook(move || {
        spawn(async move {
            let mut statuses = status_realtime.subscribe_connection_status();
            while let Some(status) = statuses.next().await {
                if status == RealtimeConnectionStatus::Disconnected {
                    let mut state = state;
                    state.set(VoiceConnectionState::Disconnected);
                }
            }
        })
    });

    rsx! {
        {children}
    }
}
