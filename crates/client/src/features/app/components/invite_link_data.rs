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
    pub(crate) limit: u32,
}

impl OwnInviteLinks {
    /// Считает количество действующих ссылок, которые занимают лимит.
    pub(crate) fn active_count(&self) -> u32 {
        self.links.iter().filter(|link| link.is_active).count() as u32
    }

    /// Достигнут ли лимит действующих ссылок.
    pub(crate) fn is_limit_reached(&self) -> bool {
        self.active_count() >= self.limit
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
mod tests {
    use super::{OwnInviteLink, OwnInviteLinks};

    fn link(is_active: bool) -> OwnInviteLink {
        OwnInviteLink {
            code: "code".to_owned(),
            created_at: "2026-01-01T00:00:00+00:00".to_owned(),
            expires_at: None,
            max_uses: None,
            uses: 0,
            is_active,
        }
    }

    fn links(limit: u32, active: usize, inactive: usize) -> OwnInviteLinks {
        let mut all = (0..active).map(|_| link(true)).collect::<Vec<_>>();
        all.extend((0..inactive).map(|_| link(false)));

        OwnInviteLinks { links: all, limit }
    }

    #[test]
    fn counts_only_active_links() {
        assert_eq!(links(3, 2, 4).active_count(), 2);
    }

    #[test]
    fn reports_limit_reached_only_when_active_count_matches_limit() {
        assert!(!links(3, 2, 0).is_limit_reached());
        assert!(links(3, 3, 0).is_limit_reached());
        assert!(links(3, 4, 0).is_limit_reached());
    }

    #[test]
    fn keeps_creating_allowed_when_only_inactive_links_exist() {
        assert!(!links(1, 0, 5).is_limit_reached());
    }
}
