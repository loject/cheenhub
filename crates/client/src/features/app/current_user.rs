//! Контекст текущего аутентифицированного пользователя.

use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;

/// Контекст, общий для компонентов аутентифицированного приложения, которым нужен текущий пользователь.
#[derive(Clone, Copy)]
pub(crate) struct CurrentUserContext {
    user: Signal<Option<AuthUser>>,
}

impl CurrentUserContext {
    /// Builds a current-user context from the app-level user signal.
    pub(crate) fn new(user: Signal<Option<AuthUser>>) -> Self {
        Self { user }
    }

    /// Returns the current authenticated user.
    pub(crate) fn require_user(&self) -> AuthUser {
        (self.user)().expect("current user context is available only after profile load")
    }

    /// Replaces the current authenticated user.
    pub(crate) fn set_user(&self, user: AuthUser) {
        let mut current = self.user;
        current.set(Some(user));
    }

    /// Очищает текущего пользователя после завершения локальной сессии.
    pub(crate) fn clear_user(&self) {
        let mut current = self.user;
        current.set(None);
    }
}
