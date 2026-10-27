//! Передача ограниченной очереди системного звука в комнату исходного захвата.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::realtime::RealtimeHandle;
use crate::features::screen_share::ScreenShareHandle;

use super::state::VoiceConnectionHandle;

/// Привязывает очередь к сессии захвата; микрофон и его mute не участвуют в передаче.
pub(super) fn use_screen_audio(
    screen: ScreenShareHandle,
    realtime: RealtimeHandle,
    voice: VoiceConnectionHandle,
) {
    use_resource(move || {
        let frames = screen.take_audio_frames();
        let target = voice.state.peek().active_target();
        let screen = screen.clone();
        let realtime = realtime.clone();
        let voice = voice.clone();
        async move {
            let (Some(mut frames), Some(target)) = (frames, target) else {
                return;
            };
            debug!(room_id = %target.room_id, "started screen audio sender");
            let mut last_warning = None::<web_time::Instant>;
            while let Some(frame) = frames.next().await {
                if !frame.can_send() || !screen.audio_track_is_active(frame.track_id) {
                    continue;
                }
                let still_in_room = voice.state.peek().active_target().is_some_and(|active| {
                    active.room_id == target.room_id
                        && active.server_id == target.server_id
                        && active.kind == target.kind
                });
                if !still_in_room {
                    break;
                }
                let track_id = frame.track_id;
                if let Err(error) =
                    super::realtime::send_screen_audio_frame(&realtime, &target.room_id, frame)
                        .await
                    && last_warning.is_none_or(|last| last.elapsed().as_secs() >= 5)
                {
                    warn!(%error, %track_id, room_id = %target.room_id, "screen audio send failed");
                    last_warning = Some(web_time::Instant::now());
                }
            }
            debug!(room_id = %target.room_id, "screen audio sender stopped");
        }
    });
}
