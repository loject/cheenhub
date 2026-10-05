//! Индексы для чтения активного realtime-присутствия в голосовом чате.
//!
//! Снимок каждой комнаты объединяет пользователей и получателей в одну
//! версию. Мутации заменяют только снимки затронутых комнат.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use uuid::Uuid;

use super::{VoicePresence, VoicePresenceTargetKind};

/// Согласованные данные для ретрансляции одного медиапакета.
pub(crate) struct MediaRouteSnapshot {
    /// Каноническое присутствие отправителя в целевой комнате.
    pub(crate) presence: Arc<VoicePresence>,
    /// Неизменяемый список сессий участников из той же версии комнаты.
    pub(crate) recipients: Arc<[Uuid]>,
}

/// Индекс неизменяемых снимков комнат и присутствия realtime-потоков.
#[derive(Default)]
pub(super) struct PresenceReadIndex {
    /// Атомарно публикуемые снимки комнат для синхронного чтения.
    rooms: DashMap<RoomKey, Arc<ArcSwap<RoomPresenceSnapshot>>>,
    /// Индекс текущего присутствия по realtime-потоку.
    by_stream: DashMap<Uuid, Arc<VoicePresence>>,
}

/// Ключ room snapshot в read index.
///
/// Вид цели входит в ключ, чтобы Server и DirectMessage с одинаковым UUID
/// комнаты оставались изолированными.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RoomKey {
    /// Вид цели, к которой относится снимок.
    target_kind: VoicePresenceTargetKind,
    /// UUID комнаты в пределах вида цели.
    room_id: Uuid,
}

/// Неизменяемая версия присутствий и получателей одной комнаты.
///
/// Оба индекса публикуются вместе, чтобы media route брал отправителя и
/// список сессий из одного состояния комнаты.
#[derive(Default)]
struct RoomPresenceSnapshot {
    /// Активное присутствие каждого пользователя комнаты.
    by_user: HashMap<Uuid, Arc<VoicePresence>>,
    /// Сессии участников в порядке присоединения.
    recipients: Arc<[Uuid]>,
}

impl PresenceReadIndex {
    /// Возвращает присутствие и получателей из одного атомарного чтения комнаты.
    pub(super) fn media_route(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<MediaRouteSnapshot> {
        let snapshot = {
            let room_entry = self.rooms.get(&RoomKey {
                target_kind,
                room_id: *room_id,
            })?;
            room_entry.value().load_full()
        };
        let presence = snapshot.by_user.get(user_id)?.clone();
        Some(MediaRouteSnapshot {
            presence,
            recipients: snapshot.recipients.clone(),
        })
    }

    /// Возвращает присутствие из снимка одной комнаты.
    pub(super) fn for_room_user(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        let snapshot = {
            let room_entry = self.rooms.get(&RoomKey {
                target_kind,
                room_id: *room_id,
            })?;
            room_entry.value().load_full()
        };
        snapshot.by_user.get(user_id).cloned()
    }

    /// Возвращает присутствие realtime-потока, если оно принадлежит пользователю.
    pub(super) fn for_stream_user(
        &self,
        realtime_stream_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        self.by_stream
            .get(realtime_stream_id)
            .filter(|presence| presence.user_id == *user_id)
            .map(|presence| Arc::clone(presence.value()))
    }

    /// Публикует новую запись для realtime-потока.
    pub(super) fn insert_stream(&self, presence: Arc<VoicePresence>) {
        self.by_stream.insert(presence.realtime_stream_id, presence);
    }

    /// Удаляет запись realtime-потока из индексированного чтения.
    pub(super) fn remove_stream(&self, realtime_stream_id: &Uuid) {
        self.by_stream.remove(realtime_stream_id);
    }

    /// Обновляет одну запись комнаты, переиспользуя её неизменяемый список сессий.
    pub(super) fn replace_presence(&self, presence: Arc<VoicePresence>) {
        let room_entry = self.rooms.get(&RoomKey {
            target_kind: presence.target_kind,
            room_id: presence.room_id,
        });
        let Some(room_entry) = room_entry else {
            return;
        };
        let room_slot = Arc::clone(room_entry.value());
        drop(room_entry);

        let snapshot = room_slot.load();
        let mut by_user = snapshot.by_user.clone();
        by_user.insert(presence.user_id, presence);
        room_slot.store(Arc::new(RoomPresenceSnapshot {
            by_user,
            recipients: Arc::clone(&snapshot.recipients),
        }));
    }

    /// Атомарно заменяет данные одной комнаты, не перестраивая остальные.
    pub(super) fn replace_room(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: Uuid,
        presences: impl Iterator<Item = Arc<VoicePresence>>,
    ) {
        let key = RoomKey {
            target_kind,
            room_id,
        };
        let mut by_user = HashMap::new();
        let mut recipients = Vec::new();
        for presence in presences {
            by_user.insert(presence.user_id, Arc::clone(&presence));
            recipients.push((presence.joined_at, presence.session_id));
        }
        recipients.sort_unstable_by_key(|(joined_at, session_id)| (*joined_at, *session_id));
        let snapshot = Arc::new(RoomPresenceSnapshot {
            by_user,
            recipients: Arc::from(
                recipients
                    .into_iter()
                    .map(|(_, session_id)| session_id)
                    .collect::<Vec<_>>(),
            ),
        });

        if snapshot.by_user.is_empty() {
            if let Some(slot) = self.rooms.get(&key).map(|entry| Arc::clone(entry.value())) {
                slot.store(snapshot);
                self.rooms.remove(&key);
            }
            return;
        }

        let slot = self
            .rooms
            .entry(key)
            .or_insert_with(|| Arc::new(ArcSwap::from_pointee(RoomPresenceSnapshot::default())))
            .value()
            .clone();
        slot.store(snapshot);
    }
}
