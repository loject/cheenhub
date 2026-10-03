//! Данные собственных ссылок-приглашений сервера для меню их создания.

use cheenhub_contracts::rest::ServerOwnInviteLink;

/// Ссылка-приглашение, созданная текущим пользователем на сервере.
#[derive(Clone, PartialEq)]
pub(crate) struct OwnInviteLink {
    /// Код приглашения, используемый для копирования ссылки и удаления.
    pub(crate) code: String,
    /// Время создания приглашения в формате RFC3339.
    pub(crate) created_at: String,
    /// Необязательное время истечения в формате RFC3339.
    pub(crate) expires_at: Option<String>,
    /// Необязательный лимит использований приглашения.
    pub(crate) max_uses: Option<u32>,
    /// Количество успешных использований приглашения.
    pub(crate) uses: u32,
    /// Действует ли приглашение прямо сейчас.
    pub(crate) is_active: bool,
}

/// Состояние собственных ссылок-приглашений на сервере.
#[derive(Clone, PartialEq)]
pub(crate) struct OwnInviteLinks {
    /// Ссылки-приглашения текущего пользователя, от новых к старым.
    pub(crate) links: Vec<OwnInviteLink>,
    /// Максимальное количество действующих ссылок, доступное пользователю.
    ///
    /// Значение `None` означает, что ограничения нет, например у владельца сервера.
    pub(crate) limit: Option<u32>,
}

impl OwnInviteLinks {
    /// Считает количество действующих ссылок, которые занимают лимит.
    pub(crate) fn active_count(&self) -> u32 {
        self.links.iter().filter(|link| link.is_active).count() as u32
    }

    /// Достигнут ли лимит действующих ссылок.
    ///
    /// Без лимита ограничение никогда не считается достигнутым.
    pub(crate) fn is_limit_reached(&self) -> bool {
        self.limit.is_some_and(|limit| self.active_count() >= limit)
    }
}

/// Преобразует REST-контракт ссылки в данные интерфейса.
pub(crate) fn own_invite_from_rest(link: ServerOwnInviteLink) -> OwnInviteLink {
    OwnInviteLink {
        code: link.code,
        created_at: link.created_at,
        expires_at: link.expires_at,
        max_uses: link.max_uses,
        uses: link.uses,
        is_active: link.is_active,
    }
}

#[cfg(test)]
mod tests;
