//! Inbound realtime envelope dispatcher.

use std::cell::RefCell;
use std::rc::Rc;

use cheenhub_contracts::realtime::{ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule};
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
