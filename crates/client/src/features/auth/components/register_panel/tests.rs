use super::registration_input_error;

#[test]
fn accepts_registration_fields_matching_server_rules() {
    assert_eq!(
        registration_input_error("cheen_hero", "hero@example.com", "password1", "password1"),
        None
    );
}

#[test]
fn rejects_unusable_registration_fields() {
    assert!(registration_input_error("ab", "hero@example.com", "password1", "password1").is_some());
    assert!(registration_input_error("cheen_hero", "hero", "password1", "password1").is_some());
    assert!(registration_input_error("cheen_hero", "hero@example.com", "short", "short").is_some());
    assert!(
        registration_input_error("cheen_hero", "hero@example.com", "password1", "password2")
            .is_some()
    );
}
