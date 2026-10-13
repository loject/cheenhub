use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use bytes::Bytes;
use cheenhub_contracts::realtime::{NetworkKind, RealtimeKind, RealtimeModule};
use dashmap::DashMap;
use serde::{Serialize, Serializer};
use uuid::Uuid;

use super::{RealtimeHub, RealtimeSession, RealtimeStream};
use crate::realtime::sink::{DatagramSink, EnvelopeSink, WebSocketOutbound};

struct CountingPayload(Arc<AtomicUsize>);

impl Serialize for CountingPayload {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        serializer.serialize_str("one serialization")
    }
}

#[test]
fn session_registry_is_sharded_by_session_id() {
    fn assert_sharded_registry(_: &DashMap<Uuid, RealtimeSession>) {}

    let hub = RealtimeHub::default();

    assert_sharded_registry(&hub.sessions);
}

#[tokio::test]
async fn fanout_serializes_payload_once_for_multiple_recipients() {
    let hub = RealtimeHub::default();
    let first_stream = Uuid::new_v4();
    let second_stream = Uuid::new_v4();
    let (first_sender, mut first_receiver) = tokio::sync::mpsc::channel(1);
    let (second_sender, mut second_receiver) = tokio::sync::mpsc::channel(1);
    for (stream_id, sender) in [(first_stream, first_sender), (second_stream, second_sender)] {
        hub.register_stream(
            stream_id,
            RealtimeModule::Network,
            Uuid::new_v4(),
            EnvelopeSink::websocket(sender),
        )
        .await;
    }

    let count = Arc::new(AtomicUsize::new(0));
    hub.fanout_to_streams(
        RealtimeModule::Network,
        &Uuid::new_v4(),
        RealtimeKind::Network(NetworkKind::Ping),
        &[first_stream, second_stream],
        CountingPayload(count.clone()),
    )
    .await;

    assert_eq!(count.load(Ordering::SeqCst), 1);
    for receiver in [&mut first_receiver, &mut second_receiver] {
        let Ok(WebSocketOutbound::PreparedEnvelope(text)) = receiver.try_recv() else {
            panic!("событие должно попасть в очередь каждого получателя");
        };
        let envelope: cheenhub_contracts::realtime::RealtimeEnvelope =
            serde_json::from_str(text.as_str()).expect("корректный общий конверт");
        assert_eq!(envelope.payload, serde_json::json!("one serialization"));
    }
}

#[test]
fn stream_registry_is_sharded_by_stream_id() {
    fn assert_sharded_registry(_: &DashMap<Uuid, RealtimeStream>) {}

    assert_sharded_registry(&RealtimeHub::default().streams);
}

#[tokio::test]
async fn stream_registry_filters_module_and_ignores_duplicate_registration() {
    let hub = RealtimeHub::default();
    let stream_id = Uuid::new_v4();
    let other_stream_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
    let (other_sender, mut other_receiver) = tokio::sync::mpsc::channel(2);

    hub.register_stream(
        stream_id,
        cheenhub_contracts::realtime::RealtimeModule::Network,
        user_id,
        crate::realtime::EnvelopeSink::websocket(sender),
    )
    .await;
    hub.register_stream(
        other_stream_id,
        cheenhub_contracts::realtime::RealtimeModule::TextChat,
        Uuid::new_v4(),
        crate::realtime::EnvelopeSink::websocket(other_sender),
    )
    .await;
    let (duplicate_sender, _duplicate_receiver) = tokio::sync::mpsc::channel(1);
    hub.register_stream(
        stream_id,
        cheenhub_contracts::realtime::RealtimeModule::TextChat,
        Uuid::new_v4(),
        crate::realtime::EnvelopeSink::websocket(duplicate_sender),
    )
    .await;

    let recipients = hub.module_recipients(cheenhub_contracts::realtime::RealtimeModule::Network);
    assert_eq!(recipients.len(), 1);
    assert_eq!(recipients[0].stream_id, stream_id);
    assert_eq!(recipients[0].user_id, user_id);
    assert_eq!(
        hub.module_recipients(cheenhub_contracts::realtime::RealtimeModule::TextChat)
            .len(),
        1
    );

    hub.fanout_to_streams(
        cheenhub_contracts::realtime::RealtimeModule::Network,
        &Uuid::nil(),
        cheenhub_contracts::realtime::RealtimeKind::Network(
            cheenhub_contracts::realtime::NetworkKind::Ping,
        ),
        &[stream_id, stream_id, other_stream_id, Uuid::new_v4()],
        cheenhub_contracts::realtime::Ping { sent_at_ms: 7 },
    )
    .await;

    assert!(matches!(
        receiver.try_recv(),
        Ok(WebSocketOutbound::PreparedEnvelope(_))
    ));
    assert!(receiver.try_recv().is_err());
    assert!(other_receiver.try_recv().is_err());

    hub.unregister_stream(stream_id).await;
    assert!(
        hub.module_recipients(cheenhub_contracts::realtime::RealtimeModule::Network)
            .is_empty()
    );
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
