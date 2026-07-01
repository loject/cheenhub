//! Отправка исходящих голосовых фреймов.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::{debug, spawn, warn};
use futures_channel::mpsc;
use futures_util::StreamExt;
use web_time::{Instant, SystemTime, UNIX_EPOCH};

use crate::features::microphone::{EncodedMicrophoneFrame, MicrophoneFrameCallback};
use crate::features::realtime::RealtimeHandle;

use super::realtime;

const SLOW_VOICE_FRAME_SEND_WARN_AFTER: Duration = Duration::from_millis(40);
const VOICE_FRAME_SEND_WARNING_INTERVAL_MS: u64 = 5_000;

/// Создает callback, который отправляет аудиофреймы через одну долгоживущую задачу.
pub(super) fn voice_frame_sender_callback(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) -> MicrophoneFrameCallback {
    let (sender, receiver) = mpsc::unbounded();
    spawn_voice_frame_sender(receiver, realtime, server_id.clone(), room_id.clone());

    Rc::new(move |frame| {
        if sender.unbounded_send(frame).is_err() {
            debug!(
                server_id = %server_id,
                room_id = %room_id,
                "dropped encoded voice frame because sender task is closed"
            );
        }
    })
}

fn spawn_voice_frame_sender(
    mut frames: mpsc::UnboundedReceiver<EncodedMicrophoneFrame>,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    spawn(async move {
        let last_slow_send_warning_ms = Cell::new(0);
        while let Some(frame) = frames.next().await {
            let frame_sequence = frame.sequence;
            let frame_duration_us = frame.duration_us;
            let payload_bytes = frame.bytes.len();
            let started_at = Instant::now();
            let result = realtime::send_voice_frame(&realtime, &server_id, &room_id, frame).await;
            let elapsed = started_at.elapsed();
            if elapsed >= SLOW_VOICE_FRAME_SEND_WARN_AFTER
                && should_emit_voice_frame_warning(
                    &last_slow_send_warning_ms,
                    voice_frame_sender_now_ms(),
                )
            {
                warn!(
                    server_id = %server_id,
                    room_id = %room_id,
                    sequence = frame_sequence,
                    duration_us = frame_duration_us,
                    payload_bytes,
                    elapsed_ms = elapsed.as_millis(),
                    "slow outbound voice frame send"
                );
            }
            if let Err(error) = result {
                warn!(
                    %error,
                    server_id = %server_id,
                    room_id = %room_id,
                    sequence = frame_sequence,
                    "failed to send encoded voice frame"
                );
            }
        }
        debug!(
            server_id = %server_id,
            room_id = %room_id,
            "voice frame sender stopped"
        );
    });
}

fn should_emit_voice_frame_warning(last_warning_ms: &Cell<u64>, now_ms: u64) -> bool {
    let last_ms = last_warning_ms.get();
    if last_ms != 0 && now_ms.saturating_sub(last_ms) < VOICE_FRAME_SEND_WARNING_INTERVAL_MS {
        return false;
    }

    last_warning_ms.set(now_ms);
    true
}

fn voice_frame_sender_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
