//! Проверки сценариев глобальных настроек хоста.

use super::{non_empty, update_gmail_client_id, update_secret};

#[test]
fn blank_secret_keeps_existing_value_and_explicit_clear_removes_it() {
    let mut value = Some("existing".to_owned());
    update_secret(&mut value, Some("   ".to_owned()), false);
    assert_eq!(value.as_deref(), Some("existing"));

    update_secret(&mut value, Some("replacement".to_owned()), false);
    assert_eq!(value.as_deref(), Some("replacement"));

    update_secret(&mut value, None, true);
    assert_eq!(value, None);
}

#[test]
fn blank_non_secret_is_an_explicit_clear() {
    assert_eq!(non_empty("   ".to_owned()), None);
    assert_eq!(non_empty(" host ".to_owned()).as_deref(), Some("host"));
}

#[test]
fn unchanged_environment_client_id_is_not_copied_into_database() {
    let environment = "environment-id".to_owned();
    let mut database = None;

    update_gmail_client_id(&mut database, Some(environment.clone()), Some(&environment));
    assert_eq!(database, None);

    update_gmail_client_id(
        &mut database,
        Some("database-id".to_owned()),
        Some(&environment),
    );
    assert_eq!(database.as_deref(), Some("database-id"));
}
