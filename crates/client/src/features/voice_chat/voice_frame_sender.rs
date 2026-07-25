//! Отправка исходящих голосовых фреймов.

use std::cell::{Cell, RefCell};
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

#[derive(Default)]
struct PendingVoiceFrame {
    frame: Option<EncodedMicrophoneFrame>,
    dropped_frames: u64,
    first_dropped_sequence: Option<u64>,
    last_dropped_sequence: Option<u64>,
}

struct NextVoiceFrame {
    frame: EncodedMicrophoneFrame,
    dropped_frames: u64,
    first_dropped_sequence: Option<u64>,
    last_dropped_sequence: Option<u64>,
}

impl PendingVoiceFrame {
    fn replace(&mut self, frame: EncodedMicrophoneFrame) {
        if let Some(stale_frame) = self.frame.replace(frame) {
            self.dropped_frames = self.dropped_frames.saturating_add(1);
            self.first_dropped_sequence
                .get_or_insert(stale_frame.sequence);
            self.last_dropped_sequence = Some(stale_frame.sequence);
        }
    }

    fn take(&mut self) -> Option<NextVoiceFrame> {
        Some(NextVoiceFrame {
            frame: self.frame.take()?,
            dropped_frames: std::mem::take(&mut self.dropped_frames),
            first_dropped_sequence: self.first_dropped_sequence.take(),
            last_dropped_sequence: self.last_dropped_sequence.take(),
        })
    }
}

/// Создает callback, который отправляет аудиофреймы через одну долгоживущую задачу.
pub(super) fn voice_frame_sender_callback(
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) -> MicrophoneFrameCallback {
    let pending = Rc::new(RefCell::new(PendingVoiceFrame::default()));
    let (wake_sender, wake_receiver) = mpsc::channel(1);
    spawn_voice_frame_sender(
        wake_receiver,
        Rc::clone(&pending),
        realtime,
        server_id.clone(),
        room_id.clone(),
    );
    let wake_sender = RefCell::new(wake_sender);
    let sender_closed_logged = Cell::new(false);

    Rc::new(move |frame| {
        pending.borrow_mut().replace(frame);
        let wake_result = wake_sender.borrow_mut().try_send(());
        if wake_result
            .as_ref()
            .is_err_and(|error| error.is_disconnected())
            && !sender_closed_logged.replace(true)
        {
            debug!(
                server_id = %server_id,
                room_id = %room_id,
                "dropped encoded voice frame because sender task is closed"
            );
        }
    })
}

fn spawn_voice_frame_sender(
    mut wake_receiver: mpsc::Receiver<()>,
    pending: Rc<RefCell<PendingVoiceFrame>>,
    realtime: RealtimeHandle,
    server_id: String,
    room_id: String,
) {
    spawn(async move {
        let last_slow_send_warning_ms = Cell::new(0);
        let last_drop_warning_ms = Cell::new(0);
        while wake_receiver.next().await.is_some() {
            let Some(next_frame) = pending.borrow_mut().take() else {
                continue;
            };
            if next_frame.dropped_frames > 0
                && should_emit_voice_frame_warning(
                    &last_drop_warning_ms,
                    voice_frame_sender_now_ms(),
                )
            {
                warn!(
                    server_id = %server_id,
                    room_id = %room_id,
                    dropped_frames = next_frame.dropped_frames,
                    first_dropped_sequence = ?next_frame.first_dropped_sequence,
                    last_dropped_sequence = ?next_frame.last_dropped_sequence,
                    next_sequence = next_frame.frame.sequence,
                    "dropped stale outbound voice frames while network send was congested"
                );
            }
            let frame = next_frame.frame;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::microphone::MicrophoneCodec;

    #[test]
    fn pending_voice_frame_keeps_only_latest_frame() {
        let mut pending = PendingVoiceFrame::default();

        pending.replace(frame(10));
        pending.replace(frame(11));
        pending.replace(frame(12));

        let next = pending.take().expect("ожидался последний голосовой фрейм");
        assert_eq!(next.frame.sequence, 12);
        assert_eq!(next.dropped_frames, 2);
        assert_eq!(next.first_dropped_sequence, Some(10));
        assert_eq!(next.last_dropped_sequence, Some(11));
    }

    #[test]
    fn pending_voice_frame_resets_drop_summary_after_take() {
        let mut pending = PendingVoiceFrame::default();
        pending.replace(frame(20));
        pending.replace(frame(21));
        let _ = pending.take();

        pending.replace(frame(22));

        let next = pending.take().expect("ожидался новый голосовой фрейм");
        assert_eq!(next.frame.sequence, 22);
        assert_eq!(next.dropped_frames, 0);
        assert_eq!(next.first_dropped_sequence, None);
        assert_eq!(next.last_dropped_sequence, None);
    }

    fn frame(sequence: u64) -> EncodedMicrophoneFrame {
        EncodedMicrophoneFrame {
            sequence,
            timestamp_us: sequence * 20_000,
            duration_us: 20_000,
            codec: MicrophoneCodec::Opus,
            sample_rate_hz: 48_000,
            channels: 1,
            bytes: vec![1, 2, 3],
        }
    }
}
