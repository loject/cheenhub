//! Current authenticated user context.

use cheenhub_contracts::rest::AuthUser;
use dioxus::prelude::*;

/// Context shared by authenticated app components that need the current user.
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
}
