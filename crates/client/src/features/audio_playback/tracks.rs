//! Разделение playback по дорожкам и отсечение поздних пакетов заменённого захвата.

use std::collections::{HashMap, VecDeque};

use cheenhub_contracts::media::MediaAudioSource;
use uuid::Uuid;

use super::backend::VoiceFrame;

const RETIRED_TRACK_LIMIT: usize = 32;

/// Текущие дорожки одного назначения у каждого отправителя.
///
/// Число декодеров ограничено двумя на пользователя. Короткая история UUID
/// не даёт поздним пакетам недавних захватов заменить текущий декодер.
#[derive(Default)]
pub(super) struct PlaybackTracks {
    slots: HashMap<(String, MediaAudioSource), TrackSlot>,
}

#[derive(Default)]
struct TrackSlot {
    active: Option<(Uuid, u8)>,
    retired: VecDeque<Uuid>,
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
        let slot = self
            .slots
            .entry((frame.sender_user_id.clone(), frame.audio_source))
            .or_default();
        if slot.retired.contains(&frame.track_id) {
            return None;
        }
        let key = playback_key(&frame.sender_user_id, frame.audio_source, frame.track_id);
        if let Some((id, channels)) = slot.active {
            if id == frame.track_id {
                return (channels == frame.audio_channels).then_some(TrackAdmission {
                    key,
                    replaced: None,
                });
            }
            let replaced = playback_key(&frame.sender_user_id, frame.audio_source, id);
            retire(slot, id);
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
        track_id: Uuid,
    ) -> Option<String> {
        let slot = self.slots.entry((sender.to_owned(), source)).or_default();
        retire(slot, track_id);
        if slot.active.is_some_and(|(id, _)| id == track_id) {
            slot.active = None;
            Some(playback_key(sender, source, track_id))
        } else {
            None
        }
    }

    /// Удаляет слоты ушедшего пользователя, освобождая историю его захватов.
    pub(super) fn remove_sender(&mut self, sender: &str) {
        self.slots.retain(|(owner, _), _| owner != sender);
    }

    /// Очищает слоты при выходе из комнаты или выключении воспроизведения.
    pub(super) fn clear(&mut self) {
        self.slots.clear();
    }
}

fn retire(slot: &mut TrackSlot, id: Uuid) {
    if slot.retired.contains(&id) {
        return;
    }
    if slot.retired.len() == RETIRED_TRACK_LIMIT {
        slot.retired.pop_front();
    }
    slot.retired.push_back(id);
}

/// Формирует внутренний ключ очереди из владельца, назначения и UUID захвата.
pub(super) fn playback_key(sender: &str, source: MediaAudioSource, id: Uuid) -> String {
    format!("{sender}/{}/{id}", source as u8)
}

/// Возвращает владельца внутреннего ключа очереди для настроек громкости пользователя.
pub(super) fn playback_owner(key: &str) -> &str {
    key.split_once('/').map_or(key, |(owner, _)| owner)
}

#[cfg(test)]
mod tests;
