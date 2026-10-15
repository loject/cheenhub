//! Проверки сохранения существующих параметров dotenv.

use super::EnvFile;

#[test]
fn preserves_unrelated_lines_when_updating_value() {
    let mut env = EnvFile::new("# note\nFOO=bar\nJWT_KEY_ID=old\n".into());

    env.set("JWT_KEY_ID", "new").unwrap();

    assert_eq!(env.content(), "# note\nFOO=bar\nJWT_KEY_ID=new\n");
}

#[test]
fn appends_missing_value() {
    let mut env = EnvFile::new("FOO=bar\n".into());

    env.set("POSTGRES_PASSWORD", "generated").unwrap();

    assert_eq!(env.content(), "FOO=bar\nPOSTGRES_PASSWORD=generated\n");
}

#[test]
fn rejects_duplicate_active_values() {
    let env = EnvFile::new("POSTGRES_PASSWORD=a\nexport POSTGRES_PASSWORD=b\n".into());

    assert!(env.get("POSTGRES_PASSWORD").is_err());
}

#[test]
fn removes_only_active_secret_value() {
    let mut env =
        EnvFile::new("# POSTGRES_PASSWORD=example\nPOSTGRES_PASSWORD=unsafe\nFOO=bar\n".into());

    env.remove("POSTGRES_PASSWORD").unwrap();

    assert_eq!(env.content(), "# POSTGRES_PASSWORD=example\nFOO=bar\n");
}

#[test]
fn reads_quoted_values_but_ignores_comments() {
    let env = EnvFile::new("# JWT_KEY_ID=old\nexport JWT_KEY_ID='new'\n".into());

    assert_eq!(env.get("JWT_KEY_ID").unwrap().as_deref(), Some("new"));
}
