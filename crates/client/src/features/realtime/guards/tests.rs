use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cheenhub_contracts::realtime::RealtimeModule;
use futures_channel::oneshot;
use uuid::Uuid;

use super::{
    PendingRequest, PendingRequestGuard, PendingRequests, reject_pending_requests_for_stream,
};

#[test]
fn dropping_pending_request_guard_removes_its_receiver() {
    let pending: PendingRequests = Rc::new(RefCell::new(HashMap::new()));
    let key = (RealtimeModule::TextChat, Uuid::new_v4());
    let (sender, _receiver) = oneshot::channel();
    pending.borrow_mut().insert(
        key,
        PendingRequest {
            sender,
            stream_id: None,
        },
    );

    drop(PendingRequestGuard::new(pending.clone(), key));

    assert!(!pending.borrow().contains_key(&key));
}

#[test]
fn closed_stream_rejects_only_its_own_pending_requests() {
    let pending: PendingRequests = Rc::new(RefCell::new(HashMap::new()));
    let old_stream = Uuid::new_v4();
    let new_stream = Uuid::new_v4();
    let old_key = (RealtimeModule::Social, Uuid::new_v4());
    let new_key = (RealtimeModule::Social, Uuid::new_v4());
    let other_key = (RealtimeModule::TextChat, Uuid::new_v4());
    for (key, stream_id) in [
        (old_key, old_stream),
        (new_key, new_stream),
        (other_key, old_stream),
    ] {
        let (sender, _receiver) = oneshot::channel();
        pending.borrow_mut().insert(
            key,
            PendingRequest {
                sender,
                stream_id: Some(stream_id),
            },
        );
    }

    reject_pending_requests_for_stream(
        &pending,
        RealtimeModule::Social,
        old_stream,
        "Old stream closed.",
    );

    assert!(!pending.borrow().contains_key(&old_key));
    assert!(pending.borrow().contains_key(&new_key));
    assert!(pending.borrow().contains_key(&other_key));
}
