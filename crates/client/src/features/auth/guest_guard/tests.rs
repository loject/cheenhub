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
