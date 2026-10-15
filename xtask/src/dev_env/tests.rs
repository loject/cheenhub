//! Сквозные проверки подготовки .env без доступа к реальной конфигурации.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::ensure_file;

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cheenhub-dev-env-test-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
}

#[test]
fn creates_missing_env_then_second_run_is_identical() {
    let dir = TestDir::new();
    let template = dir.path("example");
    let env = dir.path(".env");
    write(&template, "# Local configuration\nPOSTGRES_DB=cheenhub\n");

    ensure_file(&env, &template).unwrap();
    let first = fs::read(&env).unwrap();
    ensure_file(&env, &template).unwrap();

    assert_eq!(fs::read(&env).unwrap(), first);
    let content = String::from_utf8(first).unwrap();
    assert!(content.contains("DATABASE_URL=postgres://"));
    assert!(content.contains("JWT_ED25519_PRIVATE_KEY_BASE64="));
    assert!(content.contains("CHEENHUB_JWT_PUBLIC_KEY_BASE64="));
    assert!(content.contains("# Local configuration"));
}

#[test]
fn does_not_copy_published_example_secrets() {
    let dir = TestDir::new();
    let template = dir.path("example");
    let env = dir.path(".env");
    write(
        &template,
        "POSTGRES_USER=postgres\nPOSTGRES_DB=cheenhub\nPOSTGRES_PORT=5432\nPOSTGRES_PASSWORD=published-example\nDATABASE_URL=postgres://postgres:published-example@127.0.0.1:5432/cheenhub\nJWT_ED25519_PRIVATE_KEY_BASE64=published-jwt\nCHEENHUB_JWT_PUBLIC_KEY_BASE64=published-public\n",
    );

    ensure_file(&env, &template).unwrap();
    let generated = fs::read_to_string(&env).unwrap();

    assert!(!generated.contains("published-example"));
    assert!(!generated.contains("published-jwt"));
    assert!(!generated.contains("published-public"));
    assert!(generated.contains("POSTGRES_PASSWORD="));
    assert!(generated.contains("JWT_ED25519_PRIVATE_KEY_BASE64="));
}

#[test]
fn preserves_existing_file_if_database_settings_conflict() {
    let dir = TestDir::new();
    let template = dir.path("example");
    let env = dir.path(".env");
    write(&template, "# empty\n");
    let original = "DATABASE_URL=postgres://u:correct@localhost/db\nPOSTGRES_PASSWORD=wrong\n";
    write(&env, original);

    assert!(ensure_file(&env, &template).is_err());
    assert_eq!(fs::read_to_string(&env).unwrap(), original);
}

#[test]
fn preserves_existing_file_if_jwt_key_is_invalid() {
    let dir = TestDir::new();
    let template = dir.path("example");
    let env = dir.path(".env");
    write(&template, "# empty\n");
    let original =
        "DATABASE_URL=postgres://u:pass@localhost/db\nJWT_ED25519_PRIVATE_KEY_BASE64=invalid\n";
    write(&env, original);

    assert!(ensure_file(&env, &template).is_err());
    assert_eq!(fs::read_to_string(&env).unwrap(), original);
}

#[test]
fn preserves_existing_valid_keys_and_database_url() {
    let dir = TestDir::new();
    let template = dir.path("example");
    let env = dir.path(".env");
    write(&template, "# empty\n");
    let original = "DATABASE_URL=postgres://u:pass@localhost:5433/db\n";
    write(&env, original);

    ensure_file(&env, &template).unwrap();
    let first = fs::read(&env).unwrap();
    ensure_file(&env, &template).unwrap();

    assert_eq!(fs::read(&env).unwrap(), first);
    let updated = String::from_utf8(first).unwrap();
    assert!(updated.starts_with(original));
    assert!(updated.contains("POSTGRES_PASSWORD=pass"));
}
