//! Подписки на входящие видеодатаграммы и завершение медиапотоков.

use dioxus::prelude::*;
use futures_util::StreamExt;

use crate::features::audio_playback::AudioPlaybackHandle;
use crate::features::realtime::RealtimeHandle;

use super::local_video::participant_source_from_contract;
use super::realtime;
use super::state::VoiceConnectionState;
use super::video_streams::{ParticipantVideoHandle, ParticipantVideoSource};

/// Подключает обработчики кадров камеры, демонстрации экрана и завершения видео.
///
/// Кадры принимаются только для текущей комнаты; завершение потока дополнительно
/// освобождает связанную системную аудиодорожку, если она была опубликована.
pub(super) fn use_video_datagram_runtime(
    state: Signal<VoiceConnectionState>,
    realtime: RealtimeHandle,
    participant_video: ParticipantVideoHandle,
    playback: AudioPlaybackHandle,
) {
    let screen_datagram_realtime = realtime.clone();
    let screen_datagram_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_screen_frames(&screen_datagram_realtime);
            while let Some(frame) = frames.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id {
                    continue;
                }
                screen_datagram_video
                    .publish_frame(ParticipantVideoSource::ScreenShare, frame.into());
            }
        })
    });

    let camera_datagram_realtime = realtime.clone();
    let camera_datagram_video = participant_video.clone();
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_camera_frames(&camera_datagram_realtime);
            while let Some(frame) = frames.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id {
                    continue;
                }
                camera_datagram_video.publish_frame(ParticipantVideoSource::Camera, frame.into());
            }
        })
    });

    use_hook(move || {
        spawn(async move {
            let mut events = realtime::subscribe_video_stream_ended(&realtime);
            while let Some(event) = events.next().await {
                let current = state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if event.server_id != target.server_id || event.room_id != target.room_id {
                    continue;
                }
                participant_video.release_stream_for_event(
                    participant_source_from_contract(event.source),
                    &event.sender_user_id,
                    event.session_id,
                );
                if let Some(track_id) = event.audio_track_id {
                    playback.stop_system_audio(&event.sender_user_id, event.session_id, track_id);
                }
            }
        })
    });
}
