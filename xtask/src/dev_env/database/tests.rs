//! Проверки сохранения параметров PostgreSQL и отказа при конфликтах.

use super::ensure;
use crate::dev_env::env_file::EnvFile;

#[test]
fn generates_new_password_and_matching_url_once() {
    let mut env = EnvFile::new(String::new());

    ensure(&mut env).unwrap();
    let first = env.content().to_owned();
    ensure(&mut env).unwrap();

    assert_eq!(env.content(), first);
    assert_eq!(
        env.nonempty("POSTGRES_USER").unwrap().as_deref(),
        Some("postgres")
    );
    assert_eq!(
        env.nonempty("POSTGRES_PORT").unwrap().as_deref(),
        Some("5432")
    );
    let password = env.nonempty("POSTGRES_PASSWORD").unwrap().unwrap();
    assert!(password.len() >= 40);
    assert!(
        env.nonempty("DATABASE_URL")
            .unwrap()
            .unwrap()
            .contains(&password)
    );
}

#[test]
fn derives_missing_compose_credentials_from_existing_url() {
    let mut env =
        EnvFile::new("DATABASE_URL=postgres://app:old-password@127.0.0.1:5444/old-db\n".into());

    ensure(&mut env).unwrap();

    assert_eq!(
        env.nonempty("POSTGRES_USER").unwrap().as_deref(),
        Some("app")
    );
    assert_eq!(
        env.nonempty("POSTGRES_PASSWORD").unwrap().as_deref(),
        Some("old-password")
    );
    assert_eq!(
        env.nonempty("POSTGRES_DB").unwrap().as_deref(),
        Some("old-db")
    );
    assert_eq!(
        env.nonempty("POSTGRES_PORT").unwrap().as_deref(),
        Some("5444")
    );
}

#[test]
fn decodes_percent_encoded_existing_password() {
    let mut env =
        EnvFile::new("DATABASE_URL=postgres://app:old%40password@localhost:5432/db\n".into());

    ensure(&mut env).unwrap();

    assert_eq!(
        env.nonempty("POSTGRES_PASSWORD").unwrap().as_deref(),
        Some("old@password")
    );
}

#[test]
fn refuses_conflicting_password_without_changing_input() {
    let mut env = EnvFile::new(
        "DATABASE_URL=postgres://app:one@localhost/db\nPOSTGRES_PASSWORD=two\n".into(),
    );
    let before = env.content().to_owned();

    assert!(ensure(&mut env).is_err());

    assert_eq!(env.content(), before);
}

#[test]
fn refuses_unusable_database_url() {
    let mut env = EnvFile::new("DATABASE_URL=sqlite://localhost/test\n".into());

    assert!(ensure(&mut env).is_err());
}
