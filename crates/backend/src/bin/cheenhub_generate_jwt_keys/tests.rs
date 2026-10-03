use super::*;

#[test]
fn upsert_replaces_existing_value() {
    let content = "JWT_KEY_ID=old\nOTHER=value\n";

    let updated = upsert_env_value(content, "JWT_KEY_ID", "new");

    assert_eq!(updated, "JWT_KEY_ID=new\nOTHER=value\n");
}

#[test]
fn upsert_appends_missing_value() {
    let content = "OTHER=value\n";

    let updated = upsert_env_value(content, "JWT_KEY_ID", "new");

    assert_eq!(updated, "OTHER=value\nJWT_KEY_ID=new\n");
}

#[test]
fn reads_exported_and_quoted_values() {
    let content = "export JWT_KEY_ID=\"quoted\"\n";

    assert_eq!(env_value(content, "JWT_KEY_ID").as_deref(), Some("quoted"));
}
