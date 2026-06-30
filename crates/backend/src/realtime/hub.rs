//! Общий реестр потоков realtime и вещания.

use std::time::Duration;

use cheenhub_contracts::realtime::{RealtimeKind, RealtimeModule};
use futures_util::future::join_all;
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::state::AppState;

use super::protocol;
use super::sink::{DatagramSink, EnvelopeSink};

const SLOW_DATAGRAM_FANOUT_WARN_AFTER: Duration = Duration::from_millis(40);
const SLOW_DATAGRAM_FANOUT_WARNING_INTERVAL: Duration = Duration::from_secs(5);

/// Общий реестр активных потоков realtime, привязанных к модулям.
#[derive(Default)]
pub(crate) struct RealtimeHub {
    streams: Mutex<Vec<RealtimeStream>>,
    sessions: Mutex<Vec<RealtimeSession>>,
    last_slow_datagram_fanout_warning_at: Mutex<Option<Instant>>,
}

#[derive(Clone)]
struct RealtimeStream {
    id: Uuid,
    module: RealtimeModule,
    user_id: Uuid,
    send: EnvelopeSink,
}

#[derive(Clone)]
struct RealtimeSession {
    id: Uuid,
    user_id: Uuid,
    datagrams: DatagramSink,
}

struct DatagramFanoutOutcome {
    elapsed: Duration,
    failed: bool,
}

/// Публичный идентификатор потока, используемый в политиках вещания на уровне функций.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RealtimeRecipient {
    /// Stable realtime stream identifier.
    pub(crate) stream_id: Uuid,
    /// Authenticated user that owns the stream.
    pub(crate) user_id: Uuid,
}

impl RealtimeHub {
    /// Регистрирует надежный поток, привязанный к модулю.
    pub(crate) async fn register_stream(
        &self,
        stream_id: Uuid,
        module: RealtimeModule,
        user_id: Uuid,
        send: EnvelopeSink,
    ) {
        let mut streams = self.streams.lock().await;
        if streams.iter().any(|stream| stream.id == stream_id) {
            return;
        }
        streams.push(RealtimeStream {
            id: stream_id,
            module,
            user_id,
            send,
        });
        debug!(%stream_id, ?module, %user_id, "registered realtime stream");
    }

    /// Регистрирует аутентифицированную сессию WebTransport для вещания датаграмм.
    pub(crate) async fn register_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        datagrams: DatagramSink,
    ) {
        let mut sessions = self.sessions.lock().await;
        if sessions.iter().any(|session| session.id == session_id) {
            return;
        }
        sessions.push(RealtimeSession {
            id: session_id,
            user_id,
            datagrams,
        });
        debug!(%session_id, %user_id, "registered realtime session");
    }

    /// Удаляет аутентифицированную сессию WebTransport.
    pub(crate) async fn unregister_session(&self, session_id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|session| session.id != session_id);
        debug!(%session_id, "unregistered realtime session");
    }

    /// Отправляет одну сырую датаграмму выбранным активным сессиям.
    pub(crate) async fn fanout_datagram_to_sessions(
        &self,
        session_ids: &[Uuid],
        bytes: bytes::Bytes,
    ) {
        let started_at = Instant::now();
        let payload_bytes = bytes.len();
        let sessions = self
            .sessions
            .lock()
            .await
            .iter()
            .filter(|session| session_ids.contains(&session.id))
            .cloned()
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
        let mut streams = self.streams.lock().await;
        streams.retain(|stream| stream.id != stream_id);
        debug!(%stream_id, "unregistered realtime stream");
    }

    /// Возвращает активных получателей для модуля realtime на одном сервере.
    pub(crate) async fn recipients(
        &self,
        state: &AppState,
        module: RealtimeModule,
        server_id: &Uuid,
    ) -> Vec<RealtimeRecipient> {
        let streams = self
            .streams
            .lock()
            .await
            .iter()
            .filter(|stream| stream.module == module)
            .cloned()
            .collect::<Vec<_>>();
        let mut recipients = Vec::new();

        for stream in streams {
            match user_has_server_access(state, &stream.user_id, server_id).await {
                Ok(true) => recipients.push(RealtimeRecipient {
                    stream_id: stream.id,
                    user_id: stream.user_id,
                }),
                Ok(false) => {}
                Err(error) => {
                    warn!(
                        stream_id = %stream.id,
                        ?module,
                        user_id = %stream.user_id,
                        %server_id,
                        %error,
                        "failed to evaluate realtime server recipient"
                    );
                }
            }
        }

        recipients
    }

    /// Возвращает активные потоки модуля для конкретных пользователей.
    pub(crate) async fn recipients_for_users(
        &self,
        module: RealtimeModule,
        user_ids: &[Uuid],
    ) -> Vec<RealtimeRecipient> {
        self.streams
            .lock()
            .await
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
        P: Serialize + Clone,
    {
        let streams = self
            .streams
            .lock()
            .await
            .iter()
            .filter(|stream| stream.module == module && stream_ids.contains(&stream.id))
            .cloned()
            .collect::<Vec<_>>();

        // TODO: benchmark this fanout path before adding bounded concurrency or task spawning.
        join_all(streams.into_iter().map(|stream| {
            let payload = payload.clone();
            async move {
                if let Err(error) =
                    protocol::write_envelope(&stream.send, module, kind, None, payload).await
                {
                    warn!(
                        stream_id = %stream.id,
                        ?module,
                        %server_id,
                        user_id = %stream.user_id,
                        %error,
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
    ) where
        P: Serialize + Clone,
    {
        let recipients = self.recipients_for_users(module, user_ids).await;
        let stream_ids = recipients
            .iter()
            .map(|recipient| recipient.stream_id)
            .collect::<Vec<_>>();
        self.fanout_to_streams(module, &Uuid::nil(), kind, &stream_ids, payload)
            .await;
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

async fn user_has_server_access(
    state: &AppState,
    user_id: &Uuid,
    server_id: &Uuid,
) -> anyhow::Result<bool> {
    let Some(server) = state.server_store.find_server(server_id).await? else {
        return Ok(false);
    };
    if server.owner_user_id == *user_id {
        return Ok(true);
    }

    Ok(state
        .server_store
        .find_active_server_member(server_id, user_id)
        .await?
        .is_some())
}
