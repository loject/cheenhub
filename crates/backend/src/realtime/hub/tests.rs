use bytes::Bytes;
use dashmap::DashMap;
use uuid::Uuid;

use super::{RealtimeHub, RealtimeSession};
use crate::realtime::sink::{DatagramSink, WebSocketOutbound};

#[test]
fn session_registry_is_sharded_by_session_id() {
    fn assert_sharded_registry(_: &DashMap<Uuid, RealtimeSession>) {}

    let hub = RealtimeHub::default();

    assert_sharded_registry(&hub.sessions);
}

#[tokio::test]
async fn datagram_fanout_sends_only_to_requested_sessions() {
    let hub = RealtimeHub::default();
    let selected_session_id = Uuid::new_v4();
    let other_session_id = Uuid::new_v4();
    let (selected_sender, mut selected_receiver) = tokio::sync::mpsc::channel(1);
    let (other_sender, mut other_receiver) = tokio::sync::mpsc::channel(1);

    hub.register_session(
        selected_session_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        DatagramSink::websocket(selected_sender),
    )
    .await;
    hub.register_session(
        other_session_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        DatagramSink::websocket(other_sender),
    )
    .await;

    hub.fanout_datagram_to_sessions_except(
        &[selected_session_id],
        Uuid::nil(),
        Bytes::from_static(b"frame"),
    )
    .await;

    assert!(matches!(
        selected_receiver.try_recv(),
        Ok(WebSocketOutbound::Datagram(bytes)) if bytes == Bytes::from_static(b"frame")
    ));
    assert!(other_receiver.try_recv().is_err());
}

#[tokio::test]
async fn datagram_fanout_does_not_echo_to_the_sender() {
    let hub = RealtimeHub::default();
    let sender_session_id = Uuid::new_v4();
    let recipient_session_id = Uuid::new_v4();
    let (sender, mut sender_receiver) = tokio::sync::mpsc::channel(1);
    let (recipient, mut recipient_receiver) = tokio::sync::mpsc::channel(1);

    hub.register_session(
        sender_session_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        DatagramSink::websocket(sender),
    )
    .await;
    hub.register_session(
        recipient_session_id,
        Uuid::new_v4(),
        Uuid::new_v4(),
        DatagramSink::websocket(recipient),
    )
    .await;

    hub.fanout_datagram_to_sessions_except(
        &[sender_session_id, recipient_session_id],
        sender_session_id,
        Bytes::from_static(b"frame"),
    )
    .await;

    assert!(sender_receiver.try_recv().is_err());
    assert!(matches!(
        recipient_receiver.try_recv(),
        Ok(WebSocketOutbound::Datagram(bytes)) if bytes == Bytes::from_static(b"frame")
    ));
}
