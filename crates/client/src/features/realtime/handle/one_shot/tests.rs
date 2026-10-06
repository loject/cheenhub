use cheenhub_contracts::realtime::{NetworkKind, Ping};

use super::*;

#[test]
fn rejects_response_for_another_request_without_routing_it_to_pending() {
    let request_id = Uuid::new_v4();
    let response = RealtimeEnvelope::new(
        RealtimeModule::Network,
        RealtimeKind::Network(NetworkKind::Ping),
        Some(Uuid::new_v4()),
        Ping { sent_at_ms: 1 },
    )
    .expect("response payload serializes");

    assert!(validate_response(RealtimeModule::Network, request_id, &response).is_err());
}
