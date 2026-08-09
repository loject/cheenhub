//! Публикация состояния резервного realtime-соединения.

use futures_channel::mpsc;

use crate::features::realtime::status::RealtimeFallbackInfo;

use super::RealtimeHandle;

impl RealtimeHandle {
    /// Возвращает диагностику активного резервного соединения.
    pub(crate) fn fallback_info(&self) -> Option<RealtimeFallbackInfo> {
        self.inner.fallback_info.get()
    }

    /// Подписывается на подтвержденные переходы к WebSocket fallback и восстановление WebTransport.
    pub(crate) fn subscribe_fallback_info(
        &self,
    ) -> mpsc::UnboundedReceiver<Option<RealtimeFallbackInfo>> {
        let (sender, receiver) = mpsc::unbounded();
        let _ = sender.unbounded_send(self.fallback_info());
        self.inner.fallback_listeners.borrow_mut().push(sender);

        receiver
    }

    pub(super) fn publish_fallback_info(&self, fallback: Option<RealtimeFallbackInfo>) {
        self.inner.fallback_info.set(fallback);
        self.inner
            .fallback_listeners
            .borrow_mut()
            .retain(|listener| listener.unbounded_send(fallback).is_ok());
    }
}
