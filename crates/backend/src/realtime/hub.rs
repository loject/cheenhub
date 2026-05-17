//! Shared realtime stream registry and fanout.

use cheenhub_contracts::realtime::{RealtimeKind, RealtimeModule};
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::state::AppState;

use super::protocol;
use super::sink::{DatagramSink, EnvelopeSink};

/// Shared registry for active module-bound realtime streams.
#[derive(Default)]
pub(crate) struct RealtimeHub {
    streams: Mutex<Vec<RealtimeStream>>,
    sessions: Mutex<Vec<RealtimeSession>>,
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

/// Public stream identity used by feature-level fanout policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RealtimeRecipient {
    /// Stable realtime stream identifier.
    pub(crate) stream_id: Uuid,
    /// Authenticated user that owns the stream.
    pub(crate) user_id: Uuid,
}

impl RealtimeHub {
    /// Registers a module-bound reliable stream.
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

    /// Registers an authenticated WebTransport session for datagram fanout.
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

    /// Removes an authenticated WebTransport session.
    pub(crate) async fn unregister_session(&self, session_id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|session| session.id != session_id);
        debug!(%session_id, "unregistered realtime session");
    }

    /// Sends one raw datagram to selected active sessions.
    pub(crate) async fn fanout_datagram_to_sessions(
        &self,
        session_ids: &[Uuid],
        bytes: bytes::Bytes,
    ) {
        let sessions = self
            .sessions
            .lock()
            .await
            .iter()
            .filter(|session| session_ids.contains(&session.id))
            .cloned()
            .collect::<Vec<_>>();

        for session in sessions {
            if let Err(error) = session.datagrams.send_datagram(bytes.clone()).await {
                warn!(
                    session_id = %session.id,
                    user_id = %session.user_id,
                    %error,
                    "failed to fan out realtime datagram"
                );
            }
        }
    }

    /// Removes a module-bound reliable stream.
    pub(crate) async fn unregister_stream(&self, stream_id: Uuid) {
        let mut streams = self.streams.lock().await;
        streams.retain(|stream| stream.id != stream_id);
        debug!(%stream_id, "unregistered realtime stream");
    }

    /// Returns active recipients for a realtime module on one server.
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

    /// Fans out a server-scoped event to selected streams of one realtime module.
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

        for stream in streams {
            if let Err(error) =
                protocol::write_envelope(&stream.send, module, kind, None, payload.clone()).await
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
