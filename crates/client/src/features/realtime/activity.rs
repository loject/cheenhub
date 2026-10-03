//! Разрешение realtime-работы по видимости приложения и требованиям активной сессии.

use std::cell::{Cell, RefCell};
use std::future::Future;

use dioxus::prelude::debug;
use futures_channel::mpsc;
use futures_util::future::{Either, select};
use futures_util::{
    FutureExt, StreamExt,
    stream::{self, LocalBoxStream},
};

/// Требования владельца активной сессии к работе в фоне.
#[derive(Default)]
pub(super) struct BackgroundActivity {
    required: Cell<bool>,
    listeners: RefCell<Vec<mpsc::UnboundedSender<bool>>>,
}

impl BackgroundActivity {
    /// Обновляет требование и уведомляет подписчиков только при изменении.
    pub(super) fn set_required(&self, required: bool) {
        if self.required.replace(required) == required {
            return;
        }
        debug!(required, "updated realtime background activity requirement");
        self.listeners
            .borrow_mut()
            .retain(|listener| listener.unbounded_send(required).is_ok());
    }

    /// Подписывается на требование с немедленной отправкой текущего значения.
    pub(super) fn subscribe(&self) -> mpsc::UnboundedReceiver<bool> {
        let (sender, receiver) = mpsc::unbounded();
        let _ = sender.unbounded_send(self.required.get());
        self.listeners
            .borrow_mut()
            .retain(|listener| !listener.is_closed());
        self.listeners.borrow_mut().push(sender);
        receiver
    }
}

#[derive(Default)]
struct ActivityPolicy {
    visible: bool,
    background_required: bool,
}

impl ActivityPolicy {
    fn allows_connection(&self) -> bool {
        self.visible || self.background_required
    }
}

enum ActivityEvent {
    Visibility(bool),
    BackgroundRequired(bool),
}

/// Ожидание видимости или активной фоновой сессии без периодических таймеров.
pub(super) struct ActivityGate {
    policy: ActivityPolicy,
    events: LocalBoxStream<'static, ActivityEvent>,
}

impl ActivityGate {
    /// Объединяет платформенную видимость и требования активной сессии.
    pub(super) fn new(
        visibility: LocalBoxStream<'static, bool>,
        background: mpsc::UnboundedReceiver<bool>,
    ) -> Self {
        Self {
            policy: ActivityPolicy::default(),
            events: stream::select(
                visibility.map(ActivityEvent::Visibility),
                background.map(ActivityEvent::BackgroundRequired),
            )
            .boxed_local(),
        }
    }

    /// Ожидает разрешения на открытие или восстановление соединения.
    pub(super) async fn wait_until_allowed(&mut self) {
        self.wait_until(true).await;
    }

    /// Отменяет текущую операцию при сворачивании без активной фоновой сессии.
    pub(super) async fn run_until_suspended<F: Future>(
        &mut self,
        operation: F,
    ) -> Option<F::Output> {
        match select(
            self.wait_until(false).boxed_local(),
            operation.boxed_local(),
        )
        .await
        {
            Either::Left(_) => None,
            Either::Right((result, _)) => Some(result),
        }
    }

    async fn wait_until(&mut self, allowed: bool) {
        loop {
            // Обрабатываем очередь до проверки, чтобы быстрый onStop/onStart не оставил устаревший запрет.
            while let Some(Some(event)) = self.events.next().now_or_never() {
                self.apply(event);
            }
            if self.policy.allows_connection() == allowed {
                return;
            }
            match self.events.next().await {
                Some(event) => self.apply(event),
                None => std::future::pending::<()>().await,
            }
        }
    }

    fn apply(&mut self, event: ActivityEvent) {
        match event {
            ActivityEvent::Visibility(visible) => self.policy.visible = visible,
            ActivityEvent::BackgroundRequired(required) => {
                self.policy.background_required = required
            }
        }
    }
}

#[cfg(test)]
mod tests;
