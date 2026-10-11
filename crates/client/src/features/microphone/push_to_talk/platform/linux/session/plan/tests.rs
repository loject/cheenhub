use super::{Plan, plan};

#[test]
fn capture_without_restorable_binding_never_requests_dialog() {
    assert_eq!(plan(false, false, false, 1), Plan::BindingRequired);
}

#[test]
fn capture_restores_listed_binding_on_version_one() {
    assert_eq!(plan(false, true, true, 1), Plan::Restore);
}

#[test]
fn first_assignment_uses_bind_on_version_one() {
    assert_eq!(plan(true, false, false, 1), Plan::FirstAssignment);
}

#[test]
fn reassignment_uses_configuration_only_when_available() {
    assert_eq!(plan(true, true, true, 2), Plan::Reassign);
    assert_eq!(plan(true, true, true, 1), Plan::SettingsRequired);
}

#[test]
fn capture_restores_saved_binding_when_list_is_empty() {
    for version in [1, 2] {
        let operation = plan(false, false, true, version);
        assert_eq!(operation, Plan::Restore);
        assert!(operation.requires_bind());
    }
}

#[test]
fn version_one_reassignment_never_binds_to_open_settings() {
    for listed in [false, true] {
        let operation = plan(true, listed, true, 1);
        assert_eq!(operation, Plan::SettingsRequired);
        assert!(!operation.requires_bind());
    }
}
