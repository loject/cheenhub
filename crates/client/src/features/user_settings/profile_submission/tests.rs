use super::*;

#[test]
fn easter_egg_selects_local_activation_instead_of_profile_request() {
    let submission = profile_submission(
        "developer_mode",
        "developer_mode@cheenhub.ru",
        "user@example.com",
    );

    assert!(matches!(
        submission,
        ProfileSubmission::ActivateDeveloperMode
    ));
}

#[test]
fn changed_email_without_exact_pair_never_selects_profile_request() {
    for (nickname, email) in [
        ("Developer_Mode", "developer_mode@cheenhub.ru"),
        ("developer_mode", "developer_mode@CheenHub.ru"),
        ("normal_name", "new@example.com"),
    ] {
        let submission = profile_submission(nickname, email, "user@example.com");

        assert!(
            matches!(submission, ProfileSubmission::RejectEmailChange),
            "{nickname}, {email}"
        );
    }
}

#[test]
fn ordinary_nickname_change_with_current_email_keeps_profile_request() {
    let submission = profile_submission("  new_nickname  ", "user@example.com", "user@example.com");

    let ProfileSubmission::UpdateNickname(request) = submission else {
        panic!("expected nickname update")
    };
    assert_eq!(request.nickname, "new_nickname");
}
