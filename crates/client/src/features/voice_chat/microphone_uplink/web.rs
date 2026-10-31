//! Dedicated Worker uplink микрофона для браузера.

use std::cell::Cell;

use dioxus::prelude::{debug, info, spawn, warn};

use crate::features::auth::api;
use crate::features::microphone::{MicrophoneHandle, MicrophoneUplinkConfig};
use crate::features::realtime::{
    RealtimeHandle, realtime_cert_sha256_hex, realtime_url, realtime_websocket_url,
};

use super::super::state::VoiceConnectionHandle;
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
    voice: VoiceConnectionHandle,
) {
    let preparation_generation = next_preparation_generation();
    let microphone_generation = microphone.operation_generation();
    spawn(async move {
        let uplink = match prepare_uplink(&realtime_handle, &server_id, &room_id, &voice).await {
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
            || voice
                .outbound_session_id_for(&server_id, &room_id)
                .is_none_or(|session_id| session_id.to_string() != uplink.sender_session_id)
        {
            debug!(%room_id, "discarded stale dedicated microphone uplink grant");
            return;
        }

        let fallback = voice_frame_sender::voice_frame_sender_callback(
            realtime_handle,
            server_id,
            room_id.clone(),
            voice,
        );
        info!(%room_id, "starting dedicated microphone worker uplink");
        microphone.restart_with_uplink(fallback, uplink);
    });
}

async fn prepare_uplink(
    realtime_handle: &RealtimeHandle,
    server_id: &str,
    room_id: &str,
    voice: &VoiceConnectionHandle,
) -> Result<MicrophoneUplinkConfig, String> {
    let initial_session = voice
        .outbound_session_id_for(server_id, room_id)
        .ok_or_else(|| "Основное голосовое подключение недоступно.".to_owned())?;
    let access_token = api::fresh_access_token().await?;
    let grant = realtime::issue_microphone_uplink_grant(realtime_handle, room_id.to_owned())
        .await
        .map_err(|error| error.to_string())?;
    let realtime_url = realtime_url().map_err(|error| error.to_string())?;
    let realtime_websocket_url = realtime_websocket_url().map_err(|error| error.to_string())?;
    let realtime_cert_sha256 = realtime_cert_sha256_hex().map_err(|error| error.to_string())?;
    let sender_session_id = voice
        .outbound_session_id_for(server_id, room_id)
        .filter(|session_id| *session_id == initial_session)
        .ok_or_else(|| "Основное голосовое подключение сменилось.".to_owned())?;

    Ok(MicrophoneUplinkConfig {
        access_token,
        grant: grant.grant,
        room_id: grant.room_id,
        sender_session_id: sender_session_id.to_string(),
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
