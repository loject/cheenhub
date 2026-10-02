//! Транспортные операции индикатора набора для комнат и личных диалогов.

use cheenhub_contracts::realtime::{
    DirectMessageTypingChanged, DirectMessageTypingSnapshot, DirectMessageTypingSnapshotRequest,
    RealtimeEnvelope, RealtimeKind, RealtimeModule, SocialKind, StartDirectMessageTyping,
    StartTyping, StopDirectMessageTyping, StopTyping, TextChatKind, TypingAuthor, TypingChanged,
    TypingSnapshot, TypingSnapshotRequest,
};
use dioxus::prelude::warn;
use futures_channel::mpsc;
use futures_util::StreamExt;
use std::time::Duration;
use web_time::Instant;

use crate::features::realtime::{RealtimeError, RealtimeHandle};

/// Действие формы сообщения, меняющее состояние набора.
///
/// Форма не знает про модуль realtime: она сообщает только намерение, а чат
/// подставляет `TypingTransport` для комнаты или личного диалога.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypingIntent {
    /// Пользователь печатает: начать набор или продлить существующий.
    Started,
    /// Пользователь перестал печатать: снять набор.
    Stopped,
}

/// Цель, в которой пользователь печатает сообщение.
///
/// Определяет, через какой модуль realtime уходит событие набора: комната
/// использует `TextChat`, личный диалог — `Social`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypingTarget {
    /// Комната серверного чата.
    Room {
        /// Идентификатор сервера.
        server_id: String,
        /// Идентификатор комнаты.
        room_id: String,
    },
    /// Личный диалог.
    DirectMessage {
        /// Идентификатор диалога.
        conversation_id: String,
    },
}

impl TypingTarget {
    /// Возвращает идентификатор цели для фильтрации входящих событий.
    ///
    /// У комнаты это идентификатор комнаты, у диалога — идентификатор диалога,
    /// поэтому сравнение с payload события одинаково для обоих случаев.
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Room { room_id, .. } => room_id,
            Self::DirectMessage { conversation_id } => conversation_id,
        }
    }
}

/// Время жизни строки индикатора у получателя без новых событий.
///
/// Сервер при истечении своей TTL молча выбрасывает запись, не рассылая отмену,
/// поэтому клиент обязан гасить индикатор сам: потерянный `stop` или обрыв
/// связи иначе оставляли бы «печатает…» на многие минуты. Значение совпадает с
/// серверной TTL, чтобы обе стороны гасили индикатор примерно одновременно.
pub(crate) const TYPING_PEER_TTL: Duration = Duration::from_secs(8);

/// Как часто получатель проверяет протухание строк индикатора.
pub(crate) const TYPING_SWEEP_INTERVAL: Duration = Duration::from_secs(2);

/// Участник, который печатает, вместе со временем последнего подтверждения.
///
/// Время нужно для локального протухания: сервер не рассылает отмену при истечении
/// своей TTL, поэтому получатель обязан сам гасить строку индикатора.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TyperEntry {
    /// Снимок автора набора.
    pub(crate) author: TypingAuthor,
    /// Момент последнего события набора по этому участнику.
    pub(crate) seen_at: Instant,
}

/// Входящее событие набора, уже разделенное по модулю realtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypingEvent {
    /// Участник комнаты начал или закончил печатать.
    Room(TypingChanged),
    /// Участник личного диалога начал или закончил печатать.
    DirectMessage(DirectMessageTypingChanged),
}

/// Целевые операции отправки набора одной цели.
///
/// Собираются из уже известных чату идентификаторов, поэтому вызывающий код не
/// передает их повторно на каждый ввод. Ошибки отправки только логируются:
/// потеря события набора не должна влиять на работу формы сообщения.
#[derive(Clone)]
pub(crate) struct TypingTransport {
    realtime: RealtimeHandle,
    target: TypingTarget,
}

impl TypingTransport {
    /// Создает транспорт набора для указанной цели.
    pub(crate) fn new(realtime: RealtimeHandle, target: TypingTarget) -> Self {
        Self { realtime, target }
    }

    /// Отправляет начало набора или продление.
    pub(crate) async fn start(&self) {
        let result = match &self.target {
            TypingTarget::Room { server_id, room_id } => {
                start_room_typing(&self.realtime, server_id.clone(), room_id.clone()).await
            }
            TypingTarget::DirectMessage { conversation_id } => {
                start_direct_message_typing(&self.realtime, conversation_id.clone()).await
            }
        };
        if let Err(error) = result {
            warn!(
                target_id = self.target.id(),
                %error,
                "typing start was not delivered"
            );
        }
    }

    /// Отправляет завершение набора.
    pub(crate) async fn stop(&self) {
        let result = match &self.target {
            TypingTarget::Room { server_id, room_id } => {
                stop_room_typing(&self.realtime, server_id.clone(), room_id.clone()).await
            }
            TypingTarget::DirectMessage { conversation_id } => {
                stop_direct_message_typing(&self.realtime, conversation_id.clone()).await
            }
        };
        if let Err(error) = result {
            warn!(
                target_id = self.target.id(),
                %error,
                "typing stop was not delivered"
            );
        }
    }
}

/// Сообщает серверу, что пользователь печатает в комнате.
///
/// Отправляется один раз при начале набора и повторяется как продление, пока
/// пользователь печатает: сервер сам ограничивает частоту и не рассылает лишние
/// события, поэтому клиент может продлевать на каждый ввод.
pub(crate) async fn start_room_typing(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::StartTyping),
            StartTyping { server_id, room_id },
        )
        .await
}

/// Сообщает серверу, что пользователь перестал печатать в комнате.
pub(crate) async fn stop_room_typing(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::StopTyping),
            StopTyping { server_id, room_id },
        )
        .await
}

/// Сообщает серверу, что пользователь печатает в личном диалоге.
pub(crate) async fn start_direct_message_typing(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::StartDirectMessageTyping),
            StartDirectMessageTyping { conversation_id },
        )
        .await
}

/// Сообщает серверу, что пользователь перестал печатать в личном диалоге.
pub(crate) async fn stop_direct_message_typing(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<(), RealtimeError> {
    realtime
        .send_reliable(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::StopDirectMessageTyping),
            StopDirectMessageTyping { conversation_id },
        )
        .await
}

/// Запрашивает снимок участников комнаты, которые печатают.
pub(crate) async fn load_room_typing_snapshot(
    realtime: &RealtimeHandle,
    server_id: String,
    room_id: String,
) -> Result<TypingSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::TextChat,
            RealtimeKind::TextChat(TextChatKind::TypingSnapshot),
            TypingSnapshotRequest { server_id, room_id },
        )
        .await
}

/// Запрашивает снимок участников диалога, которые печатают.
pub(crate) async fn load_direct_message_typing_snapshot(
    realtime: &RealtimeHandle,
    conversation_id: String,
) -> Result<DirectMessageTypingSnapshot, RealtimeError> {
    realtime
        .request(
            RealtimeModule::Social,
            RealtimeKind::Social(SocialKind::DirectMessageTypingSnapshot),
            DirectMessageTypingSnapshotRequest { conversation_id },
        )
        .await
}

/// Подписывается на входящие события набора для комнат и личных диалогов.
///
/// События обоих модулей приходят в один поток: чат выбирает нужные по
/// идентификатору цели, поэтому переподключение realtime не требует отдельной
/// подписки на каждый модуль.
pub(crate) fn subscribe_typing_events(
    realtime: &RealtimeHandle,
) -> mpsc::UnboundedReceiver<TypingEvent> {
    let events = realtime.subscribe_events();
    let (sender, receiver) = mpsc::unbounded();

    dioxus::prelude::spawn(async move {
        let mut events = events;
        while let Some(envelope) = events.next().await {
            let Some(event) = decode_typing_event(envelope) else {
                continue;
            };
            if sender.unbounded_send(event).is_err() {
                break;
            }
        }
    });

    receiver
}

/// Возвращает идентификатор цели, к которому относится событие набора.
pub(crate) fn typing_event_target(event: &TypingEvent) -> &str {
    match event {
        TypingEvent::Room(payload) => &payload.room_id,
        TypingEvent::DirectMessage(payload) => &payload.conversation_id,
    }
}

/// Возвращает автора и признак набора из события любого модуля.
pub(crate) fn typing_event_author(event: &TypingEvent) -> (&TypingAuthor, bool) {
    match event {
        TypingEvent::Room(payload) => (&payload.author, payload.is_typing),
        TypingEvent::DirectMessage(payload) => (&payload.author, payload.is_typing),
    }
}

/// Применяет событие набора к снимку участников.
///
/// Возвращает `true`, если снимок изменился. Событие о собственном наборе и
/// повторное событие с тем же состоянием игнорируются, чтобы лишние события не
/// вызывали перерисовку индикатора. Каждое обновление продлевает время жизни
/// строки, чтобы индикатор гас только после реального прекращения набора.
pub(crate) fn apply_typing_event(
    typers: &mut Vec<TyperEntry>,
    author: &TypingAuthor,
    is_typing: bool,
    current_user_id: &str,
    now: Instant,
) -> bool {
    if author.user_id == current_user_id {
        return false;
    }
    let existing = typers
        .iter()
        .position(|entry| entry.author.user_id == author.user_id);
    match (is_typing, existing) {
        (true, None) => {
            typers.push(TyperEntry {
                author: author.clone(),
                seen_at: now,
            });
            true
        }
        (true, Some(position)) => {
            typers[position].seen_at = now;
            if typers[position].author == *author {
                return false;
            }
            typers[position].author = author.clone();
            true
        }
        (false, Some(position)) => {
            typers.remove(position);
            true
        }
        (false, None) => false,
    }
}

/// Убирает участников, которые перестали продлевать набор.
///
/// Сервер молча выбрасывает истекшие записи без события отмены, поэтому без этой
/// проверки индикатор оставался бы висеть до переподключения realtime.
pub(crate) fn drop_expired_typers(typers: &mut Vec<TyperEntry>, now: Instant) -> bool {
    let previous_len = typers.len();
    typers.retain(|entry| now.saturating_duration_since(entry.seen_at) < TYPING_PEER_TTL);
    typers.len() != previous_len
}

/// Заменяет снимок участников, не добавляя в него текущего пользователя.
///
/// Снимок с сервера уже без автора, но фильтр остается: он защищает от рассинхронизации
/// состояния и не дает показать свой же набор в том же чате. Время жизни строк
/// сбрасывается на текущий момент, иначе снимок сразу устареет и индикатор погаснет.
pub(crate) fn adopt_typing_snapshot(
    typers: &mut Vec<TyperEntry>,
    snapshot: Vec<TypingAuthor>,
    current_user_id: &str,
    now: Instant,
) {
    typers.clear();
    typers.extend(
        snapshot
            .into_iter()
            .filter(|author| author.user_id != current_user_id)
            .map(|author| TyperEntry {
                author,
                seen_at: now,
            }),
    );
}

fn decode_typing_event(envelope: RealtimeEnvelope) -> Option<TypingEvent> {
    match envelope.kind {
        RealtimeKind::TextChat(TextChatKind::TypingChanged) => {
            serde_json::from_value::<TypingChanged>(envelope.payload)
                .ok()
                .map(TypingEvent::Room)
        }
        RealtimeKind::Social(SocialKind::DirectMessageTypingChanged) => {
            serde_json::from_value::<DirectMessageTypingChanged>(envelope.payload)
                .ok()
                .map(TypingEvent::DirectMessage)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(nickname: &str, seen_at: Instant) -> TyperEntry {
        TyperEntry {
            author: TypingAuthor {
                user_id: nickname.to_owned(),
                nickname: nickname.to_owned(),
                avatar_url: None,
            },
            seen_at,
        }
    }

    #[test]
    fn lost_stop_event_still_expires_the_indicator() {
        // Регрессия: потерянный stop или сбой вкладки оставлял индикатор висеть до
        // переподключения realtime. Теперь строка гаснет по локальному TTL.
        let mut typers = vec![entry("Аня", Instant::now())];

        assert!(
            !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL / 2),
            "до истечения TTL строка должна оставаться"
        );
        assert!(
            drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL),
            "после истечения TTL строка обязана исчезнуть"
        );
        assert!(typers.is_empty());
    }

    #[test]
    fn refresh_extends_indicator_lifetime() {
        let author = TypingAuthor {
            user_id: "Аня".to_owned(),
            nickname: "Аня".to_owned(),
            avatar_url: None,
        };
        let mut typers = vec![entry("Аня", Instant::now())];

        for step in 1..=3 {
            apply_typing_event(
                &mut typers,
                &author,
                true,
                "me",
                Instant::now() + TYPING_PEER_TTL / 2 * step,
            );
        }

        assert!(
            !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL * 2),
            "продления должны сдвигать момент протухания"
        );
        assert!(
            drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL * 4),
            "без новых продлений строка в итоге протухает"
        );
    }

    #[test]
    fn stop_event_removes_immediately_without_waiting_for_ttl() {
        let author = TypingAuthor {
            user_id: "Аня".to_owned(),
            nickname: "Аня".to_owned(),
            avatar_url: None,
        };
        let mut typers = vec![entry("Аня", Instant::now())];

        assert!(apply_typing_event(
            &mut typers,
            &author,
            false,
            "me",
            Instant::now(),
        ));
        assert!(
            typers.is_empty(),
            "явная отмена должна гасить индикатор сразу"
        );
    }

    #[test]
    fn snapshot_resets_lifetime() {
        let author = TypingAuthor {
            user_id: "Аня".to_owned(),
            nickname: "Аня".to_owned(),
            avatar_url: None,
        };
        let mut typers = vec![entry("Аня", Instant::now())];

        adopt_typing_snapshot(&mut typers, vec![author], "me", Instant::now());

        assert!(
            !drop_expired_typers(&mut typers, Instant::now() + TYPING_PEER_TTL / 2),
            "снимок сервера должен сбрасывать время жизни строки"
        );
    }
}
