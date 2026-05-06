//! Shared realtime WebTransport contracts.

mod control;
mod envelope;
mod network;

pub use control::{
    Authenticate, Authenticated, ControlAck, ControlKind, ControlText, Rejected, RejectionCode,
};
pub use envelope::{RealtimeEnvelope, RealtimeKind, RealtimeModule};
pub use network::{NetworkKind, Ping, Pong};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn envelope_round_trips_uuid_and_typed_kind() {
        let request_id = Uuid::new_v4();
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Network,
            RealtimeKind::Network(NetworkKind::Ping),
            Some(request_id),
            Ping { sent_at_ms: 42 },
        )
        .expect("payload serializes");

        let json = serde_json::to_string(&envelope).expect("envelope serializes");
        assert!(json.contains("\"module\":\"network\""));
        assert!(json.contains("\"kind\":\"ping\""));
        let decoded: RealtimeEnvelope = serde_json::from_str(&json).expect("envelope decodes");

        assert_eq!(decoded.request_id, Some(request_id));
        assert_eq!(decoded.kind, RealtimeKind::Network(NetworkKind::Ping));
        assert!(decoded.has_matching_module_kind());
    }

    #[test]
    fn module_kind_mismatch_is_detected() {
        let envelope = RealtimeEnvelope::new(
            RealtimeModule::Control,
            RealtimeKind::Network(NetworkKind::Ping),
            None,
            Ping { sent_at_ms: 42 },
        )
        .expect("payload serializes");

        assert!(!envelope.has_matching_module_kind());
    }
}
