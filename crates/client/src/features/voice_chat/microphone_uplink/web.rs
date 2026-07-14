//! Dedicated Worker uplink микрофона для браузера.

use std::cell::Cell;

use dioxus::prelude::{debug, info, spawn, warn};

use crate::features::auth::api;
use crate::features::microphone::{MicrophoneHandle, MicrophoneStatus, MicrophoneUplinkConfig};
use crate::features::realtime::{
    RealtimeHandle, realtime_cert_sha256_hex, realtime_url, realtime_websocket_url,
};

use super::super::{realtime, voice_frame_sender};

thread_local! {
    static PREPARATION_GENERATION: Cell<u64> = const { Cell::new(0) };
}

/// Перезапускает микрофон через отдельную browser realtime-сессию.
pub(crate) fn restart(
    microphone: MicrophoneHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    prepare_and_start(microphone, realtime_handle, server_id, room_id, true);
}

/// Переключает микрофон через отдельную browser realtime-сессию.
pub(crate) fn toggle(
    microphone: MicrophoneHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    if matches!(
        microphone.status_untracked(),
        MicrophoneStatus::Live | MicrophoneStatus::Starting
    ) {
        next_preparation_generation();
        microphone.stop();
        return;
    }

    prepare_and_start(microphone, realtime_handle, server_id, room_id, false);
}

fn prepare_and_start(
    microphone: MicrophoneHandle,
    realtime_handle: RealtimeHandle,
    server_id: String,
    room_id: String,
    restart: bool,
) {
    let preparation_generation = next_preparation_generation();
    let microphone_generation = microphone.operation_generation();
    spawn(async move {
        let uplink = match prepare_uplink(&realtime_handle, &room_id).await {
            Ok(uplink) => uplink,
            Err(error) => {
                warn!(%error, %room_id, "failed to prepare dedicated microphone uplink");
                if preparation_is_current(preparation_generation)
                    && microphone.operation_generation() == microphone_generation
                {
                    microphone.stop();
                }
                return;
            }
        };
        if !preparation_is_current(preparation_generation)
            || microphone.operation_generation() != microphone_generation
        {
            debug!(%room_id, "discarded stale dedicated microphone uplink grant");
            return;
        }

        let fallback = voice_frame_sender::voice_frame_sender_callback(
            realtime_handle,
            server_id,
            room_id.clone(),
        );
        info!(%room_id, restart, "starting dedicated microphone worker uplink");
        if restart {
            microphone.restart_with_uplink(fallback, uplink);
        } else {
            microphone.start_with_uplink(fallback, uplink);
        }
    });
}

async fn prepare_uplink(
    realtime_handle: &RealtimeHandle,
    room_id: &str,
) -> Result<MicrophoneUplinkConfig, String> {
    let access_token = api::fresh_access_token().await?;
    let grant = realtime::issue_microphone_uplink_grant(realtime_handle, room_id.to_owned())
        .await
        .map_err(|error| error.to_string())?;
    let realtime_url = realtime_url().map_err(|error| error.to_string())?;
    let realtime_websocket_url = realtime_websocket_url().map_err(|error| error.to_string())?;
    let realtime_cert_sha256 = realtime_cert_sha256_hex().map_err(|error| error.to_string())?;

    Ok(MicrophoneUplinkConfig {
        access_token,
        grant: grant.grant,
        room_id: grant.room_id,
        realtime_url: realtime_url.to_string(),
        realtime_websocket_url: realtime_websocket_url.to_string(),
        realtime_cert_sha256,
    })
}

fn next_preparation_generation() -> u64 {
    PREPARATION_GENERATION.with(|generation| {
        let next = generation.get().saturating_add(1);
        generation.set(next);
        next
    })
}

fn preparation_is_current(expected: u64) -> bool {
    PREPARATION_GENERATION.with(|generation| generation.get() == expected)
}
