//! Guard'ы отмены для realtime-запросов и записи в потоки.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::{
    ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule, Rejected, RejectionCode,
};
use dioxus::prelude::warn;
use futures_channel::oneshot;
use futures_util::lock::Mutex;
use uuid::Uuid;
use web_transport::SendStream;

use super::task::spawn_task;

pub(super) type PendingKey = (RealtimeModule, Uuid);
pub(super) type PendingRequests =
    Rc<RefCell<HashMap<PendingKey, oneshot::Sender<RealtimeEnvelope>>>>;
pub(super) type ModuleStreams = Rc<Mutex<HashMap<RealtimeModule, Rc<Mutex<SendStream>>>>>;

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
        }
    }
}

pub(super) struct StreamWriteGuard {
    module: RealtimeModule,
    streams: ModuleStreams,
    stream: Rc<Mutex<SendStream>>,
    active: Cell<bool>,
}

impl StreamWriteGuard {
    pub(super) fn new(
        module: RealtimeModule,
        streams: ModuleStreams,
        stream: Rc<Mutex<SendStream>>,
    ) -> Self {
        Self {
            module,
            streams,
            stream,
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

        let module = self.module;
        let streams = self.streams.clone();
        let stream = self.stream.clone();
        if remove_cached_stream_now(&streams, module, &stream) {
            warn!(
                module = ?module,
                "dropped cached WebTransport realtime stream after canceled frame write"
            );
            return;
        }

        spawn_task(async move {
            remove_cached_stream(streams, module, stream).await;
            warn!(
                module = ?module,
                "dropped cached WebTransport realtime stream after canceled frame write"
            );
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
        .is_some_and(|current| Rc::ptr_eq(current, &stream));
    if should_remove {
        streams.remove(&module);
    }
}

pub(super) fn reject_pending_requests_for_module(
    pending: &PendingRequests,
    module: RealtimeModule,
    message: &str,
) {
    let keys = pending
        .borrow()
        .keys()
        .filter(|(pending_module, _)| *pending_module == module)
        .copied()
        .collect::<Vec<_>>();

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
        let _ = sender.send(envelope);
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
        .is_some_and(|current| Rc::ptr_eq(current, stream));
    if should_remove {
        streams.remove(&module);
    }

    should_remove
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use cheenhub_contracts::realtime::RealtimeModule;
    use futures_channel::oneshot;
    use uuid::Uuid;

    use super::{PendingRequestGuard, PendingRequests};

    #[test]
    fn dropping_pending_request_guard_removes_its_receiver() {
        let pending: PendingRequests = Rc::new(RefCell::new(HashMap::new()));
        let key = (RealtimeModule::TextChat, Uuid::new_v4());
        let (sender, _receiver) = oneshot::channel();
        pending.borrow_mut().insert(key, sender);

        drop(PendingRequestGuard::new(pending.clone(), key));

        assert!(!pending.borrow().contains_key(&key));
    }
}
