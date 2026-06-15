//! Pure guard decisions for guest-only authentication pages.

/// Guest-only authentication page protected by the saved-session guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuestAuthPage {
    /// Email/password login page.
    Login,
    /// Account registration page.
    Register,
}

impl GuestAuthPage {
    /// Returns the routed path for this guest authentication page.
    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::Login => "/login",
            Self::Register => "/register",
        }
    }
}

/// Result of evaluating access to a guest-only authentication page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuestAuthGuardDecision {
    /// The guest page can be rendered.
    Render,
    /// The user has a saved session and should be sent to the app home.
    RedirectToAppHome { source: GuestAuthPage },
}

/// Decides whether a guest authentication page can render for the current session state.
pub(crate) const fn decide_guest_auth_guard(
    source: GuestAuthPage,
    has_saved_session: bool,
) -> GuestAuthGuardDecision {
    if has_saved_session {
        GuestAuthGuardDecision::RedirectToAppHome { source }
    } else {
        GuestAuthGuardDecision::Render
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_login_without_saved_session() {
        let decision = decide_guest_auth_guard(GuestAuthPage::Login, false);

        assert_eq!(decision, GuestAuthGuardDecision::Render);
    }

    #[test]
    fn renders_register_without_saved_session() {
        let decision = decide_guest_auth_guard(GuestAuthPage::Register, false);

        assert_eq!(decision, GuestAuthGuardDecision::Render);
    }

    #[test]
    fn redirects_login_with_saved_session() {
        let decision = decide_guest_auth_guard(GuestAuthPage::Login, true);

        assert_eq!(
            decision,
            GuestAuthGuardDecision::RedirectToAppHome {
                source: GuestAuthPage::Login
            }
        );
    }

    #[test]
    fn redirects_register_with_saved_session() {
        let decision = decide_guest_auth_guard(GuestAuthPage::Register, true);

        assert_eq!(
            decision,
            GuestAuthGuardDecision::RedirectToAppHome {
                source: GuestAuthPage::Register
            }
        );
    }

    #[test]
    fn reports_guest_page_paths_for_redirect_logs() {
        assert_eq!(GuestAuthPage::Login.path(), "/login");
        assert_eq!(GuestAuthPage::Register.path(), "/register");
    }
}
