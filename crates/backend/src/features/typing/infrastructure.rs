//! Оперативное хранилище состояния набора сообщения.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;
use uuid::Uuid;

/// Время жизни записи набора без продления от клиента.
///
/// Клиент продлевает набор чаще этого интервала, поэтому истекшая запись означает,
/// что вкладка закрылась или потеряла связь, не отправив финальное событие.
pub(crate) const TYPING_TTL: Duration = Duration::from_secs(8);

/// Минимальный интервал между двумя продлениями одного потока realtime.
///
/// Клиент шлет продление на каждый ввод, поэтому сервер ограничивает частоту,
/// чтобы один активный пользователь не создавал лишнюю нагрузку на рассылку.
const TYPING_REFRESH_INTERVAL: Duration = Duration::from_secs(2);

/// Тип цели, в которой пользователь печатает сообщение.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TypingTargetKind {
    /// Комната серверного чата.
    Room,
    /// Личный диалог с другим пользователем.
    DirectMessage,
}

/// Цель набора сообщения.
///
/// Для личного диалога в `server_id` и `room_id` записывается один и тот же
/// идентификатор диалога: это позволяет переиспользовать ключ вещания, который
/// ожидает `RealtimeHub::fanout_to_streams`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TypingTarget {
    /// Тип цели.
    pub(crate) kind: TypingTargetKind,
    /// Идентификатор сервера или личного диалога.
    pub(crate) server_id: Uuid,
    /// Идентификатор комнаты или личного диалога.
    pub(crate) room_id: Uuid,
}

impl TypingTarget {
    /// Возвращает идентификатор, по которому цель попадает в маршрут вещания.
    pub(crate) fn route_id(self) -> Uuid {
        self.server_id
    }

    /// Создает цель набора в комнате серверного чата.
    pub(crate) fn room(server_id: Uuid, room_id: Uuid) -> Self {
        Self {
            kind: TypingTargetKind::Room,
            server_id,
            room_id,
        }
    }

    /// Создает цель набора в личном диалоге.
    pub(crate) fn direct_message(conversation_id: Uuid) -> Self {
        Self {
            kind: TypingTargetKind::DirectMessage,
            server_id: conversation_id,
            room_id: conversation_id,
        }
    }
}

/// Активная запись о том, что пользователь печатает сообщение.
#[derive(Debug, Clone)]
pub(crate) struct TypingAuthorEntry {
    /// Цель, в которой пользователь печатает.
    pub(crate) target: TypingTarget,
    /// Пользователь, который печатает.
    pub(crate) user_id: Uuid,
    /// Снимок ника пользователя на момент последнего продления.
    pub(crate) nickname: String,
    /// Снимок публичного URL аватара пользователя.
    pub(crate) avatar_url: Option<String>,
    /// Поток realtime, отправивший последнее продление.
    ///
    /// Используется для очистки при закрытии вкладки и для защиты от подмены
    /// чужого состояния: продлить или снять набор может только владелец потока.
    pub(crate) realtime_stream_id: Uuid,
    /// Момент последнего продления, после которого запись истекает.
    pub(crate) refreshed_at: Instant,
}

/// Оперативное хранилище состояния набора для активных потоков realtime.
#[derive(Default)]
pub(crate) struct InMemoryTypingStore {
    entries: Mutex<Vec<TypingAuthorEntry>>,
    last_refresh_at: Mutex<Vec<(Uuid, Instant)>>,
}

impl InMemoryTypingStore {
    /// Регистрирует начало набора или продлевает уже начатый.
    ///
    /// Возвращает `true`, если состояние набора изменилось и его нужно разослать
    /// участникам, и `false`, если это было только продление уже известного
    /// набора. Продление чаще [`TYPING_REFRESH_INTERVAL`] отбрасывается, чтобы
    /// ввод пользователя не превращался в поток событий.
    pub(crate) async fn start(&self, entry: TypingAuthorEntry) -> bool {
        let now = Instant::now();
        let mut entries = self.entries.lock().await;
        retain_fresh(&mut entries, now);

        let refresh_allowed = self.allow_refresh(entry.realtime_stream_id, now).await;

        match entries
            .iter_mut()
            .find(|existing| existing.target == entry.target && existing.user_id == entry.user_id)
        {
            Some(existing) => {
                existing.nickname = entry.nickname;
                existing.avatar_url = entry.avatar_url;
                if refresh_allowed {
                    existing.refreshed_at = now;
                    // Продлевать и снимать набор может только владелец потока, иначе
                    // вторая вкладка того же пользователя вытеснила бы первую.
                    existing.realtime_stream_id = entry.realtime_stream_id;
                }
                false
            }
            None => {
                entries.push(entry);
                true
            }
        }
    }

    /// Снимает набор пользователя в цели и возвращает снятую запись, если она была.
    pub(crate) async fn stop(
        &self,
        target: TypingTarget,
        user_id: Uuid,
        realtime_stream_id: Uuid,
    ) -> Option<TypingAuthorEntry> {
        let now = Instant::now();
        let mut entries = self.entries.lock().await;
        retain_fresh(&mut entries, now);
        let position = entries.iter().position(|existing| {
            existing.target == target
                && existing.user_id == user_id
                && existing.realtime_stream_id == realtime_stream_id
        })?;
        Some(entries.remove(position))
    }

    /// Возвращает участников, которые печатают в цели прямо сейчас.
    ///
    /// Заодно удаляет истекшие записи, чтобы состояние не росло при обрывах вкладок.
    pub(crate) async fn typers(&self, target: TypingTarget) -> Vec<TypingAuthorEntry> {
        let now = Instant::now();
        let mut entries = self.entries.lock().await;
        retain_fresh(&mut entries, now);
        entries
            .iter()
            .filter(|existing| existing.target == target)
            .cloned()
            .collect()
    }

    /// Удаляет все записи потока realtime и возвращает их для рассылки отмены.
    ///
    /// Вызывается при закрытии надежного потока, чтобы другие участники сразу
    /// перестали видеть индикатор набора закрытой вкладки.
    pub(crate) async fn remove_stream(&self, realtime_stream_id: Uuid) -> Vec<TypingAuthorEntry> {
        let mut entries = self.entries.lock().await;
        let mut removed = Vec::new();
        entries.retain(|existing| {
            if existing.realtime_stream_id == realtime_stream_id {
                removed.push(existing.clone());
                false
            } else {
                true
            }
        });
        self.last_refresh_at
            .lock()
            .await
            .retain(|(stream_id, _)| *stream_id != realtime_stream_id);
        removed
    }

    /// Удаляет записи, у которых истекло время жизни без продления.
    ///
    /// Фоновая задача вызывает этот метод, чтобы рассылать отмену набора тем,
    /// кто перестал продлевать его из-за потери связи или сбоя вкладки. Без
    /// этого записи просто исчезали бы в памяти, а у получателей индикатор
    /// оставался бы висеть до переподключения realtime.
    pub(crate) async fn remove_expired(&self) -> Vec<TypingAuthorEntry> {
        let now = Instant::now();
        let mut entries = self.entries.lock().await;
        let mut expired = Vec::new();
        entries.retain(|existing| {
            if now.saturating_duration_since(existing.refreshed_at) >= TYPING_TTL {
                expired.push(existing.clone());
                false
            } else {
                true
            }
        });
        expired
    }

    async fn allow_refresh(&self, realtime_stream_id: Uuid, now: Instant) -> bool {
        let mut last_refresh_at = self.last_refresh_at.lock().await;
        last_refresh_at.retain(|(stream_id, _)| *stream_id != realtime_stream_id);
        if let Some((_, last)) = last_refresh_at.first()
            && now.saturating_duration_since(*last) < TYPING_REFRESH_INTERVAL
        {
            return false;
        }
        last_refresh_at.push((realtime_stream_id, now));
        true
    }
}

fn retain_fresh(entries: &mut Vec<TypingAuthorEntry>, now: Instant) {
    entries.retain(|entry| now.saturating_duration_since(entry.refreshed_at) < TYPING_TTL);
}

#[cfg(test)]
mod tests;
