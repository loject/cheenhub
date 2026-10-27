//! Приём аудиодорожек активной комнаты и индикатор речи только для микрофона.

use dioxus::prelude::*;
use futures_util::StreamExt;

use super::realtime;
use super::state::VoiceConnectionHandle;
use crate::features::audio_playback::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};
use crate::features::realtime::RealtimeHandle;

/// Подписывает playback на аудио активной комнаты с учётом фокуса native-звонка.
pub(super) fn use_inbound_audio(
    realtime: RealtimeHandle,
    playback: AudioPlaybackHandle,
    current_user_id: String,
    handle: VoiceConnectionHandle,
    voice_audio_focused: Signal<bool>,
) {
    use_hook(move || {
        spawn(async move {
            let mut frames = realtime::subscribe_voice_frames(&realtime);
            while let Some(frame) = frames.next().await {
                if !voice_audio_focused() {
                    continue;
                }
                let current = handle.state();
                let Some(target) = current.active_target() else {
                    continue;
                };
                if frame.room_id != target.room_id || frame.sender_user_id == current_user_id {
                    continue;
                }
                if frame.audio_source == cheenhub_contracts::media::MediaAudioSource::Microphone {
                    handle.mark_user_speaking(frame.sender_user_id.clone());
                }
                playback.play_voice_frame(VoiceFrame {
                    sender_user_id: frame.sender_user_id,
                    track_id: frame.track_id,
                    audio_source: frame.audio_source,
                    audio_channels: frame.audio_channels,
                    sequence: frame.sequence,
                    timestamp_us: frame.timestamp_us,
                    duration_us: frame.duration_us,
                    codec: PlaybackCodec::Opus,
                    bytes: frame.bytes,
                });
            }
        })
    });
}
