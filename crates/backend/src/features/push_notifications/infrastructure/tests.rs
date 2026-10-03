//! Проверки хранения push-установок и очереди доставки.

use super::installation_owner_changed;
use uuid::Uuid;

#[test]
fn user_or_session_rebind_requires_old_delivery_cleanup() {
    let first_user = Uuid::new_v4();
    let second_user = Uuid::new_v4();
    let first_session = Uuid::new_v4();
    let second_session = Uuid::new_v4();

    assert!(installation_owner_changed(
        first_user,
        first_session,
        second_user,
        first_session
    ));
    assert!(installation_owner_changed(
        first_user,
        first_session,
        first_user,
        second_session
    ));
    assert!(!installation_owner_changed(
        first_user,
        first_session,
        first_user,
        first_session
    ));
}
