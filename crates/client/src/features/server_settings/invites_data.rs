//! Данные приглашений сервера для интерфейса настроек.

use cheenhub_contracts::realtime::ServerInviteLink;

/// Статус доступности приглашения сервера.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum InviteStatus {
    /// Приглашение можно использовать для входа на сервер.
    Active,
    /// Приглашение отозвано и больше не действует.
    Revoked,
}

/// Ссылка-приглашение сервера, отображаемая в настройках.
#[derive(Clone, PartialEq)]
pub(super) struct InviteLink {
    /// Стабильный локальный идентификатор строки приглашения.
    pub(super) id: String,
    /// Код приглашения, показываемый администраторам.
    pub(super) code: String,
    /// Отображаемое имя создателя приглашения.
    pub(super) author: String,
    /// Человекочитаемое время создания.
    pub(super) created_at: String,
    /// Человекочитаемое время истечения.
    pub(super) expires_at: String,
    /// Необязательный лимит использований.
    pub(super) max_uses: Option<u32>,
    /// Текущий статус приглашения.
    pub(super) status: InviteStatus,
    /// Участники, присоединившиеся по этому приглашению.
    pub(super) joined_members: Vec<InviteJoin>,
}

/// Запись участника, вошедшего по приглашению.
#[derive(Clone, PartialEq)]
pub(super) struct InviteJoin {
    /// Стабильный идентификатор участника.
    pub(super) id: String,
    /// Отображаемое имя участника.
    pub(super) name: String,
    /// Человекочитаемое время вступления.
    pub(super) joined_at: String,
    /// Можно ли сейчас исключить этого участника.
    pub(super) is_active_member: bool,
}

/// Преобразует realtime-полезную нагрузку приглашения в данные интерфейса.
pub(super) fn invite_from_realtime(invite: ServerInviteLink) -> InviteLink {
    InviteLink {
        id: invite.code.clone(),
        code: invite.code,
        author: invite.author_nickname,
        created_at: invite.created_at,
        expires_at: invite.expires_at.unwrap_or_else(|| "без срока".to_owned()),
        max_uses: invite.max_uses,
        status: if invite.revoked_at.is_some() {
            InviteStatus::Revoked
        } else {
            InviteStatus::Active
        },
        joined_members: invite
            .joined_members
            .into_iter()
            .map(|member| InviteJoin {
                id: member.user_id,
                name: member.nickname,
                joined_at: member.joined_at,
                is_active_member: member.is_active_member,
            })
            .collect(),
    }
}
