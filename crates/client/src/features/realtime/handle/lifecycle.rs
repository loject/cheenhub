//! Фоновые требования и освобождение текущего realtime-соединения.

use futures_channel::mpsc;

use super::{ConnectedTransport, RealtimeHandle};
use crate::features::realtime::RealtimeConnectionStatus;

impl RealtimeHandle {
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
