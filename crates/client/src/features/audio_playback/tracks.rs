//! Разделение playback по дорожкам и отсечение поздних пакетов заменённого захвата.

use std::collections::{HashMap, VecDeque};

use cheenhub_contracts::media::MediaAudioSource;
use uuid::Uuid;

use super::backend::VoiceFrame;

const RETIRED_TRACK_LIMIT: usize = 32;

/// Активные дорожки захватов и независимо подключённых голосовых сессий.
///
/// Legacy-захваты сохраняют историю замен, а проверенные микрофонные сессии
/// одного пользователя получают независимые jitter buffers и декодеры.
#[derive(Default)]
pub(super) struct PlaybackTracks {
    slots: HashMap<(String, MediaAudioSource, Option<Uuid>), TrackSlot>,
    session_microphones: HashMap<(String, Uuid), u8>,
}

#[derive(Default)]
struct TrackSlot {
    active: Option<(Uuid, u8)>,
    retired: VecDeque<Uuid>,
    /// Явно завершённые голосовые сессии не возобновляются после позднего snapshot.
    stopped_sessions: VecDeque<Uuid>,
}

/// Ключ принятой дорожки и ключ заменённого декодера для освобождения ресурсов.
pub(super) struct TrackAdmission {
    /// Ключ всех очередей и декодера принятого кадра.
    pub(super) key: String,
    /// Старый ключ того же назначения, который нужно остановить.
    pub(super) replaced: Option<String>,
}

impl PlaybackTracks {
    /// Принимает новый захват либо продолжение текущего; отклоняет старый UUID и смену формата.
    pub(super) fn admit(&mut self, frame: &VoiceFrame) -> Option<TrackAdmission> {
        if !matches!(frame.audio_channels, 1 | 2)
            || (frame.track_id.is_nil() && frame.audio_source == MediaAudioSource::System)
        {
            return None;
        }
        if frame.audio_source == MediaAudioSource::Microphone && frame.session_track {
            if frame.track_id.is_nil()
                || self
                    .slots
                    .get(&(frame.sender_user_id.clone(), frame.audio_source, None))
                    .is_some_and(|slot| slot.stopped_sessions.contains(&frame.track_id))
            {
                return None;
            }
            let channels = self
                .session_microphones
                .entry((frame.sender_user_id.clone(), frame.track_id))
                .or_insert(frame.audio_channels);
            return (*channels == frame.audio_channels).then_some(TrackAdmission {
                key: playback_key_for(
                    &frame.sender_user_id,
                    frame.audio_source,
                    frame.voice_session_id,
                    frame.track_id,
                ),
                replaced: None,
            });
        }
        let session_id = (frame.audio_source == MediaAudioSource::System)
            .then_some(frame.voice_session_id)
            .flatten();
        let slot = self
            .slots
            .entry((frame.sender_user_id.clone(), frame.audio_source, session_id))
            .or_default();
        if slot.retired.contains(&frame.track_id) {
            return None;
        }
        let key = playback_key_for(
            &frame.sender_user_id,
            frame.audio_source,
            session_id,
            frame.track_id,
        );
        if let Some((id, channels)) = slot.active {
            if id == frame.track_id {
                return (channels == frame.audio_channels).then_some(TrackAdmission {
                    key,
                    replaced: None,
                });
            }
            let replaced =
                playback_key_for(&frame.sender_user_id, frame.audio_source, session_id, id);
            retire(&mut slot.retired, id);
            slot.active = Some((frame.track_id, frame.audio_channels));
            return Some(TrackAdmission {
                key,
                replaced: Some(replaced),
            });
        }
        slot.active = Some((frame.track_id, frame.audio_channels));
        Some(TrackAdmission {
            key,
            replaced: None,
        })
    }

    /// Закрывает конкретный UUID, в том числе до первого пакета, не затрагивая новый захват.
    pub(super) fn stop_source(
        &mut self,
        sender: &str,
        source: MediaAudioSource,
        voice_session_id: Option<Uuid>,
        track_id: Uuid,
    ) -> Option<String> {
        let voice_session_id = (source == MediaAudioSource::System)
            .then_some(voice_session_id)
            .flatten();
        let slot = self
            .slots
            .entry((sender.to_owned(), source, voice_session_id))
            .or_default();
        retire(&mut slot.retired, track_id);
        if slot.active.is_some_and(|(id, _)| id == track_id) {
            slot.active = None;
            Some(playback_key_for(sender, source, voice_session_id, track_id))
        } else {
            None
        }
    }

    /// Останавливает одну подтверждённую микрофонную сессию, отсекая поздние пакеты.
    pub(super) fn stop_microphone_session(
        &mut self,
        sender: &str,
        session_id: Uuid,
    ) -> Option<String> {
        let present = self
            .session_microphones
            .remove(&(sender.to_owned(), session_id));
        let slot = self
            .slots
            .entry((sender.to_owned(), MediaAudioSource::Microphone, None))
            .or_default();
        let was_legacy_active = slot.active.is_some_and(|(id, _)| id == session_id);
        if was_legacy_active {
            slot.active = None;
        }
        retire(&mut slot.retired, session_id);
        retire(&mut slot.stopped_sessions, session_id);
        (present.is_some() || was_legacy_active)
            .then(|| playback_key(sender, MediaAudioSource::Microphone, session_id))
    }

    /// Разрешает новую публикацию прежней голосовой сессии после подтверждённого входа.
    ///
    /// Удаляет запрет поздних пакетов только для указанного UUID. Состояние
    /// декодера освобождает владеющий им AudioPlaybackHandle.
    pub(super) fn reactivate_microphone_session(&mut self, sender: &str, session_id: Uuid) {
        self.session_microphones
            .remove(&(sender.to_owned(), session_id));
        let slot = self
            .slots
            .entry((sender.to_owned(), MediaAudioSource::Microphone, None))
            .or_default();
        if slot.active.is_some_and(|(id, _)| id == session_id) {
            slot.active = None;
        }
        slot.retired.retain(|id| *id != session_id);
        slot.stopped_sessions.retain(|id| *id != session_id);
    }

    /// Удаляет слоты ушедшего пользователя, освобождая историю его захватов.
    pub(super) fn remove_sender(&mut self, sender: &str) {
        self.slots.retain(|(owner, _, _), _| owner != sender);
        self.session_microphones
            .retain(|(owner, _), _| owner != sender);
    }

    /// Очищает слоты при выходе из комнаты или выключении воспроизведения.
    pub(super) fn clear(&mut self) {
        self.slots.clear();
        self.session_microphones.clear();
    }
}

fn retire(retired: &mut VecDeque<Uuid>, id: Uuid) {
    if retired.contains(&id) {
        return;
    }
    if retired.len() == RETIRED_TRACK_LIMIT {
        retired.pop_front();
    }
    retired.push_back(id);
}

/// Формирует внутренний ключ очереди из владельца, назначения и UUID захвата.
pub(super) fn playback_key(sender: &str, source: MediaAudioSource, id: Uuid) -> String {
    format!("{sender}/{}/{id}", source as u8)
}

/// Отделяет системный звук двух устройств даже при одинаковом UUID захвата.
pub(super) fn playback_key_for(
    sender: &str,
    source: MediaAudioSource,
    voice_session_id: Option<Uuid>,
    track_id: Uuid,
) -> String {
    if source == MediaAudioSource::System
        && let Some(session_id) = voice_session_id
    {
        format!("{sender}/{}/{session_id}/{track_id}", source as u8)
    } else {
        playback_key(sender, source, track_id)
    }
}

/// Возвращает владельца внутреннего ключа очереди для настроек громкости пользователя.
pub(super) fn playback_owner(key: &str) -> &str {
    key.split_once('/').map_or(key, |(owner, _)| owner)
}

#[cfg(test)]
mod tests;
