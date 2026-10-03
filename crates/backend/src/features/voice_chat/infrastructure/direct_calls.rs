//! In-memory состояние приглашений и активных личных звонков.

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// In-memory хранилище незавершённых личных звонков.
#[derive(Default)]
pub(crate) struct InMemoryDirectCallStore {
    calls: Mutex<Vec<DirectCall>>,
}

/// Незавершённый личный звонок.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectCall {
    /// Стабильный идентификатор звонка.
    pub(crate) id: Uuid,
    /// Идентификатор личного диалога.
    pub(crate) conversation_id: Uuid,
    /// Идентификатор инициатора.
    pub(crate) caller_user_id: Uuid,
    /// Снимок ника инициатора.
    pub(crate) caller_nickname: String,
    /// Снимок URL аватара инициатора.
    pub(crate) caller_avatar_url: Option<String>,
    /// Идентификатор вызываемого пользователя.
    pub(crate) callee_user_id: Uuid,
    /// Снимок ника вызываемого пользователя.
    pub(crate) callee_nickname: String,
    /// Снимок URL аватара вызываемого пользователя.
    pub(crate) callee_avatar_url: Option<String>,
    /// Момент создания приглашения.
    pub(crate) started_at: DateTime<Utc>,
    /// Момент истечения приглашения.
    pub(crate) expires_at: DateTime<Utc>,
    /// Момент принятия звонка.
    pub(crate) answered_at: Option<DateTime<Utc>>,
    /// Было ли приглашение показано вызываемому пользователю.
    pub(crate) callee_notified: bool,
}

impl DirectCall {
    /// Проверяет участие пользователя в звонке.
    pub(crate) fn includes_user(&self, user_id: &Uuid) -> bool {
        self.caller_user_id == *user_id || self.callee_user_id == *user_id
    }

    /// Проверяет, ожидает ли звонок ответа.
    pub(crate) fn is_ringing(&self) -> bool {
        self.answered_at.is_none()
    }
}

/// Результат перехода состояния личного звонка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectCallTransition {
    /// Звонок принят и остаётся активным.
    Accepted(DirectCall),
    /// Звонок завершён и удалён из активного состояния.
    Ended(DirectCall),
}

/// Ожидаемая ошибка операции с личным звонком.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectCallStoreError {
    /// Инициатор уже участвует в другом незавершённом звонке.
    CallerBusy,
    /// Звонок не найден.
    NotFound,
    /// Пользователь не может выполнить эту операцию.
    Unauthorized,
    /// Операция не соответствует текущему состоянию звонка.
    InvalidState,
    /// Приглашение истекло во время операции.
    Expired(DirectCall),
}

impl InMemoryDirectCallStore {
    /// Создаёт приглашение, скрывая его от уже занятого вызываемого пользователя.
    pub(crate) async fn start(
        &self,
        mut call: DirectCall,
    ) -> Result<DirectCall, DirectCallStoreError> {
        let mut calls = self.calls.lock().await;
        if calls
            .iter()
            .any(|existing| existing.includes_user(&call.caller_user_id))
        {
            return Err(DirectCallStoreError::CallerBusy);
        }
        call.callee_notified = !calls
            .iter()
            .any(|existing| existing.includes_user(&call.callee_user_id));
        calls.push(call.clone());
        Ok(call)
    }

    /// Принимает или отклоняет ожидающий звонок от имени вызываемого пользователя.
    pub(crate) async fn respond(
        &self,
        call_id: &Uuid,
        callee_user_id: &Uuid,
        accept: bool,
        now: DateTime<Utc>,
    ) -> Result<DirectCallTransition, DirectCallStoreError> {
        let mut calls = self.calls.lock().await;
        let Some(index) = calls.iter().position(|call| call.id == *call_id) else {
            return Err(DirectCallStoreError::NotFound);
        };
        if calls[index].callee_user_id != *callee_user_id {
            return Err(DirectCallStoreError::Unauthorized);
        }
        if !calls[index].callee_notified {
            return Err(DirectCallStoreError::NotFound);
        }
        if !calls[index].is_ringing() {
            return Err(DirectCallStoreError::InvalidState);
        }
        if calls[index].expires_at <= now {
            return Err(DirectCallStoreError::Expired(calls.remove(index)));
        }
        if accept {
            calls[index].answered_at = Some(now);
            Ok(DirectCallTransition::Accepted(calls[index].clone()))
        } else {
            Ok(DirectCallTransition::Ended(calls.remove(index)))
        }
    }

    /// Отменяет ожидающий звонок от имени инициатора.
    pub(crate) async fn cancel(
        &self,
        call_id: &Uuid,
        caller_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> Result<DirectCall, DirectCallStoreError> {
        let mut calls = self.calls.lock().await;
        let Some(index) = calls.iter().position(|call| call.id == *call_id) else {
            return Err(DirectCallStoreError::NotFound);
        };
        if calls[index].caller_user_id != *caller_user_id {
            return Err(DirectCallStoreError::Unauthorized);
        }
        if !calls[index].is_ringing() {
            return Err(DirectCallStoreError::InvalidState);
        }
        if calls[index].expires_at <= now {
            return Err(DirectCallStoreError::Expired(calls.remove(index)));
        }
        Ok(calls.remove(index))
    }

    /// Завершает принятый звонок от имени любого участника.
    pub(crate) async fn end(
        &self,
        call_id: &Uuid,
        user_id: &Uuid,
    ) -> Result<DirectCall, DirectCallStoreError> {
        let mut calls = self.calls.lock().await;
        let Some(index) = calls.iter().position(|call| call.id == *call_id) else {
            return Err(DirectCallStoreError::NotFound);
        };
        if !calls[index].includes_user(user_id) {
            return Err(DirectCallStoreError::Unauthorized);
        }
        if calls[index].is_ringing() {
            return Err(DirectCallStoreError::InvalidState);
        }
        Ok(calls.remove(index))
    }

    /// Завершает принятый звонок пользователя в указанном личном диалоге.
    pub(crate) async fn end_active_for_conversation(
        &self,
        conversation_id: &Uuid,
        user_id: &Uuid,
    ) -> Option<DirectCall> {
        let mut calls = self.calls.lock().await;
        let index = calls.iter().position(|call| {
            call.conversation_id == *conversation_id
                && call.includes_user(user_id)
                && !call.is_ringing()
        })?;
        Some(calls.remove(index))
    }

    /// Возвращает незавершённые звонки пользователя.
    pub(crate) async fn list_for_user(&self, user_id: &Uuid) -> Vec<DirectCall> {
        self.calls
            .lock()
            .await
            .iter()
            .filter(|call| {
                call.caller_user_id == *user_id
                    || (call.callee_user_id == *user_id && call.callee_notified)
            })
            .cloned()
            .collect()
    }

    /// Завершает одно истёкшее приглашение, если оно ещё ожидает ответа.
    pub(crate) async fn expire(&self, call_id: &Uuid, now: DateTime<Utc>) -> Option<DirectCall> {
        let mut calls = self.calls.lock().await;
        let index = calls
            .iter()
            .position(|call| call.id == *call_id && call.is_ringing() && call.expires_at <= now)?;
        Some(calls.remove(index))
    }

    /// Удаляет все истёкшие приглашения и возвращает их для адресной рассылки.
    pub(crate) async fn expire_pending(&self, now: DateTime<Utc>) -> Vec<DirectCall> {
        let mut calls = self.calls.lock().await;
        let mut expired = Vec::new();
        calls.retain(|call| {
            let is_expired = call.is_ringing() && call.expires_at <= now;
            if is_expired {
                expired.push(call.clone());
            }
            !is_expired
        });
        expired
    }
}

#[cfg(test)]
mod tests;
