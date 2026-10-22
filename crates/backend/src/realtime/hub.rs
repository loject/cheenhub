//! Общий реестр потоков realtime и вещания.

use std::collections::HashSet;
use std::time::Duration;

use cheenhub_contracts::realtime::{RealtimeKind, RealtimeModule};
use dashmap::DashMap;
use futures_util::future::join_all;
use serde::Serialize;
use tokio::sync::{Mutex, watch};
use tokio::time::Instant;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::sink::{DatagramSink, EnvelopeSink, PreparedEnvelope};

mod disconnect;
pub(crate) use disconnect::DisconnectReason;

const SLOW_DATAGRAM_FANOUT_WARN_AFTER: Duration = Duration::from_millis(40);
const SLOW_DATAGRAM_FANOUT_WARNING_INTERVAL: Duration = Duration::from_secs(5);

/// Реестр активных realtime-транспортов и точечной доставки событий.
///
/// Хранит привязку потоков к аутентифицированным пользователям и модулям,
/// а также сессии для датаграмм и сигналов отключения. Не определяет,
/// кто имеет право получать событие: получателей выбирают модули приложения.
#[derive(Default)]
pub(crate) struct RealtimeHub {
    /// Надёжные потоки по их идентификаторам, независимо от транспорта.
    streams: DashMap<Uuid, RealtimeStream>,
    /// Активные транспортные сессии по идентификатору подключения.
    sessions: DashMap<Uuid, RealtimeSession>,
    /// Время последнего предупреждения для ограничения повторяющихся логов.
    last_slow_datagram_fanout_warning_at: Mutex<Option<Instant>>,
}

/// Зарегистрированный надёжный поток одного realtime-модуля.
///
/// Владелец определяется при аутентификации; запись не хранит права доступа
/// к серверам или комнатам и используется только для адресной доставки.
#[derive(Clone)]
struct RealtimeStream {
    /// Уникальный идентификатор потока для поиска и удаления из реестра.
    id: Uuid,
    /// Модуль, сообщения которого принимает этот поток.
    module: RealtimeModule,
    /// Аутентифицированный пользователь, открывший поток.
    user_id: Uuid,
    /// Абстракция отправки конвертов через WebTransport или WebSocket.
    send: EnvelopeSink,
}

/// Активная транспортная сессия для датаграмм и управления отключением.
///
/// Не определяет участие пользователя в голосовых комнатах: это состояние
/// принадлежит модулю голосового чата.
#[derive(Clone)]
struct RealtimeSession {
    /// Идентификатор транспортного подключения.
    id: Uuid,
    /// Аутентифицированный пользователь, которому принадлежит сессия.
    user_id: Uuid,
    /// Идентификатор auth-сессии для адресного отзыва подключения.
    auth_session_id: Uuid,
    /// Канал отправки ненадёжных датаграмм по выбранному транспорту.
    datagrams: DatagramSink,
    /// Сигнал закрытия транспорта с причиной отключения.
    disconnect: watch::Sender<Option<DisconnectReason>>,
}

/// Результат доставки одной датаграммы одному получателю.
///
/// Используется для диагностики медленных и неуспешных отправок.
struct DatagramFanoutOutcome {
    /// Продолжительность попытки отправки конкретному получателю.
    elapsed: Duration,
    /// Признак ошибки отправки, независимо от её длительности.
    failed: bool,
}

/// Сведения об активном потоке для выбора получателей модулем приложения.
///
/// Наличие записи подтверждает активность потока, но не право пользователя
/// получать конкретное событие: это проверяет вызывающий модуль.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RealtimeRecipient {
    /// Идентификатор надёжного realtime-потока.
    pub(crate) stream_id: Uuid,
    /// Аутентифицированный пользователь — владелец потока.
    pub(crate) user_id: Uuid,
}

impl RealtimeHub {
    /// Регистрирует тестовый WebSocket-транспорт без сетевого соединения.
    #[cfg(test)]
    pub(crate) async fn register_test_session(
        &self,
        user_id: Uuid,
        auth_session_id: Uuid,
    ) -> watch::Receiver<Option<DisconnectReason>> {
        let (outbound, _receiver) = tokio::sync::mpsc::channel(1);
        self.register_session(
            Uuid::new_v4(),
            user_id,
            auth_session_id,
            DatagramSink::websocket(outbound),
        )
        .await
    }

    /// Регистрирует надежный поток, привязанный к модулю.
    pub(crate) async fn register_stream(
        &self,
        stream_id: Uuid,
        module: RealtimeModule,
        user_id: Uuid,
        send: EnvelopeSink,
    ) {
        if let dashmap::Entry::Vacant(entry) = self.streams.entry(stream_id) {
            entry.insert(RealtimeStream {
                id: stream_id,
                module,
                user_id,
                send,
            });
            debug!(%stream_id, ?module, %user_id, "registered realtime stream");
        }
    }

    /// Регистрирует аутентифицированную realtime-сессию для вещания датаграмм.
    ///
    /// Возвращает сигнал, который транспорт обязан обработать завершением
    /// соединения после отзыва связанной auth-сессии или остановки процесса.
    pub(crate) async fn register_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        auth_session_id: Uuid,
        datagrams: DatagramSink,
    ) -> watch::Receiver<Option<DisconnectReason>> {
        let receiver = match self.sessions.entry(session_id) {
            dashmap::Entry::Occupied(session) => session.get().disconnect.subscribe(),
            dashmap::Entry::Vacant(session) => {
                let (disconnect, receiver) = watch::channel(None);
                session.insert(RealtimeSession {
                    id: session_id,
                    user_id,
                    auth_session_id,
                    datagrams,
                    disconnect,
                });
                receiver
            }
        };
        debug!(%session_id, %user_id, %auth_session_id, "registered realtime session");
        receiver
    }

    /// Удаляет аутентифицированную сессию WebTransport.
    pub(crate) async fn unregister_session(&self, session_id: Uuid) {
        self.sessions.remove(&session_id);
        debug!(%session_id, "unregistered realtime session");
    }

    /// Завершает все realtime-транспорты, связанные с одной auth-сессией.
    pub(crate) async fn disconnect_auth_session(&self, auth_session_id: &Uuid) -> usize {
        self.disconnect_matching(
            |session| session.auth_session_id == *auth_session_id,
            DisconnectReason::AuthSessionRevoked,
            "revoked auth session",
        )
        .await
    }

    /// Завершает все realtime-транспорты активных auth-сессий пользователя.
    pub(crate) async fn disconnect_user_sessions(&self, user_id: &Uuid) -> usize {
        self.disconnect_matching(
            |session| session.user_id == *user_id,
            DisconnectReason::AuthSessionRevoked,
            "revoked user sessions",
        )
        .await
    }

    /// Завершает все активные realtime-транспорты при остановке процесса.
    ///
    /// Вызывается при переходе в фазу завершения, чтобы клиенты узнали о
    /// плановом рестарте по коду закрытия и сразу переподключились, не дожидаясь
    /// истечения ping-таймаута. Возвращает количество закрытых транспортов.
    pub(crate) async fn disconnect_all_sessions(&self) -> usize {
        self.disconnect_matching(
            |_| true,
            DisconnectReason::ServiceRestarting,
            "process exit",
        )
        .await
    }

    /// Закрывает транспорты, отобранные фильтром, одной причиной.
    ///
    /// Сессии удаляются из реестра сразу, чтобы fanout больше не пытался писать
    /// в закрываемые транспорты.
    async fn disconnect_matching(
        &self,
        matches: impl Fn(&RealtimeSession) -> bool,
        reason: DisconnectReason,
        context: &'static str,
    ) -> usize {
        let mut disconnected = 0;
        self.sessions.retain(|_, session| {
            if !matches(session) {
                return true;
            }

            disconnected += 1;
            let _ = session.disconnect.send(Some(reason));
            false
        });
        let remaining = self.sessions.len();
        if disconnected > 0 {
            info!(
                %reason,
                realtime_session_count = disconnected,
                "disconnected realtime transports"
            );
        } else {
            debug!(%reason, "no realtime transports matched disconnect request");
        }
        debug!(remaining_sessions = remaining, removed_sessions = disconnected, %context, "realtime session registry updated after disconnect request");
        disconnected
    }

    /// Отправляет сырую датаграмму активным сессиям из снимка, кроме источника.
    ///
    /// `session_ids` может быть immutable-снимком получателей комнаты;
    /// `excluded_session_id` не получает собственную датаграмму отправителя.
    /// Список сессий копируется до первого `await`, поэтому map guards не
    /// переживают асинхронную отправку.
    pub(crate) async fn fanout_datagram_to_sessions_except(
        &self,
        session_ids: &[Uuid],
        excluded_session_id: Uuid,
        bytes: bytes::Bytes,
    ) {
        let started_at = Instant::now();
        let payload_bytes = bytes.len();
        let sessions = session_ids
            .iter()
            .filter(|session_id| **session_id != excluded_session_id)
            .filter_map(|session_id| {
                self.sessions
                    .get(session_id)
                    .map(|entry| entry.value().clone())
            })
            .collect::<Vec<_>>();
        let recipient_count = sessions.len();

        // TODO: benchmark this hot path before adding bounded concurrency or task spawning.
        let outcomes = join_all(sessions.into_iter().map(|session| {
            let bytes = bytes.clone();
            async move {
                let recipient_started_at = Instant::now();
                let result = session.datagrams.send_datagram(bytes).await;
                let elapsed = recipient_started_at.elapsed();
                if let Err(error) = result {
                    warn!(
                        session_id = %session.id,
                        user_id = %session.user_id,
                        %error,
                        "failed to fan out realtime datagram"
                    );
                    return DatagramFanoutOutcome {
                        elapsed,
                        failed: true,
                    };
                }

                DatagramFanoutOutcome {
                    elapsed,
                    failed: false,
                }
            }
        }))
        .await;

        let elapsed = started_at.elapsed();
        if elapsed >= SLOW_DATAGRAM_FANOUT_WARN_AFTER
            && self.should_warn_slow_datagram_fanout().await
        {
            let failed_recipient_count = outcomes.iter().filter(|outcome| outcome.failed).count();
            let slow_recipient_count = outcomes
                .iter()
                .filter(|outcome| outcome.elapsed >= SLOW_DATAGRAM_FANOUT_WARN_AFTER)
                .count();
            let max_recipient_send_ms = outcomes
                .iter()
                .map(|outcome| outcome.elapsed.as_millis())
                .max()
                .unwrap_or_default();

            // TODO: отправлять сообщение в телеграм(после появления пушей - администратору)
            warn!(
                recipient_count,
                slow_recipient_count,
                failed_recipient_count,
                payload_bytes,
                elapsed_ms = elapsed.as_millis(),
                max_recipient_send_ms,
                "slow realtime datagram fanout"
            );
        }
    }

    /// Удаляет надежный поток, привязанный к модулю.
    pub(crate) async fn unregister_stream(&self, stream_id: Uuid) {
        self.streams.remove(&stream_id);
        debug!(%stream_id, "unregistered realtime stream");
    }

    /// Возвращает идентификаторы и владельцев потоков модуля без проверок доступа.
    ///
    /// Принимающий модуль самостоятельно определяет допустимых получателей.
    pub(crate) fn module_recipients(&self, module: RealtimeModule) -> Vec<RealtimeRecipient> {
        self.streams
            .iter()
            .filter(|stream| stream.module == module)
            .map(|stream| RealtimeRecipient {
                stream_id: stream.id,
                user_id: stream.user_id,
            })
            .collect()
    }

    /// Возвращает активные потоки модуля для конкретных пользователей.
    pub(crate) async fn recipients_for_users(
        &self,
        module: RealtimeModule,
        user_ids: &[Uuid],
    ) -> Vec<RealtimeRecipient> {
        let user_ids = user_ids.iter().copied().collect::<HashSet<_>>();
        self.streams
            .iter()
            .filter(|stream| stream.module == module && user_ids.contains(&stream.user_id))
            .map(|stream| RealtimeRecipient {
                stream_id: stream.id,
                user_id: stream.user_id,
            })
            .collect()
    }

    /// Вещает событие, ограниченное сервером, выбранным потокам одного модуля realtime.
    pub(crate) async fn fanout_to_streams<P>(
        &self,
        module: RealtimeModule,
        server_id: &Uuid,
        kind: RealtimeKind,
        stream_ids: &[Uuid],
        payload: P,
    ) where
        P: Serialize,
    {
        let stream_ids = stream_ids.iter().copied().collect::<HashSet<_>>();
        let streams = stream_ids
            .iter()
            .filter_map(|stream_id| self.streams.get(stream_id))
            .filter(|stream| stream.module == module)
            .map(|stream| stream.value().clone())
            .collect::<Vec<_>>();

        if streams.is_empty() {
            return;
        }
        let prepared = match PreparedEnvelope::new(module, kind, payload) {
            Ok(prepared) => prepared,
            Err(error) => {
                warn!(
                    ?module, ?kind, %server_id, %error,
                    "failed to prepare realtime fanout envelope"
                );
                return;
            }
        };

        // TODO: benchmark this fanout path before adding bounded concurrency or task spawning.
        join_all(streams.into_iter().map(|stream| {
            let prepared = &prepared;
            async move {
                if let Err(error) = stream.send.send_prepared_envelope(prepared).await {
                    let transport = match &stream.send {
                        EnvelopeSink::WebTransport(_) => "webtransport",
                        EnvelopeSink::WebSocket(_) => "websocket",
                    };
                    let started_at = Instant::now();
                    warn!(
                        stream_id = %stream.id,
                        ?module,
                        ?kind,
                        %transport,
                        %server_id,
                        user_id = %stream.user_id,
                        elapsed_ms = started_at.elapsed().as_millis(),
                        error = %format_args!("{error:#}"),
                        root_cause = %error.root_cause(),
                        "failed to fan out realtime event"
                    );
                }
            }
        }))
        .await;
    }

    /// Вещает событие выбранным пользователям без server-scoped проверки.
    pub(crate) async fn fanout_to_user_streams<P>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        user_ids: &[Uuid],
        payload: P,
    ) -> usize
    where
        P: Serialize,
    {
        let recipients = self.recipients_for_users(module, user_ids).await;
        let stream_ids = recipients
            .iter()
            .map(|recipient| recipient.stream_id)
            .collect::<Vec<_>>();
        let recipient_count = stream_ids.len();
        self.fanout_to_streams(module, &Uuid::nil(), kind, &stream_ids, payload)
            .await;
        recipient_count
    }

    async fn should_warn_slow_datagram_fanout(&self) -> bool {
        let now = Instant::now();
        let mut last_warning_at = self.last_slow_datagram_fanout_warning_at.lock().await;
        if last_warning_at.is_some_and(|last_warning_at| {
            now.duration_since(last_warning_at) < SLOW_DATAGRAM_FANOUT_WARNING_INTERVAL
        }) {
            return false;
        }

        *last_warning_at = Some(now);
        true
    }
}

#[cfg(test)]
mod tests;
