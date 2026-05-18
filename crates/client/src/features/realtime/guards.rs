//! Cancellation guards for realtime requests and stream writes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::{RealtimeEnvelope, RealtimeModule};
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
