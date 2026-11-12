//! Диспетчер входящих realtime-конвертов.

use std::cell::RefCell;
use std::rc::Rc;

use cheenhub_contracts::realtime::{ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule};
use dioxus::prelude::debug;
use futures_channel::mpsc;
use futures_util::StreamExt;

use super::guards::PendingRequests;
use super::task::spawn_task;

pub(super) type EventListeners = Rc<RefCell<Vec<mpsc::UnboundedSender<RealtimeEnvelope>>>>;

pub(super) fn spawn_universal_reader(
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
            let request_id = envelope.request_id;
            let module = envelope.module;
            let kind = envelope.kind;
            if !deliver_pending_response(&pending, envelope, None) {
                debug!(
                    ?module,
                    ?kind,
                    ?request_id,
                    "received realtime response without matching pending receiver"
                );
            }
        }
    });
}

/// Доставляет ответ синхронно только запросу, принадлежащему указанному потоку.
///
/// Вызов непосредственно из WebTransport-reader исключает гонку между закрытием
/// потока и обработкой уже прочитанного ответа общей очередью событий.
pub(super) fn deliver_pending_response(
    pending: &PendingRequests,
    envelope: RealtimeEnvelope,
    stream_id: Option<uuid::Uuid>,
) -> bool {
    let Some(request_id) = envelope.request_id else {
        return false;
    };
    let key = (envelope.module, request_id);
    let mut requests = pending.borrow_mut();
    let matching_key = if requests
        .get(&key)
        .is_some_and(|request| request.stream_id == stream_id)
    {
        Some(key)
    } else if is_rejection(&envelope) {
        requests
            .iter()
            .find(|((_, id), request)| *id == request_id && request.stream_id == stream_id)
            .map(|(key, _)| *key)
    } else {
        None
    };
    let Some(matching_key) = matching_key else {
        return false;
    };
    if let Some(request) = requests.remove(&matching_key) {
        let _ = request.sender.send(envelope);
        true
    } else {
        false
    }
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

#[cfg(test)]
mod tests;
