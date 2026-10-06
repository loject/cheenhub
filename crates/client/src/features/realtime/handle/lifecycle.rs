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
mod tests;
