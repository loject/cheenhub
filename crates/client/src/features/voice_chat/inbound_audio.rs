//! Приём аудиодорожек активной комнаты и индикатор речи только для микрофона.

use cheenhub_contracts::realtime::VoiceRoomParticipant;
use dioxus::prelude::*;
use futures_util::StreamExt;
use uuid::Uuid;

use super::realtime;
use super::state::VoiceConnectionHandle;
use crate::features::audio_playback::{AudioPlaybackHandle, PlaybackCodec, VoiceFrame};
use crate::features::realtime::RealtimeHandle;

/// Изменения подтверждённых микрофонных сессий при обновлении состава участников.
struct AudioSessionChanges {
    stopped: Vec<(String, Uuid)>,
    restarted: Vec<(String, Uuid)>,
}

/// Находит ушедшие и заново присоединившиеся голосовые сессии.
///
/// Изменение времени входа при прежнем UUID означает новую публикацию даже
/// без промежуточного снимка с отсутствующим участником.
fn audio_session_changes(
    previous: &[VoiceRoomParticipant],
    next: &[VoiceRoomParticipant],
) -> AudioSessionChanges {
    let same_presence = |first: &VoiceRoomParticipant, second: &VoiceRoomParticipant| {
        first.user_id == second.user_id
            && first.session_id == second.session_id
            && first.joined_at == second.joined_at
    };
    let stopped = previous
        .iter()
        .filter(|participant| !next.iter().any(|other| same_presence(participant, other)))
        .filter_map(|participant| {
            Uuid::parse_str(&participant.session_id)
                .ok()
                .map(|session_id| (participant.user_id.clone(), session_id))
        })
        .collect();
    let restarted = next
        .iter()
        .filter(|participant| {
            !previous
                .iter()
                .any(|other| same_presence(participant, other))
        })
        .filter_map(|participant| {
            Uuid::parse_str(&participant.session_id)
                .ok()
                .map(|session_id| (participant.user_id.clone(), session_id))
        })
        .collect();
    AudioSessionChanges { stopped, restarted }
}

/// Синхронизирует аудио с подтверждённым присутствием, не затрагивая соседние сессии.
pub(super) fn reconcile_audio_sessions(
    playback: &AudioPlaybackHandle,
    previous: &[VoiceRoomParticipant],
    next: &[VoiceRoomParticipant],
) {
    let AudioSessionChanges { stopped, restarted } = audio_session_changes(previous, next);
    for (user_id, session_id) in stopped {
        playback.stop_microphone_session(&user_id, session_id);
        debug!(%user_id, %session_id, "микрофонная сессия остановлена после обновления участников");
    }
    for (user_id, session_id) in restarted {
        playback.reactivate_microphone_session(&user_id, session_id);
        debug!(%user_id, %session_id, "микрофонная сессия повторно активирована после обновления участников");
    }
}

/// Определяет независимую дорожку микрофона по подтверждённой сессии участника.
///
/// Сервер нормализует track ID старых отправителей, поэтому версия их
/// клиентского протокола не влияет на разделение устройств одного аккаунта.
fn is_session_microphone_frame(
    participants: &[VoiceRoomParticipant],
    sender_user_id: &str,
    track_id: Uuid,
) -> bool {
    let session_id = track_id.to_string();
    participants.iter().any(|participant| {
        participant.user_id == sender_user_id && participant.session_id == session_id
    })
}

/// Отсекает собственный микрофон, не блокируя другие устройства того же аккаунта.
fn is_own_microphone_frame(
    sender_user_id: &str,
    current_user_id: &str,
    audio_source: cheenhub_contracts::media::MediaAudioSource,
    track_id: Uuid,
    own_session_id: Option<Uuid>,
) -> bool {
    sender_user_id == current_user_id
        && audio_source == cheenhub_contracts::media::MediaAudioSource::Microphone
        && own_session_id == Some(track_id)
}

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
                if frame.room_id != target.room_id
                    || is_own_microphone_frame(
                        &frame.sender_user_id,
                        &current_user_id,
                        frame.audio_source,
                        frame.track_id,
                        handle.outbound_session_id_for(&target.server_id, &target.room_id),
                    )
                {
                    continue;
                }
                let microphone =
                    frame.audio_source == cheenhub_contracts::media::MediaAudioSource::Microphone;
                let session_track = microphone
                    && is_session_microphone_frame(
                        current.participants(),
                        &frame.sender_user_id,
                        frame.track_id,
                    );
                if microphone {
                    handle.mark_user_speaking(
                        frame.sender_user_id.clone(),
                        session_track.then_some(frame.track_id),
                    );
                }
                playback.play_voice_frame(VoiceFrame {
                    sender_user_id: frame.sender_user_id,
                    track_id: frame.track_id,
                    voice_session_id: frame.voice_session_id,
                    session_track,
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

#[cfg(test)]
mod tests;
