//! Realtime connection handle.

mod fire_and_forget;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use bytes::Bytes;
use cheenhub_contracts::realtime::{
    Authenticate, Authenticated, ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule,
    Rejected,
};
use dioxus::prelude::{debug, info, warn};
use futures_channel::{mpsc, oneshot};
use futures_util::lock::Mutex;
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;
use web_transport::{SendStream, Session};

use super::config;
use super::error::RealtimeError;
use super::framing;
use super::guards::{
    ModuleStreams, PendingRequestGuard, PendingRequests, StreamWriteGuard, remove_cached_stream,
};
use super::inbound::{EventListeners, spawn_universal_reader};
use super::status::{RealtimeConnectionStatus, RealtimeTransportKind};
use super::websocket::{self, WebSocketOutbound, WebSocketOutboundSender};
use super::webtransport;

pub(super) type DatagramListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<Bytes>>>>;
type StatusListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<RealtimeConnectionStatus>>>>;

/// Cloneable handle exposed by the realtime provider.
#[derive(Clone)]
pub(crate) struct RealtimeHandle {
    inner: Rc<RealtimeInner>,
}

struct RealtimeInner {
    session: Mutex<Option<ConnectedSession>>,
    streams: ModuleStreams,
    pending: PendingRequests,
    event_listeners: EventListeners,
    datagram_listeners: DatagramListeners,
    inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
    generation: Cell<u64>,
    connection_status: Cell<RealtimeConnectionStatus>,
    status_listeners: StatusListeners,
}

#[derive(Clone)]
struct ConnectedSession {
    generation: u64,
    transport: ConnectedTransport,
}

#[derive(Clone)]
enum ConnectedTransport {
    WebTransport(Rc<Session>),
    WebSocket(WebSocketOutboundSender),
}

impl RealtimeHandle {
    /// Opens and authenticates the realtime session.
    pub(crate) async fn connect(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        match self.connect_webtransport(access_token.clone()).await {
            Ok(authenticated) => Ok(authenticated),
            Err(webtransport_error) => {
                warn!(
                    %webtransport_error,
                    "WebTransport realtime connection failed; trying WebSocket fallback"
                );
                self.connect_websocket(access_token).await.map_err(|websocket_error| {
                    RealtimeError::new(format!(
                        "Failed to connect realtime session: WebTransport error: {webtransport_error}; WebSocket fallback error: {websocket_error}"
                    ))
                })
            }
        }
    }

    async fn connect_webtransport(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        let client = config::realtime_client()?;
        let url = config::realtime_url()?;
        info!(%url, "connecting WebTransport realtime session");
        let session = client.connect(url.clone()).await.map_err(|error| {
            RealtimeError::new(format!("Failed to connect realtime session: {error}"))
        })?;

        info!(%url, "WebTransport transport connected");
        let generation = self.next_generation();
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.inner.session.lock().await.replace(ConnectedSession {
            generation,
            transport: ConnectedTransport::WebTransport(Rc::new(session.clone())),
        });

        let authenticated = self
            .request(
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::Authenticate),
                Authenticate { access_token },
            )
            .await;
        let authenticated: Authenticated = match authenticated {
            Ok(authenticated) => authenticated,
            Err(error) => {
                self.clear_generation(generation).await;
                return Err(error);
            }
        };
        info!(%url, user_id = %authenticated.user.id, "WebTransport realtime authenticated");
        self.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebTransport,
        ));
        webtransport::spawn_datagram_reader(
            session.clone(),
            generation,
            self.inner.datagram_listeners.clone(),
        );
        webtransport::spawn_connection_watcher(url.to_string(), session, generation, self.clone());

        Ok(authenticated)
    }

    async fn connect_websocket(
        &self,
        access_token: String,
    ) -> Result<Authenticated, RealtimeError> {
        let url = config::realtime_websocket_url()?;
        info!(%url, "connecting WebSocket realtime fallback session");
        let (writer, reader) = websocket::split(url.as_str())?;
        let (sender, receiver) = mpsc::unbounded();
        let generation = self.next_generation();
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.inner.session.lock().await.replace(ConnectedSession {
            generation,
            transport: ConnectedTransport::WebSocket(sender),
        });
        websocket::spawn_writer(
            url.to_string(),
            generation,
            writer,
            receiver,
            Some(self.clone()),
        );
        websocket::spawn_reader(
            url.to_string(),
            generation,
            reader,
            self.inner.inbound.clone(),
            self.inner.datagram_listeners.clone(),
            self.clone(),
        );

        let authenticated = self
            .request(
                RealtimeModule::Control,
                RealtimeKind::Control(ControlKind::Authenticate),
                Authenticate { access_token },
            )
            .await;
        let authenticated: Authenticated = match authenticated {
            Ok(authenticated) => authenticated,
            Err(error) => {
                self.clear_generation(generation).await;
                return Err(error);
            }
        };
        info!(%url, user_id = %authenticated.user.id, "WebSocket realtime fallback authenticated");
        self.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebSocketFallback,
        ));

        Ok(authenticated)
    }

    /// Sends one raw unreliable datagram message.
    pub(crate) async fn send_unreliable_bytes(&self, bytes: Bytes) -> Result<(), RealtimeError> {
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };

        match connected.transport {
            ConnectedTransport::WebTransport(session) => {
                session.send_datagram(bytes).await.map_err(|error| {
                    RealtimeError::new(format!("Failed to send realtime datagram: {error}"))
                })
            }
            ConnectedTransport::WebSocket(sender) => sender
                .unbounded_send(WebSocketOutbound::Datagram(bytes))
                .map_err(|_| RealtimeError::new("Realtime WebSocket fallback writer is closed.")),
        }
    }

    /// Subscribes to inbound fire-and-forget realtime events for this tab.
    pub(crate) fn subscribe_events(&self) -> mpsc::UnboundedReceiver<RealtimeEnvelope> {
        let (sender, receiver) = mpsc::unbounded();
        self.inner.event_listeners.borrow_mut().push(sender);

        receiver
    }

    /// Subscribes to inbound raw unreliable datagrams for this tab.
    pub(crate) fn subscribe_datagrams(&self) -> mpsc::UnboundedReceiver<Bytes> {
        let (sender, receiver) = mpsc::unbounded();
        self.inner.datagram_listeners.borrow_mut().push(sender);

        receiver
    }

    /// Returns the current realtime connection status.
    pub(crate) fn connection_status(&self) -> RealtimeConnectionStatus {
        self.inner.connection_status.get()
    }

    /// Subscribes to realtime connection status changes for this tab.
    pub(crate) fn subscribe_connection_status(
        &self,
    ) -> mpsc::UnboundedReceiver<RealtimeConnectionStatus> {
        let (sender, receiver) = mpsc::unbounded();
        let _ = sender.unbounded_send(self.connection_status());
        self.inner.status_listeners.borrow_mut().push(sender);

        receiver
    }

    /// Sends one request and waits for a typed response.
    pub(crate) async fn request<P, R>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<R, RealtimeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        validate_module_kind(module, kind)?;
        let request_id = Uuid::new_v4();
        let envelope =
            RealtimeEnvelope::new(module, kind, Some(request_id), payload).map_err(|error| {
                RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
            })?;
        let (sender, receiver) = oneshot::channel();
        self.inner
            .pending
            .borrow_mut()
            .insert((module, request_id), sender);
        let pending_guard =
            PendingRequestGuard::new(self.inner.pending.clone(), (module, request_id));

        self.write_envelope(envelope).await?;

        let response = receiver
            .await
            .map_err(|_| RealtimeError::new("Realtime response channel closed."))?;
        pending_guard.disarm();
        if response.kind == RealtimeKind::Control(ControlKind::Rejected) {
            let rejected =
                serde_json::from_value::<Rejected>(response.payload).map_err(|error| {
                    RealtimeError::new(format!("Failed to decode realtime rejection: {error}"))
                })?;
            return Err(RealtimeError::new(rejected.message));
        }
        serde_json::from_value(response.payload).map_err(|error| {
            RealtimeError::new(format!("Failed to decode realtime response: {error}"))
        })
    }

    async fn write_envelope(&self, envelope: RealtimeEnvelope) -> Result<(), RealtimeError> {
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };

        match connected.transport {
            ConnectedTransport::WebTransport(session) => {
                if uses_cached_stream(envelope.module) {
                    self.write_webtransport_envelope(envelope, (*session).clone())
                        .await
                } else {
                    self.write_webtransport_one_shot(envelope, (*session).clone())
                        .await
                }
            }
            ConnectedTransport::WebSocket(sender) => sender
                .unbounded_send(WebSocketOutbound::Envelope(envelope))
                .map_err(|_| RealtimeError::new("Realtime WebSocket fallback writer is closed.")),
        }
    }

    async fn write_webtransport_envelope(
        &self,
        envelope: RealtimeEnvelope,
        session: Session,
    ) -> Result<(), RealtimeError> {
        let module = envelope.module;
        let mut last_error = None;

        for attempt in 0..2 {
            let stream = self.stream_for(module, session.clone()).await?;
            let write_guard =
                StreamWriteGuard::new(module, self.inner.streams.clone(), stream.clone());
            match framing::write_envelope(&stream, &envelope).await {
                Ok(()) => {
                    write_guard.disarm();
                    return Ok(());
                }
                Err(error) => {
                    remove_cached_stream(self.inner.streams.clone(), module, stream).await;
                    write_guard.disarm();
                    warn!(
                        module = ?module,
                        attempt,
                        %error,
                        "failed to write WebTransport realtime frame; dropped cached module stream"
                    );
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| RealtimeError::new("Failed to write realtime frame.")))
    }

    async fn write_webtransport_one_shot(
        &self,
        envelope: RealtimeEnvelope,
        session: Session,
    ) -> Result<(), RealtimeError> {
        let module = envelope.module;
        let mut last_error = None;

        for attempt in 0..2 {
            let (send, recv) = session.open_bi().await.map_err(|error| {
                RealtimeError::new(format!("Failed to open realtime stream: {error}"))
            })?;
            let send = Rc::new(Mutex::new(send));
            debug!(module = ?module, "opened one-shot WebTransport realtime stream");
            webtransport::spawn_stream_reader(module, recv, self.inner.inbound.clone(), None);

            match framing::write_envelope(&send, &envelope).await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    warn!(
                        module = ?module,
                        attempt,
                        %error,
                        "failed to write one-shot WebTransport realtime frame"
                    );
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| RealtimeError::new("Failed to write realtime frame.")))
    }

    async fn stream_for(
        &self,
        module: RealtimeModule,
        session: Session,
    ) -> Result<Rc<Mutex<SendStream>>, RealtimeError> {
        if let Some(stream) = self.inner.streams.lock().await.get(&module).cloned() {
            return Ok(stream);
        }

        let (send, recv) = session.open_bi().await.map_err(|error| {
            RealtimeError::new(format!("Failed to open realtime stream: {error}"))
        })?;
        let send = Rc::new(Mutex::new(send));
        self.inner.streams.lock().await.insert(module, send.clone());
        debug!(module = ?module, "opened WebTransport realtime stream");
        webtransport::spawn_stream_reader(
            module,
            recv,
            self.inner.inbound.clone(),
            Some((self.inner.streams.clone(), send.clone())),
        );

        Ok(send)
    }

    fn next_generation(&self) -> u64 {
        let generation = self.inner.generation.get().saturating_add(1);
        self.inner.generation.set(generation);
        generation
    }

    /// Marks the current realtime connection as disconnected.
    pub(crate) async fn mark_disconnected(&self) {
        self.inner.session.lock().await.take();
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.set_connection_status(RealtimeConnectionStatus::Disconnected);
    }

    pub(super) async fn clear_generation(&self, generation: u64) {
        let mut session = self.inner.session.lock().await;
        let should_clear = session
            .as_ref()
            .is_some_and(|connected| connected.generation == generation);
        if should_clear {
            session.take();
            drop(session);
            self.inner.streams.lock().await.clear();
            self.inner.pending.borrow_mut().clear();
            self.set_connection_status(RealtimeConnectionStatus::Disconnected);
        }
    }

    fn set_connection_status(&self, status: RealtimeConnectionStatus) {
        if self.inner.connection_status.get() == status {
            return;
        }
        self.inner.connection_status.set(status);
        self.inner
            .status_listeners
            .borrow_mut()
            .retain(|listener| listener.unbounded_send(status).is_ok());
    }
}

/// Creates a disconnected realtime handle.
pub(crate) fn create_handle() -> RealtimeHandle {
    let (inbound, receiver) = mpsc::unbounded();
    let handle = RealtimeHandle {
        inner: Rc::new(RealtimeInner {
            session: Mutex::new(None),
            streams: Rc::new(Mutex::new(HashMap::new())),
            pending: Rc::new(RefCell::new(HashMap::new())),
            event_listeners: Rc::new(RefCell::new(Vec::new())),
            datagram_listeners: Rc::new(RefCell::new(Vec::new())),
            inbound,
            generation: Cell::new(0),
            connection_status: Cell::new(RealtimeConnectionStatus::Disconnected),
            status_listeners: Rc::new(RefCell::new(Vec::new())),
        }),
    };
    spawn_universal_reader(
        handle.inner.pending.clone(),
        handle.inner.event_listeners.clone(),
        receiver,
    );

    handle
}

fn validate_module_kind(module: RealtimeModule, kind: RealtimeKind) -> Result<(), RealtimeError> {
    if kind.module() == module {
        Ok(())
    } else {
        Err(RealtimeError::new(
            "Realtime module and kind do not belong together.",
        ))
    }
}

fn uses_cached_stream(module: RealtimeModule) -> bool {
    matches!(
        module,
        RealtimeModule::Control
            | RealtimeModule::Network
            | RealtimeModule::TextChat
            | RealtimeModule::VoiceChat
    )
}
