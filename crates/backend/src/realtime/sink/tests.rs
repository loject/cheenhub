use cheenhub_contracts::realtime::{NetworkKind, Ping, RealtimeKind, RealtimeModule};

use super::*;

#[tokio::test]
async fn websocket_envelope_reports_full_outbound_queue() {
    let (sender, _receiver) = mpsc::channel(1);
    sender
        .try_send(WebSocketOutbound::Datagram(Bytes::from_static(b"occupied")))
        .expect("очередь принимает первое сообщение");
    let sink = EnvelopeSink::websocket(sender);
    let envelope = RealtimeEnvelope::new(
        RealtimeModule::Network,
        RealtimeKind::Network(NetworkKind::Ping),
        None,
        Ping { sent_at_ms: 1 },
    )
    .expect("конверт сериализуется");

    let error = sink
        .send_envelope(&envelope)
        .await
        .expect_err("переполненная очередь отклоняет надёжное сообщение");

    assert!(error.to_string().contains("outbound queue is full"));
}

#[tokio::test]
async fn websocket_datagram_is_dropped_when_outbound_queue_is_full() {
    let (sender, mut receiver) = mpsc::channel(1);
    sender
        .try_send(WebSocketOutbound::Datagram(Bytes::from_static(b"first")))
        .expect("очередь принимает первую датаграмму");
    let sink = DatagramSink::websocket(sender);

    sink.send_datagram(Bytes::from_static(b"second"))
        .await
        .expect("переполнение не замедляет fanout");

    let Some(WebSocketOutbound::Datagram(bytes)) = receiver.recv().await else {
        panic!("в очереди должна остаться первая датаграмма");
    };
    assert_eq!(bytes, Bytes::from_static(b"first"));
    assert!(receiver.try_recv().is_err());
}
