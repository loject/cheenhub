//! Изолированное in-memory состояние приглашений в серверные голосовые комнаты.

use cheenhub_contracts::realtime::{
    VoiceRoomInviteResponse, VoiceRoomInviteSnapshot, VoiceRoomInviteStatus,
};
use chrono::{DateTime, Duration, Utc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Время хранения завершённого приглашения для восстановления после переподключения.
const RESOLVED_RETENTION: Duration = Duration::minutes(2);
/// Минимальная пауза между приглашениями одной пары пользователей независимо от комнаты.
const REINVITE_COOLDOWN: Duration = Duration::seconds(30);
/// Максимум одновременно ожидающих приглашений одному пользователю.
const MAX_PENDING_FOR_RECIPIENT: usize = 5;

/// Атомарное хранилище приглашений, независимое от личных звонков.
#[derive(Default)]
pub(crate) struct InMemoryVoiceRoomInviteStore {
    invites: Mutex<Vec<VoiceRoomInvite>>,
}

/// Приглашение, созданное сервером после проверки полномочий отправителя.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceRoomInvite {
    pub(crate) id: Uuid,
    pub(crate) server_id: Uuid,
    pub(crate) server_name: String,
    pub(crate) room_id: Uuid,
    pub(crate) room_name: String,
    pub(crate) caller_user_id: Uuid,
    pub(crate) caller_nickname: String,
    pub(crate) caller_avatar_url: Option<String>,
    pub(crate) recipient_user_id: Uuid,
    /// Имя получателя, зафиксированное при отправке приглашения.
    pub(crate) recipient_nickname: String,
    /// Аватар получателя, зафиксированный при отправке приглашения.
    pub(crate) recipient_avatar_url: Option<String>,
    pub(crate) status: VoiceRoomInviteStatus,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) resolved_at: Option<DateTime<Utc>>,
}

impl VoiceRoomInvite {
    /// Возвращает сериализуемый снимок для обоих участников.
    pub(crate) fn snapshot(&self) -> VoiceRoomInviteSnapshot {
        VoiceRoomInviteSnapshot {
            invite_id: self.id.to_string(),
            server_id: self.server_id.to_string(),
            server_name: self.server_name.clone(),
            room_id: self.room_id.to_string(),
            room_name: self.room_name.clone(),
            caller_user_id: self.caller_user_id.to_string(),
            caller_nickname: self.caller_nickname.clone(),
            caller_avatar_url: self.caller_avatar_url.clone(),
            recipient_user_id: self.recipient_user_id.to_string(),
            recipient_nickname: Some(self.recipient_nickname.clone()),
            recipient_avatar_url: self.recipient_avatar_url.clone(),
            status: self.status,
            created_at: self.created_at.to_rfc3339(),
            expires_at: self.expires_at.to_rfc3339(),
            resolved_at: self.resolved_at.map(|resolved| resolved.to_rfc3339()),
        }
    }
}

/// Ожидаемые ошибки операций с приглашениями.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VoiceRoomInviteStoreError {
    /// Для той же пары участников и комнаты приглашение уже ожидает ответа.
    AlreadyPending,
    /// Повторное приглашение этой пары отправлено раньше разрешённого интервала.
    TooSoon,
    /// Получатель уже имеет максимально допустимое число ожидающих приглашений.
    RecipientAtCapacity,
    /// Приглашение не найдено.
    NotFound,
    /// Запрос выполняет посторонний пользователь.
    Unauthorized,
    /// Приглашение уже завершено.
    InvalidState,
    /// Приглашение истекло непосредственно во время операции.
    Expired(Box<VoiceRoomInvite>),
}

impl InMemoryVoiceRoomInviteStore {
    /// Создаёт приглашение и атомарно предотвращает повторную отправку.
    pub(crate) async fn start(
        &self,
        invite: VoiceRoomInvite,
        now: DateTime<Utc>,
    ) -> Result<VoiceRoomInvite, VoiceRoomInviteStoreError> {
        let mut invites = self.invites.lock().await;
        retain_recent(&mut invites, now);
        if invites.iter().any(|existing| {
            existing.status == VoiceRoomInviteStatus::Pending
                && existing.expires_at > now
                && existing.server_id == invite.server_id
                && existing.room_id == invite.room_id
                && existing.caller_user_id == invite.caller_user_id
                && existing.recipient_user_id == invite.recipient_user_id
        }) {
            return Err(VoiceRoomInviteStoreError::AlreadyPending);
        }
        if invites.iter().any(|existing| {
            existing.caller_user_id == invite.caller_user_id
                && existing.recipient_user_id == invite.recipient_user_id
                && now < existing.created_at + REINVITE_COOLDOWN
        }) {
            return Err(VoiceRoomInviteStoreError::TooSoon);
        }
        if invites
            .iter()
            .filter(|existing| {
                existing.recipient_user_id == invite.recipient_user_id
                    && existing.status == VoiceRoomInviteStatus::Pending
                    && existing.expires_at > now
            })
            .count()
            >= MAX_PENDING_FOR_RECIPIENT
        {
            return Err(VoiceRoomInviteStoreError::RecipientAtCapacity);
        }
        invites.push(invite.clone());
        Ok(invite)
    }

    /// Атомарно принимает или отклоняет приглашение от имени получателя.
    pub(crate) async fn respond(
        &self,
        invite_id: &Uuid,
        recipient_user_id: &Uuid,
        response: VoiceRoomInviteResponse,
        now: DateTime<Utc>,
    ) -> Result<VoiceRoomInvite, VoiceRoomInviteStoreError> {
        let mut invites = self.invites.lock().await;
        let invite = invites
            .iter_mut()
            .find(|invite| invite.id == *invite_id)
            .ok_or(VoiceRoomInviteStoreError::NotFound)?;
        if invite.recipient_user_id != *recipient_user_id {
            return Err(VoiceRoomInviteStoreError::Unauthorized);
        }
        if invite.status != VoiceRoomInviteStatus::Pending {
            return Err(VoiceRoomInviteStoreError::InvalidState);
        }
        if invite.expires_at <= now {
            mark_resolved(invite, VoiceRoomInviteStatus::Expired, now);
            return Err(VoiceRoomInviteStoreError::Expired(Box::new(invite.clone())));
        }
        let status = match response {
            VoiceRoomInviteResponse::Accept => VoiceRoomInviteStatus::Accepted,
            VoiceRoomInviteResponse::Decline => VoiceRoomInviteStatus::Declined,
        };
        mark_resolved(invite, status, now);
        Ok(invite.clone())
    }

    /// Отменяет ожидающее приглашение от имени отправителя.
    pub(crate) async fn cancel(
        &self,
        invite_id: &Uuid,
        caller_user_id: &Uuid,
        now: DateTime<Utc>,
    ) -> Result<VoiceRoomInvite, VoiceRoomInviteStoreError> {
        let mut invites = self.invites.lock().await;
        let invite = invites
            .iter_mut()
            .find(|invite| invite.id == *invite_id)
            .ok_or(VoiceRoomInviteStoreError::NotFound)?;
        if invite.caller_user_id != *caller_user_id {
            return Err(VoiceRoomInviteStoreError::Unauthorized);
        }
        if invite.status != VoiceRoomInviteStatus::Pending {
            return Err(VoiceRoomInviteStoreError::InvalidState);
        }
        if invite.expires_at <= now {
            mark_resolved(invite, VoiceRoomInviteStatus::Expired, now);
            return Err(VoiceRoomInviteStoreError::Expired(Box::new(invite.clone())));
        }
        mark_resolved(invite, VoiceRoomInviteStatus::Cancelled, now);
        Ok(invite.clone())
    }

    /// Отменяет все ожидающие приглашения между заблокировавшими друг друга пользователями.
    pub(crate) async fn cancel_between_users(
        &self,
        left: &Uuid,
        right: &Uuid,
        now: DateTime<Utc>,
    ) -> Vec<VoiceRoomInvite> {
        let mut invites = self.invites.lock().await;
        let mut cancelled = Vec::new();
        for invite in invites.iter_mut() {
            let between = (invite.caller_user_id == *left && invite.recipient_user_id == *right)
                || (invite.caller_user_id == *right && invite.recipient_user_id == *left);
            if between && invite.status == VoiceRoomInviteStatus::Pending {
                mark_resolved(invite, VoiceRoomInviteStatus::Cancelled, now);
                cancelled.push(invite.clone());
            }
        }
        cancelled
    }

    /// Возвращает приглашение для проверки целевой комнаты и членства получателя.
    pub(crate) async fn find(&self, invite_id: &Uuid) -> Option<VoiceRoomInvite> {
        self.invites
            .lock()
            .await
            .iter()
            .find(|invite| invite.id == *invite_id)
            .cloned()
    }

    /// Возвращает приглашения только указанной стороне обмена.
    pub(crate) async fn list_for_user(&self, user_id: &Uuid) -> Vec<VoiceRoomInvite> {
        let mut invites = self.invites.lock().await;
        retain_recent(&mut invites, Utc::now());
        invites
            .iter()
            .filter(|invite| {
                invite.caller_user_id == *user_id || invite.recipient_user_id == *user_id
            })
            .cloned()
            .collect()
    }

    /// Переводит одно истёкшее приглашение в завершённое состояние.
    pub(crate) async fn expire(
        &self,
        invite_id: &Uuid,
        now: DateTime<Utc>,
    ) -> Option<VoiceRoomInvite> {
        let mut invites = self.invites.lock().await;
        let invite = invites.iter_mut().find(|invite| {
            invite.id == *invite_id
                && invite.status == VoiceRoomInviteStatus::Pending
                && invite.expires_at <= now
        })?;
        mark_resolved(invite, VoiceRoomInviteStatus::Expired, now);
        Some(invite.clone())
    }

    /// Завершает все приглашения, у которых истёк срок ожидания.
    pub(crate) async fn expire_pending(&self, now: DateTime<Utc>) -> Vec<VoiceRoomInvite> {
        let mut invites = self.invites.lock().await;
        let mut expired = Vec::new();
        for invite in invites.iter_mut() {
            if invite.status == VoiceRoomInviteStatus::Pending && invite.expires_at <= now {
                mark_resolved(invite, VoiceRoomInviteStatus::Expired, now);
                expired.push(invite.clone());
            }
        }
        expired
    }
}

fn retain_recent(invites: &mut Vec<VoiceRoomInvite>, now: DateTime<Utc>) {
    invites.retain(|existing| {
        existing.status == VoiceRoomInviteStatus::Pending
            || existing
                .resolved_at
                .is_some_and(|resolved_at| now - resolved_at < RESOLVED_RETENTION)
    });
}

fn mark_resolved(invite: &mut VoiceRoomInvite, status: VoiceRoomInviteStatus, now: DateTime<Utc>) {
    invite.status = status;
    invite.resolved_at = Some(now);
}

#[cfg(test)]
mod tests;
