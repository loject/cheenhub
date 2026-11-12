//! Realtime connection handle.

mod connection;
mod fallback;
mod fire_and_forget;
mod lifecycle;
mod one_shot;
mod requests;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use bytes::Bytes;
use cheenhub_contracts::realtime::{
    ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule, Rejected,
};
use dioxus::prelude::{debug, warn};
use futures_channel::{mpsc, oneshot};
use futures_util::lock::Mutex;
use serde::de::DeserializeOwned;
use uuid::Uuid;
use web_time::{Instant, SystemTime, UNIX_EPOCH};
use web_transport::Session;

use super::error::RealtimeError;
use super::guards::{ModuleStreams, PendingRequests};
use super::inbound::{EventListeners, spawn_universal_reader};
use super::status::{RealtimeConnectionStatus, RealtimeFallbackInfo, RealtimeTransportKind};
use super::websocket::{WebSocketOutbound, WebSocketOutboundSender};
use super::webtransport;

const SLOW_DATAGRAM_SEND_WARN_AFTER: Duration = Duration::from_millis(40);
const DATAGRAM_SEND_WARNING_INTERVAL_MS: u64 = 5_000;

pub(super) type DatagramListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<Bytes>>>>;
type StatusListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<RealtimeConnectionStatus>>>>;
type FallbackListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<Option<RealtimeFallbackInfo>>>>>;

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
    background_activity: super::activity::BackgroundActivity,
    reconnect_requests: RefCell<Option<mpsc::UnboundedSender<()>>>,
    force_websocket: Cell<bool>,
    connection_status: Cell<RealtimeConnectionStatus>,
    status_listeners: StatusListeners,
    fallback_info: Cell<Option<RealtimeFallbackInfo>>,
    fallback_listeners: FallbackListeners,
    last_slow_datagram_send_warning_ms: Cell<u64>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ReliableRequestMode {
    Cached,
    OneShot,
}

impl RealtimeHandle {
    /// Sends one raw unreliable datagram message.
    pub(crate) async fn send_unreliable_bytes(&self, bytes: Bytes) -> Result<(), RealtimeError> {
        let Some(connected) = self.inner.session.lock().await.clone() else {
            return Err(RealtimeError::new("Realtime session is not connected."));
        };
        let generation = connected.generation;
        let payload_bytes = bytes.len();

        match connected.transport {
            ConnectedTransport::WebTransport(session) => {
                // web-transport 0.12 gives each Session clone its own in-flight
                // datagram operation while sharing the browser writer/backpressure.
                // Do not serialize all media behind an application-level mutex:
                // that creates head-of-line blocking between voice and video.
                let session = session.as_ref().clone();
                let send_started_at = Instant::now();
                let result = session.send_datagram(bytes).await.map_err(|error| {
                    RealtimeError::new(format!("Failed to send realtime datagram: {error}"))
                });
                let send_elapsed = send_started_at.elapsed();
                if result.is_ok()
                    && send_elapsed >= SLOW_DATAGRAM_SEND_WARN_AFTER
                    && should_emit_cell_warning(
                        &self.inner.last_slow_datagram_send_warning_ms,
                        realtime_now_ms(),
                        DATAGRAM_SEND_WARNING_INTERVAL_MS,
                    )
                {
                    warn!(
                        generation,
                        payload_bytes,
                        send_elapsed_ms = send_elapsed.as_millis(),
                        "slow outbound realtime datagram send"
                    );
                }
                result
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

    /// Отмечает выполняемую попытку подключения выбранного realtime-транспорта.
    pub(crate) fn mark_connecting(&self, transport: RealtimeTransportKind) {
        let status = match transport {
            RealtimeTransportKind::WebTransport => RealtimeConnectionStatus::ConnectingWebTransport,
            RealtimeTransportKind::WebSocketFallback => {
                RealtimeConnectionStatus::ConnectingWebSocketFallback
            }
        };
        self.set_connection_status(status);
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

    fn next_generation(&self) -> u64 {
        let generation = self.inner.generation.get().saturating_add(1);
        self.inner.generation.set(generation);
        generation
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
            background_activity: Default::default(),
            reconnect_requests: RefCell::new(None),
            force_websocket: Cell::new(false),
            connection_status: Cell::new(RealtimeConnectionStatus::Disconnected),
            status_listeners: Rc::new(RefCell::new(Vec::new())),
            fallback_info: Cell::new(None),
            fallback_listeners: Rc::new(RefCell::new(Vec::new())),
            last_slow_datagram_send_warning_ms: Cell::new(0),
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

fn decode_response<R: DeserializeOwned>(response: RealtimeEnvelope) -> Result<R, RealtimeError> {
    if response.kind == RealtimeKind::Control(ControlKind::Rejected) {
        let rejected = serde_json::from_value::<Rejected>(response.payload).map_err(|error| {
            RealtimeError::new(format!("Failed to decode realtime rejection: {error}"))
        })?;
        return Err(RealtimeError::rejected(rejected.code, rejected.message));
    }
    serde_json::from_value(response.payload)
        .map_err(|error| RealtimeError::new(format!("Failed to decode realtime response: {error}")))
}

fn new_request_id() -> Uuid {
    Uuid::new_v4()
}

fn default_request_mode(module: RealtimeModule) -> ReliableRequestMode {
    if uses_cached_stream(module) {
        ReliableRequestMode::Cached
    } else {
        ReliableRequestMode::OneShot
    }
}

fn uses_cached_stream(module: RealtimeModule) -> bool {
    matches!(
        module,
        RealtimeModule::Control
            | RealtimeModule::Network
            | RealtimeModule::Social
            | RealtimeModule::TextChat
            | RealtimeModule::VoiceChat
    )
}

fn should_emit_cell_warning(last_warning_ms: &Cell<u64>, now_ms: u64, interval_ms: u64) -> bool {
    let last_ms = last_warning_ms.get();
    if last_ms != 0 && now_ms.saturating_sub(last_ms) < interval_ms {
        return false;
    }

    last_warning_ms.set(now_ms);
    true
}

fn realtime_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
