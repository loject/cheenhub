use super::*;
use crate::features::realtime::handle::{ConnectedSession, create_handle};
use dioxus::prelude::*;
use futures_util::{FutureExt, StreamExt};

#[test]
fn manual_reconnect_wakes_runtime_and_blocks_duplicate_requests() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        for status in [
            RealtimeConnectionStatus::Disconnected,
            RealtimeConnectionStatus::Connected(
                crate::features::realtime::RealtimeTransportKind::WebSocketFallback,
            ),
        ] {
            let realtime = create_handle();
            let mut requests = realtime.subscribe_reconnect_requests();
            realtime.set_connection_status(status);
            assert!(realtime.request_reconnect());
            assert_eq!(
                realtime.connection_status(),
                RealtimeConnectionStatus::ConnectingWebTransport
            );
            assert!(!realtime.request_reconnect());
            assert_eq!(requests.next().now_or_never(), Some(Some(())));
            assert!(requests.next().now_or_never().is_none());
        }
    });
}

#[test]
fn manual_reconnect_does_not_interrupt_primary_or_connecting_transport() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        for status in [
            RealtimeConnectionStatus::ConnectingWebTransport,
            RealtimeConnectionStatus::ConnectingWebSocketFallback,
            RealtimeConnectionStatus::Connected(
                crate::features::realtime::RealtimeTransportKind::WebTransport,
            ),
        ] {
            let realtime = create_handle();
            let mut requests = realtime.subscribe_reconnect_requests();
            realtime.set_connection_status(status);
            assert!(!realtime.request_reconnect());
            assert_eq!(realtime.connection_status(), status);
            assert!(requests.next().now_or_never().is_none());
        }
    });
}

#[test]
fn manual_reconnect_without_runtime_keeps_disconnected_status() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = create_handle();
        assert!(!realtime.request_reconnect());
        let receiver = realtime.subscribe_reconnect_requests();
        drop(receiver);
        assert!(!realtime.request_reconnect());
        assert_eq!(
            realtime.connection_status(),
            RealtimeConnectionStatus::Disconnected
        );
    });
}

#[test]
fn runtime_consumes_manual_reconnect_and_closes_fallback_session() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = create_handle();
        let mut requests = realtime.subscribe_reconnect_requests();
        let (sender, mut receiver) = mpsc::unbounded();
        let (sender, _, _) =
            crate::features::realtime::websocket::WebSocketOutboundSender::new(sender);
        *realtime.inner.session.try_lock().unwrap() = Some(ConnectedSession {
            generation: 1,
            transport: ConnectedTransport::WebSocket(sender),
        });
        realtime.set_connection_status(RealtimeConnectionStatus::Connected(
            RealtimeTransportKind::WebSocketFallback,
        ));
        let mut quality = crate::features::network::NetworkQualityHandle::new(
            Signal::new(Default::default()),
            Signal::new(false),
        );
        quality.record_ping(1_000, 25.0);
        assert!(realtime.request_reconnect());
        // Закрытие очереди после запроса останавливает runtime до обращения к сети.
        realtime.inner.reconnect_requests.borrow_mut().take();
        assert_eq!(
            crate::features::realtime::connection_runtime::run_connection(
                &realtime,
                quality,
                &mut requests,
            )
            .now_or_never(),
            Some(())
        );
        assert!(matches!(receiver.next().now_or_never(), Some(None)));
        assert!(realtime.inner.session.try_lock().unwrap().is_none());
        assert!(quality.current().samples.is_empty());
        assert_eq!(
            realtime.connection_status(),
            RealtimeConnectionStatus::Disconnected
        );
    });
}

#[test]
fn disconnect_closes_websocket_even_when_sender_clones_survive() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = create_handle();
        let (sender, mut receiver) = mpsc::unbounded();
        let surviving_sender = sender.clone();
        let (sender, _, _) =
            crate::features::realtime::websocket::WebSocketOutboundSender::new(sender);
        let transport = ConnectedTransport::WebSocket(sender);
        let session = ConnectedSession {
            generation: 1,
            transport,
        };
        *realtime.inner.session.try_lock().unwrap() = Some(session);
        realtime.set_connection_status(RealtimeConnectionStatus::Connected(
            crate::features::realtime::RealtimeTransportKind::WebSocketFallback,
        ));

        assert_eq!(realtime.mark_disconnected().now_or_never(), Some(()));
        assert!(matches!(receiver.next().now_or_never(), Some(None)));
        assert!(surviving_sender.is_closed());
        assert!(realtime.inner.session.try_lock().unwrap().is_none());
        assert_eq!(
            realtime.connection_status(),
            RealtimeConnectionStatus::Disconnected
        );
    });
}

#[test]
fn retired_connection_watcher_cannot_close_resumed_session() {
    let dom = VirtualDom::new(|| rsx! {});
    dom.in_scope(ScopeId::ROOT, || {
        let realtime = create_handle();
        let (sender, mut receiver) = mpsc::unbounded();
        let (sender, _, _) =
            crate::features::realtime::websocket::WebSocketOutboundSender::new(sender);
        let session = ConnectedSession {
            generation: 2,
            transport: ConnectedTransport::WebSocket(sender),
        };
        *realtime.inner.session.try_lock().unwrap() = Some(session);
        realtime.set_connection_status(RealtimeConnectionStatus::Connected(
            crate::features::realtime::RealtimeTransportKind::WebSocketFallback,
        ));
        assert_eq!(realtime.clear_generation(1).now_or_never(), Some(()));
        assert!(receiver.next().now_or_never().is_none());
        assert_eq!(
            realtime.connection_status(),
            RealtimeConnectionStatus::Connected(
                crate::features::realtime::RealtimeTransportKind::WebSocketFallback,
            )
        );
        assert_eq!(realtime.clear_generation(2).now_or_never(), Some(()));
        assert!(matches!(receiver.next().now_or_never(), Some(None)));
    });
}
