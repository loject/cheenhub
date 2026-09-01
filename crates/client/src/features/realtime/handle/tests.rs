use cheenhub_contracts::realtime::RealtimeModule;

use super::{ReliableRequestMode, default_request_mode, new_request_id};

#[test]
fn retry_attempt_uses_a_new_request_id() {
    assert_ne!(new_request_id(), new_request_id());
}

#[test]
fn server_requests_default_to_one_shot() {
    assert!(default_request_mode(RealtimeModule::Server) == ReliableRequestMode::OneShot);
}

#[test]
fn cached_modules_default_to_cached_stream() {
    assert!(default_request_mode(RealtimeModule::TextChat) == ReliableRequestMode::Cached);
}
