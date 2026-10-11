//! Явное назначение принадлежит UI; отмена не удаляет системное сочетание.

use super::{Service, binding::PushToTalkKey};
use tokio::sync::oneshot;

/// Владеет ожиданием UI и единственным ответом команды Assign.
pub(super) struct Assignment {
    reply: Option<oneshot::Sender<Result<PushToTalkKey, String>>>,
}

impl Assignment {
    /// При Restore отсутствует ожидание пользователя.
    pub(super) fn new(reply: Option<oneshot::Sender<Result<PushToTalkKey, String>>>) -> Self {
        Self { reply }
    }
    /// Отличает явное назначение от восстановления capture.
    pub(super) fn active(&self) -> bool {
        self.reply.is_some()
    }
    /// Назначение держит сессию до ответа UI, даже если capture одновременно освобождён.
    pub(super) fn keeps_session(&self, users: usize) -> bool {
        self.active() || users > 0
    }
    /// Завершает UI после подтверждённого изменения или явного «Готово».
    pub(super) fn complete(&mut self, service: &Service) {
        service.configuring(false);
        if let Some(reply) = self.reply.take() {
            let key = service.updates.borrow().binding.unwrap_or_default();
            let _ = reply.send(Ok(key));
        }
    }
    /// Отменяет ожидание и закрывает gate; сохранённая привязка не меняется.
    pub(super) fn cancel(&mut self, service: &Service) {
        self.reply.take();
        service.configuring(false);
        service.state.fail();
    }
    /// Возвращает ошибку операции без удаления прежней настройки.
    pub(super) fn reject(&mut self, service: &Service, message: &str) {
        service.configuring(false);
        if let Some(reply) = self.reply.take() {
            let _ = reply.send(Err(message.into()));
        }
    }
    /// Drop Dioxus task уведомляет event loop немедленно, без тайм-аута.
    pub(super) async fn cancelled(&mut self) {
        if let Some(reply) = &mut self.reply {
            reply.closed().await;
        } else {
            std::future::pending::<()>().await;
        }
    }
}

#[cfg(test)]
mod tests;
