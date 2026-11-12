use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cheenhub_contracts::realtime::{
    ControlKind, RealtimeEnvelope, RealtimeKind, RealtimeModule, Rejected, RejectionCode,
    SocialKind, SocialReady,
};
use futures_channel::oneshot;
use futures_util::FutureExt;
use uuid::Uuid;

use super::deliver_pending_response;
use crate::features::realtime::guards::{PendingRequest, PendingRequests};

fn pending() -> PendingRequests {
    Rc::new(RefCell::new(HashMap::new()))
}

fn ready(id: Uuid) -> RealtimeEnvelope {
    RealtimeEnvelope::new(
        RealtimeModule::Social,
        RealtimeKind::Social(SocialKind::Ready),
        Some(id),
        SocialReady,
    )
    .unwrap()
}

#[test]
fn response_from_replaced_stream_cannot_consume_new_stream_request() {
    let pending = pending();
    let old_stream = Uuid::new_v4();
    let current_stream = Uuid::new_v4();
    let id = Uuid::new_v4();
    let (sender, mut receiver) = oneshot::channel();
    pending.borrow_mut().insert(
        (RealtimeModule::Social, id),
        PendingRequest {
            sender,
            stream_id: Some(current_stream),
        },
    );

    assert!(!deliver_pending_response(
        &pending,
        ready(id),
        Some(old_stream)
    ));
    assert!((&mut receiver).now_or_never().is_none());
    assert!(pending.borrow().contains_key(&(RealtimeModule::Social, id)));
    assert!(deliver_pending_response(
        &pending,
        ready(id),
        Some(current_stream)
    ));
    assert_eq!(
        (&mut receiver).now_or_never().unwrap().unwrap().request_id,
        Some(id)
    );
}

#[test]
fn websocket_response_uses_unscoped_request_and_webtransport_cannot_take_it() {
    let pending = pending();
    let id = Uuid::new_v4();
    let (sender, mut receiver) = oneshot::channel();
    pending.borrow_mut().insert(
        (RealtimeModule::Social, id),
        PendingRequest {
            sender,
            stream_id: None,
        },
    );

    assert!(!deliver_pending_response(
        &pending,
        ready(id),
        Some(Uuid::new_v4())
    ));
    assert!(deliver_pending_response(&pending, ready(id), None));
    assert!((&mut receiver).now_or_never().unwrap().is_ok());
}

#[test]
fn control_rejection_finds_original_request_only_on_matching_stream() {
    let pending = pending();
    let id = Uuid::new_v4();
    let stream = Uuid::new_v4();
    let (sender, mut receiver) = oneshot::channel();
    pending.borrow_mut().insert(
        (RealtimeModule::Social, id),
        PendingRequest {
            sender,
            stream_id: Some(stream),
        },
    );
    let rejected = RealtimeEnvelope::new(
        RealtimeModule::Control,
        RealtimeKind::Control(ControlKind::Rejected),
        Some(id),
        Rejected {
            code: RejectionCode::BadRequest,
            message: "Отклонено".into(),
        },
    )
    .unwrap();

    assert!(!deliver_pending_response(
        &pending,
        rejected.clone(),
        Some(Uuid::new_v4())
    ));
    assert!(deliver_pending_response(&pending, rejected, Some(stream)));
    assert_eq!(
        (&mut receiver).now_or_never().unwrap().unwrap().request_id,
        Some(id)
    );
}
