//! Канонические realtime-записи присутствия и точечные мутации комнат.
//!
//! Реестр хранит активные записи как разделяемые `Arc`, а при изменении
//! публикует новые снимки только для затронутых комнат.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use super::presence_index::{MediaRouteSnapshot, PresenceReadIndex};
use super::{VoicePresence, VoicePresenceTarget, VoicePresenceTargetKind};

/// Управляет активным голосовым присутствием и его read indexes.
#[derive(Default)]
pub(super) struct PresenceRegistry {
    /// Каноническое состояние; mutex сериализует все мутации присутствия.
    state: Mutex<PresenceState>,
    /// Синхронные индексы для чтения media route и присутствия по потоку.
    index: PresenceReadIndex,
}

/// Каноническое состояние активных подключений до публикации read snapshots.
#[derive(Default)]
struct PresenceState {
    /// Текущее присутствие каждого realtime-потока.
    by_stream: HashMap<Uuid, Arc<VoicePresence>>,
    /// Единственный активный realtime-поток каждого пользователя.
    stream_by_user: HashMap<Uuid, Uuid>,
    /// Присутствия пользователей, сгруппированные по полным целям комнаты.
    rooms: HashMap<VoicePresenceTarget, HashMap<Uuid, Arc<VoicePresence>>>,
    /// Серверные цели с участниками для точечного удаления сервера.
    server_rooms: HashMap<Uuid, HashSet<VoicePresenceTarget>>,
}

/// Этап публикации снимка комнаты при замене присутствия.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RoomPublication {
    /// Удаление записи из старой комнаты.
    Removal(VoicePresenceTarget),
    /// Добавление записи в новую комнату или замена в той же комнате.
    Addition(VoicePresenceTarget),
}

/// Формирует порядок публикации: все удаления перед единственным добавлением.
pub(super) fn join_publication_plan(
    mut removed_rooms: HashSet<VoicePresenceTarget>,
    added_room: VoicePresenceTarget,
) -> Vec<RoomPublication> {
    removed_rooms.remove(&added_room);
    let mut plan = removed_rooms
        .into_iter()
        .map(RoomPublication::Removal)
        .collect::<Vec<_>>();
    plan.push(RoomPublication::Addition(added_room));
    plan
}

impl PresenceRegistry {
    /// Заменяет присутствие пользователя или потока и возвращает удалённые записи.
    pub(super) async fn join(&self, presence: VoicePresence) -> Vec<VoicePresence> {
        let mut state = self.state.lock().await;
        let mut removed = Vec::with_capacity(2);
        let mut affected_rooms = HashSet::new();
        let mut previous_presences = Vec::with_capacity(2);

        if let Some(previous) = state.by_stream.get(&presence.realtime_stream_id).cloned() {
            previous_presences.push(previous);
        }
        let previous_for_user = state
            .stream_by_user
            .get(&presence.user_id)
            .and_then(|stream_id| state.by_stream.get(stream_id))
            .cloned();

        if let Some(previous) = previous_for_user
            && !previous_presences
                .iter()
                .any(|entry| entry.realtime_stream_id == previous.realtime_stream_id)
        {
            previous_presences.push(previous);
        }

        for previous in &previous_presences {
            // Поточный индекс заранее закрывает старое присутствие для новых читателей.
            self.index.remove_stream(&previous.realtime_stream_id);
        }
        for previous in &previous_presences {
            self.remove_from_state(&mut state, previous, &mut affected_rooms);
            removed.push((**previous).clone());
        }

        let presence = Arc::new(presence);
        self.insert_into_state(&mut state, Arc::clone(&presence));
        for publication in join_publication_plan(affected_rooms, presence.target()) {
            match publication {
                RoomPublication::Removal(target) | RoomPublication::Addition(target) => {
                    self.publish_room(&state, target);
                }
            }
        }
        // Новый stream index появляется только после публикации route:
        // на границе возможна временная ошибка авторизации, но не её расширение.
        self.index.insert_stream(presence);

        removed.sort_by_key(|previous| previous.joined_at);
        removed
    }

    /// Удаляет присутствие одного realtime-потока.
    pub(super) async fn leave_realtime_stream(
        &self,
        realtime_stream_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let mut state = self.state.lock().await;
        let Some(presence) = state.by_stream.get(realtime_stream_id).cloned() else {
            return Vec::new();
        };
        self.remove_and_publish(&mut state, &[presence])
    }

    /// Удаляет присутствие потока, если оно всё ещё относится к указанной комнате.
    pub(super) async fn leave_room(
        &self,
        realtime_stream_id: &Uuid,
        target_kind: VoicePresenceTargetKind,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let mut state = self.state.lock().await;
        let Some(presence) = state.by_stream.get(realtime_stream_id).cloned() else {
            return Vec::new();
        };
        if presence.target_kind != target_kind
            || presence.server_id != *server_id
            || presence.room_id != *room_id
        {
            return Vec::new();
        }
        self.remove_and_publish(&mut state, &[presence])
    }

    /// Удаляет присутствие всех участников комнат указанного сервера.
    pub(super) async fn remove_server(&self, server_id: &Uuid) -> Vec<VoicePresence> {
        let mut state = self.state.lock().await;
        let room_targets = state
            .server_rooms
            .get(server_id)
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let presences = room_targets
            .iter()
            .filter_map(|target| state.rooms.get(target))
            .flat_map(HashMap::values)
            .cloned()
            .collect::<Vec<_>>();
        self.remove_and_publish(&mut state, &presences)
    }

    /// Удаляет присутствие пользователя из одной серверной комнаты.
    pub(super) async fn kick_user_from_room(
        &self,
        user_id: &Uuid,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let target = VoicePresenceTarget {
            kind: VoicePresenceTargetKind::Server,
            server_id: *server_id,
            room_id: *room_id,
        };
        let mut state = self.state.lock().await;
        let Some(presence) = state
            .rooms
            .get(&target)
            .and_then(|users| users.get(user_id))
            .cloned()
        else {
            return Vec::new();
        };
        self.remove_and_publish(&mut state, &[presence])
    }

    /// Перечисляет участников одной комнаты в порядке присоединения.
    pub(super) async fn room_participants(
        &self,
        target_kind: VoicePresenceTargetKind,
        server_id: &Uuid,
        room_id: &Uuid,
    ) -> Vec<VoicePresence> {
        let target = VoicePresenceTarget {
            kind: target_kind,
            server_id: *server_id,
            room_id: *room_id,
        };
        let state = self.state.lock().await;
        let mut participants = state
            .rooms
            .get(&target)
            .into_iter()
            .flat_map(HashMap::values)
            .map(|presence| (**presence).clone())
            .collect::<Vec<_>>();
        participants.sort_by_key(|presence| presence.joined_at);
        participants
    }

    /// Перечисляет серверные комнаты и их участников в стабильном порядке.
    pub(super) async fn server_room_participants(
        &self,
        server_id: &Uuid,
    ) -> Vec<(Uuid, Vec<VoicePresence>)> {
        let state = self.state.lock().await;
        let mut rooms = state
            .server_rooms
            .get(server_id)
            .into_iter()
            .flatten()
            .filter_map(|target| {
                let mut participants = state
                    .rooms
                    .get(target)?
                    .values()
                    .map(|presence| (**presence).clone())
                    .collect::<Vec<_>>();
                participants.sort_by_key(|presence| presence.joined_at);
                Some((target.room_id, participants))
            })
            .collect::<Vec<_>>();
        rooms.sort_by_key(|(room_id, _)| *room_id);
        rooms
    }

    /// Возвращает активное присутствие пользователя в комнате без асинхронной блокировки.
    pub(super) fn room_presence_for_user(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        self.index.for_room_user(target_kind, room_id, user_id)
    }

    /// Возвращает согласованный маршрут для одного медиапакета.
    pub(super) fn media_route(
        &self,
        target_kind: VoicePresenceTargetKind,
        room_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<MediaRouteSnapshot> {
        self.index.media_route(target_kind, room_id, user_id)
    }

    /// Возвращает присутствие потока без асинхронной блокировки.
    ///
    /// Индекс обновляется в fail-closed порядке: удаление публикуется до
    /// изменения room snapshot, добавление — после него. На границе перехода
    /// возможен краткий `None`. При обновлении профиля индекс может кратко
    /// вернуть прежние никнейм или аватар; единственный вызывающий код
    /// использует только неизменную цель присутствия. Медиа-авторизация
    /// использует `media_route`.
    pub(super) fn presence_for_stream(
        &self,
        realtime_stream_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<Arc<VoicePresence>> {
        self.index.for_stream_user(realtime_stream_id, user_id)
    }

    /// Обновляет никнейм присутствующего пользователя и возвращает затронутую цель.
    pub(super) async fn update_user_nickname(
        &self,
        user_id: &Uuid,
        nickname: String,
    ) -> Vec<VoicePresenceTarget> {
        let mut state = self.state.lock().await;
        let Some(stream_id) = state.stream_by_user.get(user_id).copied() else {
            return Vec::new();
        };
        let Some(current) = state.by_stream.get(&stream_id).cloned() else {
            return Vec::new();
        };
        let mut updated = (*current).clone();
        updated.nickname = nickname;
        self.replace_canonical_presence(&mut state, Arc::new(updated));
        vec![current.target()]
    }

    /// Обновляет URL аватара присутствующего пользователя и возвращает затронутую цель.
    pub(super) async fn update_user_avatar(
        &self,
        user_id: &Uuid,
        avatar_url: Option<String>,
    ) -> Vec<VoicePresenceTarget> {
        let mut state = self.state.lock().await;
        let Some(stream_id) = state.stream_by_user.get(user_id).copied() else {
            return Vec::new();
        };
        let Some(current) = state.by_stream.get(&stream_id).cloned() else {
            return Vec::new();
        };
        let mut updated = (*current).clone();
        updated.avatar_url = avatar_url;
        self.replace_canonical_presence(&mut state, Arc::new(updated));
        vec![current.target()]
    }

    /// Возвращает число активных голосовых подключений.
    pub(super) async fn active_voice_connection_count(&self) -> usize {
        self.state.lock().await.by_stream.len()
    }

    fn insert_into_state(&self, state: &mut PresenceState, presence: Arc<VoicePresence>) {
        let stream_id = presence.realtime_stream_id;
        let user_id = presence.user_id;
        let target = presence.target();
        state.by_stream.insert(stream_id, Arc::clone(&presence));
        state.stream_by_user.insert(user_id, stream_id);
        state
            .rooms
            .entry(target)
            .or_default()
            .insert(user_id, Arc::clone(&presence));
        if target.kind == VoicePresenceTargetKind::Server {
            state
                .server_rooms
                .entry(target.server_id)
                .or_default()
                .insert(target);
        }
    }

    fn remove_from_state(
        &self,
        state: &mut PresenceState,
        presence: &Arc<VoicePresence>,
        affected_rooms: &mut HashSet<VoicePresenceTarget>,
    ) {
        let stream_id = presence.realtime_stream_id;
        let user_id = presence.user_id;
        let target = presence.target();
        state.by_stream.remove(&stream_id);
        state.stream_by_user.remove(&user_id);
        if let Some(users) = state.rooms.get_mut(&target) {
            users.remove(&user_id);
            if users.is_empty() {
                state.rooms.remove(&target);
                if target.kind == VoicePresenceTargetKind::Server
                    && let Some(rooms) = state.server_rooms.get_mut(&target.server_id)
                {
                    rooms.remove(&target);
                    if rooms.is_empty() {
                        state.server_rooms.remove(&target.server_id);
                    }
                }
            }
        }
        affected_rooms.insert(target);
    }

    fn remove_and_publish(
        &self,
        state: &mut PresenceState,
        presences: &[Arc<VoicePresence>],
    ) -> Vec<VoicePresence> {
        let mut affected_rooms = HashSet::new();
        let mut removed = Vec::with_capacity(presences.len());
        for presence in presences {
            self.index.remove_stream(&presence.realtime_stream_id);
        }
        for presence in presences {
            self.remove_from_state(state, presence, &mut affected_rooms);
            removed.push((**presence).clone());
        }
        self.publish_rooms(state, affected_rooms);
        removed.sort_by_key(|presence| presence.joined_at);
        removed
    }

    fn publish_rooms(&self, state: &PresenceState, affected_rooms: HashSet<VoicePresenceTarget>) {
        for target in affected_rooms {
            self.publish_room(state, target);
        }
    }

    fn publish_room(&self, state: &PresenceState, target: VoicePresenceTarget) {
        let members = state
            .rooms
            .get(&target)
            .into_iter()
            .flat_map(HashMap::values)
            .cloned()
            .collect::<Vec<_>>();
        self.index
            .replace_room(target.kind, target.room_id, members.into_iter());
    }

    fn replace_canonical_presence(&self, state: &mut PresenceState, presence: Arc<VoicePresence>) {
        state
            .by_stream
            .insert(presence.realtime_stream_id, Arc::clone(&presence));
        state
            .rooms
            .get_mut(&presence.target())
            .expect("active presence must belong to an indexed room")
            .insert(presence.user_id, Arc::clone(&presence));
        self.index.replace_presence(Arc::clone(&presence));
        self.index.insert_stream(presence);
    }
}
