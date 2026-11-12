//! Guard'ы отмены для realtime-запросов и записи в потоки.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::{
    ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule, Rejected, RejectionCode,
};
use dioxus::prelude::{debug, warn};
use futures_channel::oneshot;
use futures_util::lock::Mutex;
use uuid::Uuid;
use web_transport::SendStream;

use super::task::spawn_task;

pub(super) type PendingKey = (RealtimeModule, Uuid);
/// Ожидаемый ответ принадлежит одному конкретному надежному потоку.
pub(super) struct PendingRequest {
    /// Канал доставки ответа вызывающему коду.
    pub sender: oneshot::Sender<RealtimeEnvelope>,
    /// Идентификатор WebTransport-потока; None используется для WebSocket.
    pub stream_id: Option<Uuid>,
}

pub(super) type PendingRequests = Rc<RefCell<HashMap<PendingKey, PendingRequest>>>;

/// Запись кеша с собственной идентичностью для изолированной очистки.
#[derive(Clone)]
pub(super) struct CachedStream {
    /// Идентификатор экземпляра потока, не переиспользуемый после восстановления.
    pub id: Uuid,
    /// Последовательно записываемая половина двунаправленного потока.
    pub send: Rc<Mutex<SendStream>>,
}

pub(super) type ModuleStreams = Rc<Mutex<HashMap<RealtimeModule, CachedStream>>>;

pub(super) struct PendingRequestGuard {
    pending: PendingRequests,
    key: PendingKey,
    active: Cell<bool>,
}

impl PendingRequestGuard {
    pub(super) fn new(pending: PendingRequests, key: PendingKey) -> Self {
        Self {
            pending,
            key,
            active: Cell::new(true),
        }
    }

    pub(super) fn disarm(&self) {
        self.active.set(false);
    }
}

impl Drop for PendingRequestGuard {
    fn drop(&mut self) {
        if self.active.get() {
            self.pending.borrow_mut().remove(&self.key);
            debug!(
                module = ?self.key.0,
                request_id = %self.key.1,
                "canceled pending realtime request"
            );
        }
    }
}

pub(super) struct StreamWriteGuard {
    module: RealtimeModule,
    streams: ModuleStreams,
    stream: Rc<Mutex<SendStream>>,
    pending: PendingRequests,
    stream_id: Uuid,
    active: Cell<bool>,
}

impl StreamWriteGuard {
    pub(super) fn new(
        module: RealtimeModule,
        streams: ModuleStreams,
        stream: Rc<Mutex<SendStream>>,
        pending: PendingRequests,
        stream_id: Uuid,
    ) -> Self {
        Self {
            module,
            streams,
            stream,
            pending,
            stream_id,
            active: Cell::new(true),
        }
    }

    pub(super) fn disarm(&self) {
        self.active.set(false);
    }
}

impl Drop for StreamWriteGuard {
    fn drop(&mut self) {
        if !self.active.get() {
            return;
        }

        reject_pending_requests_for_stream(
            &self.pending,
            self.module,
            self.stream_id,
            "Realtime stream write was cancelled.",
        );
        let module = self.module;
        let stream_id = self.stream_id;
        let streams = self.streams.clone();
        let stream = self.stream.clone();
        remove_cached_stream_now(&streams, module, &stream);
        spawn_task(async move {
            remove_cached_stream(streams, module, stream.clone()).await;
            // Нельзя оставлять на backend полузаписанный фрейм: сбрасываем
            // send-половину даже после отмены future во время записи.
            stream.lock().await.reset(0);
            warn!(?module, %stream_id, "aborted cached WebTransport stream after cancelled write");
        });
    }
}

pub(super) async fn remove_cached_stream(
    streams: ModuleStreams,
    module: RealtimeModule,
    stream: Rc<Mutex<SendStream>>,
) {
    let mut streams = streams.lock().await;
    let should_remove = streams
        .get(&module)
        .is_some_and(|current| Rc::ptr_eq(&current.send, &stream));
    if should_remove {
        streams.remove(&module);
    }
}

/// Отклоняет только запросы, отправленные через закрытый экземпляр потока.
pub(super) fn reject_pending_requests_for_stream(
    pending: &PendingRequests,
    module: RealtimeModule,
    stream_id: Uuid,
    message: &str,
) {
    let keys = pending
        .borrow()
        .iter()
        .filter(|((pending_module, _), request)| {
            *pending_module == module && request.stream_id == Some(stream_id)
        })
        .map(|(key, _)| *key)
        .collect::<Vec<_>>();

    if !keys.is_empty() {
        debug!(?module, %stream_id, pending_count = keys.len(), "rejecting requests belonging to closed realtime stream");
    }
    for key in keys {
        reject_pending_request(pending, key, message);
    }
}

pub(super) fn reject_pending_request(pending: &PendingRequests, key: PendingKey, message: &str) {
    let Some(sender) = pending.borrow_mut().remove(&key) else {
        return;
    };
    let payload = Rejected {
        code: RejectionCode::InternalError,
        message: message.to_owned(),
    };
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Rejected),
        Some(key.1),
        payload,
    );
    if let Ok(envelope) = envelope {
        let _ = sender.sender.send(envelope);
    }
}

fn remove_cached_stream_now(
    streams: &ModuleStreams,
    module: RealtimeModule,
    stream: &Rc<Mutex<SendStream>>,
) -> bool {
    let Some(mut streams) = streams.try_lock() else {
        return false;
    };
    let should_remove = streams
        .get(&module)
        .is_some_and(|current| Rc::ptr_eq(&current.send, stream));
    if should_remove {
        streams.remove(&module);
    }

    should_remove
}

#[cfg(test)]
mod tests;
