//! Фоновые требования и освобождение текущего realtime-соединения.

use futures_channel::mpsc;

use super::{ConnectedTransport, RealtimeHandle};
use crate::features::realtime::{RealtimeConnectionStatus, RealtimeTransportKind};

impl RealtimeHandle {
    /// Запрашивает немедленную новую попытку WebTransport вместо отключения или fallback.
    ///
    /// Возвращает `false`, если уже идёт подключение, активен WebTransport или runtime
    /// недоступен. Принятый запрос сразу меняет статус, блокируя повторные нажатия;
    /// закрытием текущего транспорта и новой попыткой управляет провайдер.
    pub(crate) fn request_reconnect(&self) -> bool {
        let status = self.connection_status();
        if !matches!(
            status,
            RealtimeConnectionStatus::Disconnected
                | RealtimeConnectionStatus::Connected(RealtimeTransportKind::WebSocketFallback)
        ) {
            return false;
        }
        let requests = self.inner.reconnect_requests.borrow();
        if !requests
            .as_ref()
            .is_some_and(|sender| sender.unbounded_send(()).is_ok())
        {
            dioxus::prelude::warn!(
                ?status,
                "manual realtime reconnect unavailable without runtime"
            );
            return false;
        }
        dioxus::prelude::info!(?status, "manual realtime reconnect requested");
        self.mark_connecting(RealtimeTransportKind::WebTransport);
        true
    }

    /// Регистрирует единственного владельца ручных запросов переподключения.
    ///
    /// Провайдер хранит receiver на протяжении своего жизненного цикла,
    /// включая приостановку соединения в фоне.
    pub(in crate::features::realtime) fn subscribe_reconnect_requests(
        &self,
    ) -> mpsc::UnboundedReceiver<()> {
        let (sender, receiver) = mpsc::unbounded();
        self.inner.reconnect_requests.replace(Some(sender));
        receiver
    }

    /// Указывает, требуется ли соединение активной сессии при скрытом приложении.
    pub(crate) fn set_background_activity_required(&self, required: bool) {
        self.inner.background_activity.set_required(required);
    }

    /// Подписывается на изменение необходимости фонового соединения.
    pub(in crate::features::realtime) fn subscribe_background_activity(
        &self,
    ) -> mpsc::UnboundedReceiver<bool> {
        self.inner.background_activity.subscribe()
    }

    /// Закрывает текущий транспорт и сообщает об отключении.
    pub(crate) async fn mark_disconnected(&self) {
        if let Some(session) = self.inner.session.lock().await.take() {
            session.transport.close();
        }
        self.inner.streams.lock().await.clear();
        self.inner.pending.borrow_mut().clear();
        self.set_connection_status(RealtimeConnectionStatus::Disconnected);
    }

    /// Закрывает соединение только при совпадении поколения, сохраняя новую сессию.
    pub(in crate::features::realtime) async fn clear_generation(&self, generation: u64) {
        let mut session = self.inner.session.lock().await;
        let should_clear = session
            .as_ref()
            .is_some_and(|connected| connected.generation == generation);
        if should_clear {
            if let Some(connected) = session.take() {
                connected.transport.close();
            }
            drop(session);
            self.inner.streams.lock().await.clear();
            self.inner.pending.borrow_mut().clear();
            self.set_connection_status(RealtimeConnectionStatus::Disconnected);
        }
    }
}

impl ConnectedTransport {
    fn close(&self) {
        match self {
            Self::WebTransport(session) => session.close(0, "Client suspended or disconnected"),
            Self::WebSocket(sender) => sender.close(),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
