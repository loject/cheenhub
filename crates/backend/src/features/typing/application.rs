//! Потоки приложения индикатора набора сообщения.

use cheenhub_contracts::realtime::{
    DirectMessageTypingSnapshot, DirectMessageTypingSnapshotRequest, RejectionCode, TypingAuthor,
    TypingSnapshot, TypingSnapshotRequest,
};
use uuid::Uuid;

use self::fanout::TypingTargetKind;
use crate::features::text_chat::policy;
use crate::features::typing::infrastructure::{TypingAuthorEntry, TypingTarget};
use crate::realtime::EnvelopeSink;
use crate::realtime::protocol::send_rejection;
use crate::state::AppState;

mod fanout;

/// Как часто фоновая задача проверяет истекшие записи набора.
///
/// Значение меньше времени жизни записи, чтобы отмена уходила получателям с
/// точностью до интервала проверки, а не с задержкой в полную TTL.
const TYPING_SWEEP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Ошибка операции индикатора набора, которую нужно отдать клиенту отказом.
///
/// Формируется на границе application и переводится в `Rejected` realtime-конверт
/// вызывающим модулем-владельцем (`text_chat` или `social`).
#[derive(Debug)]
pub(crate) enum TypingApplicationError {
    /// Запрос не прошел проверку формата или доступа к комнате.
    BadRequest(String),
    /// Внутренняя ошибка чтения или рассылки состояния.
    Internal(anyhow::Error),
}

/// Обрабатывает начало набора или продление набора в комнате.
///
/// Продление не меняет видимое состояние и не рассылается повторно, поэтому
/// клиент может вызывать обработчик на каждый ввод без дополнительной фильтрации.
pub(crate) async fn start_room_typing(
    state: &AppState,
    entry: TypingAuthorEntry,
    server_id: Uuid,
    room_id: Uuid,
) -> Result<(), TypingApplicationError> {
    ensure_room_access(state, &entry.user_id, &server_id, &room_id).await?;
    let started = start_entry(
        state,
        entry.clone(),
        &server_id,
        &room_id,
        "room typing started",
    )
    .await;
    if !started {
        return Ok(());
    }
    fanout::notify_room_typing_changed(state, &server_id, &room_id, &entry, true).await;

    Ok(())
}

/// Обрабатывает завершение набора в комнате.
pub(crate) async fn stop_room_typing(
    state: &AppState,
    target: TypingTarget,
    user_id: Uuid,
    realtime_stream_id: Uuid,
) -> Result<(), TypingApplicationError> {
    let Some(entry) = state
        .typing_store
        .stop(target, user_id, realtime_stream_id)
        .await
    else {
        return Ok(());
    };

    tracing::debug!(
        %user_id,
        server_id = %target.server_id,
        room_id = %target.room_id,
        %realtime_stream_id,
        "room typing stopped"
    );
    fanout::notify_room_typing_changed(state, &target.server_id, &target.room_id, &entry, false)
        .await;

    Ok(())
}

/// Возвращает снимок участников комнаты, которые печатают сообщение.
pub(crate) async fn room_typing_snapshot(
    state: &AppState,
    user_id: &Uuid,
    request: TypingSnapshotRequest,
) -> Result<TypingSnapshot, TypingApplicationError> {
    let (server_id, room_id) = parse_room_target(&request.server_id, &request.room_id)?;
    ensure_room_access(state, user_id, &server_id, &room_id).await?;

    Ok(TypingSnapshot {
        server_id: server_id.to_string(),
        room_id: room_id.to_string(),
        typers: snapshot_typers(state, TypingTarget::room(server_id, room_id), user_id).await,
    })
}

/// Обрабатывает начало набора или продление набора в личном диалоге.
pub(crate) async fn start_direct_message_typing(
    state: &AppState,
    entry: TypingAuthorEntry,
    conversation_id: Uuid,
) -> Result<(), TypingApplicationError> {
    ensure_conversation_access(state, &entry.user_id, &conversation_id).await?;
    let started = start_entry(
        state,
        entry.clone(),
        &conversation_id,
        &conversation_id,
        "direct message typing started",
    )
    .await;
    if !started {
        return Ok(());
    }
    fanout::notify_direct_message_typing_changed(state, &conversation_id, &entry, true).await;

    Ok(())
}

/// Обрабатывает завершение набора в личном диалоге.
pub(crate) async fn stop_direct_message_typing(
    state: &AppState,
    target: TypingTarget,
    user_id: Uuid,
    realtime_stream_id: Uuid,
) -> Result<(), TypingApplicationError> {
    let Some(entry) = state
        .typing_store
        .stop(target, user_id, realtime_stream_id)
        .await
    else {
        return Ok(());
    };

    tracing::debug!(
        %user_id, conversation_id = %target.room_id, %realtime_stream_id,
        "direct message typing stopped"
    );
    fanout::notify_direct_message_typing_changed(state, &target.room_id, &entry, false).await;

    Ok(())
}

/// Возвращает снимок участников диалога, которые печатают сообщение.
pub(crate) async fn direct_message_typing_snapshot(
    state: &AppState,
    user_id: &Uuid,
    request: DirectMessageTypingSnapshotRequest,
) -> Result<DirectMessageTypingSnapshot, TypingApplicationError> {
    let conversation_id = parse_id(&request.conversation_id)?;
    ensure_conversation_access(state, user_id, &conversation_id).await?;

    Ok(DirectMessageTypingSnapshot {
        conversation_id: conversation_id.to_string(),
        typers: snapshot_typers(
            state,
            TypingTarget::direct_message(conversation_id),
            user_id,
        )
        .await,
    })
}

/// Периодически снимает наборы, которые перестали продлеваться, и рассылает отмену.
///
/// Запускается один раз при старте бэкенда. Без этой задачи истекшие записи
/// исчезали бы из памяти молча, и у получателей индикатор «печатает…» оставался
/// бы висеть до переподключения realtime. Частота проверки не влияет на точность:
/// запись считается истекшей по своему времени жизни, а не по моменту проверки.
pub(crate) async fn spawn_expiry_sweeper(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TYPING_SWEEP_INTERVAL);
        // Первый тик срабатывает сразу, но проверять ещё нечего: сразу после
        // старта ни у кого нет активного набора.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            for entry in state.typing_store.remove_expired().await {
                tracing::debug!(
                    user_id = %entry.user_id,
                    target_kind = ?entry.target.kind,
                    route_id = %entry.target.route_id(),
                    "typing state expired without refresh"
                );
                notify_typing_cancelled(&state, &entry).await;
            }
        }
    });
}

/// Рассылает отмену набора для одной истекшей записи.
async fn notify_typing_cancelled(state: &AppState, entry: &TypingAuthorEntry) {
    match entry.target.kind {
        TypingTargetKind::Room => {
            fanout::notify_room_typing_changed(
                state,
                &entry.target.server_id,
                &entry.target.room_id,
                entry,
                false,
            )
            .await;
        }
        TypingTargetKind::DirectMessage => {
            fanout::notify_direct_message_typing_changed(
                state,
                &entry.target.room_id,
                entry,
                false,
            )
            .await;
        }
    }
}

/// Снимает набор закрытого потока realtime и рассылает отмену участникам.
///
/// Вызывается из `realtime::router::cleanup_stream`, поэтому обрыв вкладки или
/// переподключение не оставляют «зависший» индикатор у других участников.
pub(crate) async fn disconnect_realtime_stream(state: &AppState, realtime_stream_id: Uuid) {
    for entry in state.typing_store.remove_stream(realtime_stream_id).await {
        tracing::debug!(
            %realtime_stream_id,
            user_id = %entry.user_id,
            target_kind = ?entry.target.kind,
            route_id = %entry.target.route_id(),
            "clearing typing state of closed realtime stream"
        );
        match entry.target.kind {
            TypingTargetKind::Room => {
                fanout::notify_room_typing_changed(
                    state,
                    &entry.target.server_id,
                    &entry.target.room_id,
                    &entry,
                    false,
                )
                .await;
            }
            TypingTargetKind::DirectMessage => {
                fanout::notify_direct_message_typing_changed(
                    state,
                    &entry.target.room_id,
                    &entry,
                    false,
                )
                .await;
            }
        }
    }
}

/// Регистрирует начало или продление набора и логирует результат.
///
/// Возвращает `true`, только если видимое состояние изменилось и событие нужно
/// разослать. Продление внутри интервала ограничения сервера тоже возвращает
/// `false`: клиент шлет его на каждый ввод и не должен получать эхо обратно.
async fn start_entry(
    state: &AppState,
    entry: TypingAuthorEntry,
    route_id: &Uuid,
    target_id: &Uuid,
    started_message: &'static str,
) -> bool {
    let started = state.typing_store.start(entry.clone()).await;
    if started {
        tracing::debug!(
            user_id = %entry.user_id,
            %route_id,
            %target_id,
            realtime_stream_id = %entry.realtime_stream_id,
            nickname = %entry.nickname,
            "{started_message}"
        );
    } else {
        tracing::debug!(
            user_id = %entry.user_id,
            %route_id,
            %target_id,
            realtime_stream_id = %entry.realtime_stream_id,
            "refreshed typing state"
        );
    }
    started
}

async fn snapshot_typers(
    state: &AppState,
    target: TypingTarget,
    current_user_id: &Uuid,
) -> Vec<TypingAuthor> {
    state
        .typing_store
        .typers(target)
        .await
        .iter()
        .filter(|entry| entry.user_id != *current_user_id)
        .map(fanout::author_snapshot)
        .collect()
}

async fn ensure_room_access(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
    room_id: &Uuid,
) -> Result<(), TypingApplicationError> {
    if policy::can_receive_room_event(state, user_id, server_id, room_id)
        .await
        .map_err(TypingApplicationError::Internal)?
    {
        Ok(())
    } else {
        Err(TypingApplicationError::BadRequest(
            "Комната недоступна.".to_owned(),
        ))
    }
}

async fn ensure_conversation_access(
    state: &AppState,
    user_id: &Uuid,
    conversation_id: &Uuid,
) -> Result<(), TypingApplicationError> {
    let participants = fanout::conversation_participant_ids(state, conversation_id)
        .await
        .map_err(TypingApplicationError::Internal)?;
    if participants.contains(user_id) {
        Ok(())
    } else {
        Err(TypingApplicationError::BadRequest(
            "Диалог недоступен.".to_owned(),
        ))
    }
}

/// Отправляет клиенту отказ по операции индикатора набора.
///
/// Набор не является критичной операцией, поэтому отказ всегда логируется на
/// уровне `debug` или `warn` и никогда не раскрывает внутренние детали.
pub(crate) async fn send_rejection_for(
    send: &EnvelopeSink,
    request_id: Option<Uuid>,
    error: TypingApplicationError,
) -> anyhow::Result<()> {
    match error {
        TypingApplicationError::BadRequest(message) => {
            tracing::debug!(?request_id, %message, "typing request rejected");
            send_rejection(send, request_id, RejectionCode::BadRequest, &message).await
        }
        TypingApplicationError::Internal(error) => {
            tracing::error!(?request_id, %error, "typing request failed");
            send_rejection(
                send,
                request_id,
                RejectionCode::InternalError,
                "Не удалось обновить состояние набора.",
            )
            .await
        }
    }
}

/// Разбирает идентификаторы сервера и комнаты из запроса снимка или набора.
///
/// Ошибки формата намеренно не различают сервер и комнату: клиенту не нужно
/// знать, какой именно идентификатор не прошел проверку.
pub(crate) fn parse_room_target(
    server_id: &str,
    room_id: &str,
) -> Result<(Uuid, Uuid), TypingApplicationError> {
    Ok((parse_id(server_id)?, parse_id(room_id)?))
}

/// Разбирает идентификатор личного диалога из запроса снимка или набора.
pub(crate) fn parse_conversation_id(value: &str) -> Result<Uuid, TypingApplicationError> {
    parse_id(value)
}

fn parse_id(value: &str) -> Result<Uuid, TypingApplicationError> {
    Uuid::parse_str(value)
        .map_err(|_| TypingApplicationError::BadRequest("Комната недоступна.".to_owned()))
}
