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
