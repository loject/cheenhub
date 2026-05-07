//! Realtime connection handle.

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
use futures_util::{StreamExt, lock::Mutex};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;
use web_transport::{RecvStream, SendStream, Session};

use super::config;
use super::error::RealtimeError;
use super::framing;
use super::status::RealtimeConnectionStatus;
use super::task::spawn_task;

type PendingKey = (RealtimeModule, Uuid);
type PendingRequests = Rc<RefCell<HashMap<PendingKey, oneshot::Sender<RealtimeEnvelope>>>>;
type ModuleStreams = Rc<Mutex<HashMap<RealtimeModule, Rc<Mutex<SendStream>>>>>;
type EventListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<RealtimeEnvelope>>>>;
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
    inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
    generation: Cell<u64>,
    connection_status: Cell<RealtimeConnectionStatus>,
    status_listeners: StatusListeners,
}

#[derive(Clone)]
struct ConnectedSession {
    generation: u64,
    session: Session,
}

impl RealtimeHandle {
    /// Opens and authenticates the realtime session.
    pub(crate) async fn connect(
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
            session: session.clone(),
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
        self.set_connection_status(RealtimeConnectionStatus::Connected);
        spawn_connection_watcher(url.to_string(), session, generation, self.clone());

        Ok(authenticated)
    }

    /// Sends one reliable fire-and-forget message.
    pub(crate) async fn send_reliable<P>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<(), RealtimeError>
    where
        P: Serialize,
    {
        validate_module_kind(module, kind)?;
        let envelope = RealtimeEnvelope::new(module, kind, None, payload).map_err(|error| {
            RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
        })?;
        self.write_envelope(envelope).await
    }

    /// Sends one unreliable datagram message.
    pub(crate) async fn send_unreliable<P>(
        &self,
        module: RealtimeModule,
        kind: RealtimeKind,
        payload: P,
    ) -> Result<(), RealtimeError>
    where
        P: Serialize,
    {
        validate_module_kind(module, kind)?;
        let envelope = RealtimeEnvelope::new(module, kind, None, payload).map_err(|error| {
            RealtimeError::new(format!("Failed to encode realtime payload: {error}"))
        })?;
        let bytes = serde_json::to_vec(&envelope).map_err(|error| {
            RealtimeError::new(format!("Failed to encode realtime datagram: {error}"))
        })?;
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };

        connected
            .session
            .send_datagram(Bytes::from(bytes))
            .await
            .map_err(|error| {
                RealtimeError::new(format!("Failed to send realtime datagram: {error}"))
            })
    }

    /// Subscribes to inbound fire-and-forget realtime events for this tab.
    pub(crate) fn subscribe_events(&self) -> mpsc::UnboundedReceiver<RealtimeEnvelope> {
        let (sender, receiver) = mpsc::unbounded();
        self.inner.event_listeners.borrow_mut().push(sender);

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

        if let Err(error) = self.write_envelope(envelope).await {
            self.inner
                .pending
                .borrow_mut()
                .remove(&(module, request_id));
            return Err(error);
        }

        let response = receiver
            .await
            .map_err(|_| RealtimeError::new("Realtime response channel closed."))?;
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
        let stream = self.stream_for(envelope.module).await?;
        framing::write_envelope(&stream, &envelope).await
    }

    async fn stream_for(
        &self,
        module: RealtimeModule,
    ) -> Result<Rc<Mutex<SendStream>>, RealtimeError> {
        if let Some(stream) = self.inner.streams.lock().await.get(&module).cloned() {
            return Ok(stream);
        }

        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };
        let (send, recv) = connected.session.open_bi().await.map_err(|error| {
            RealtimeError::new(format!("Failed to open realtime stream: {error}"))
        })?;
        let send = Rc::new(Mutex::new(send));
        self.inner.streams.lock().await.insert(module, send.clone());
        debug!(module = ?module, "opened WebTransport realtime stream");
        spawn_stream_reader(module, recv, self.inner.inbound.clone());

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

    async fn clear_generation(&self, generation: u64) {
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

fn spawn_stream_reader(
    module: RealtimeModule,
    mut recv: RecvStream,
    inbound: mpsc::UnboundedSender<RealtimeEnvelope>,
) {
    spawn_task(async move {
        loop {
            let envelope = match framing::read_envelope(&mut recv).await {
                Ok(Some(envelope)) => envelope,
                Ok(None) => {
                    debug!(module = ?module, "WebTransport realtime stream closed by peer");
                    break;
                }
                Err(error) => {
                    warn!(module = ?module, %error, "WebTransport realtime stream read failed");
                    break;
                }
            };

            if !envelope.has_matching_module_kind()
                || (envelope.module != module && !is_rejection(&envelope))
            {
                warn!(
                    module = ?module,
                    envelope_module = ?envelope.module,
                    envelope_kind = ?envelope.kind,
                    "closing realtime stream after mismatched envelope"
                );
                break;
            }
            if inbound.unbounded_send(envelope).is_err() {
                debug!(module = ?module, "realtime inbound dispatcher closed");
                break;
            }
        }
    });
}

fn spawn_connection_watcher(
    url: String,
    session: Session,
    generation: u64,
    realtime: RealtimeHandle,
) {
    spawn_task(async move {
        let error = session.closed().await;
        info!(%url, %generation, %error, "WebTransport realtime session closed");
        realtime.clear_generation(generation).await;
    });
}

fn spawn_universal_reader(
    pending: PendingRequests,
    event_listeners: EventListeners,
    mut receiver: mpsc::UnboundedReceiver<RealtimeEnvelope>,
) {
    spawn_task(async move {
        while let Some(envelope) = receiver.next().await {
            if envelope.request_id.is_none() {
                dispatch_event(&event_listeners, envelope);
                continue;
            }
            let Some(request_id) = envelope.request_id else {
                continue;
            };
            let key = (envelope.module, request_id);
            if let Some(sender) = pending.borrow_mut().remove(&key) {
                let _ = sender.send(envelope);
            } else if is_rejection(&envelope) {
                let key = pending
                    .borrow()
                    .keys()
                    .find(|(_, pending_request_id)| *pending_request_id == request_id)
                    .copied();
                if let Some(key) = key
                    && let Some(sender) = pending.borrow_mut().remove(&key)
                {
                    let _ = sender.send(envelope);
                }
            }
        }
    });
}

fn dispatch_event(event_listeners: &EventListeners, envelope: RealtimeEnvelope) {
    event_listeners
        .borrow_mut()
        .retain(|listener| listener.unbounded_send(envelope.clone()).is_ok());
}

fn is_rejection(envelope: &RealtimeEnvelope) -> bool {
    envelope.module == RealtimeModule::Control
        && envelope.kind == RealtimeKind::Control(ControlKind::Rejected)
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
