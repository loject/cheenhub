use super::*;

#[test]
fn reload_state_debounces_chain_changes_and_invalid_candidates() {
    let applied = CertificateIdentity {
        leaf_fingerprint: "leaf-a".to_owned(),
        chain_fingerprint: "chain-a".to_owned(),
        key_source: key_source("key-a"),
    };
    let changed_chain = CertificateIdentity {
        leaf_fingerprint: "leaf-a".to_owned(),
        chain_fingerprint: "chain-b".to_owned(),
        key_source: key_source("key-a"),
    };
    let changed_leaf = CertificateIdentity {
        leaf_fingerprint: "leaf-b".to_owned(),
        chain_fingerprint: "chain-c".to_owned(),
        key_source: key_source("key-b"),
    };
    let mut state = ReloadState::default();
    assert!(matches!(
        state.observe(applied.clone(), Some(&applied)),
        ReloadDecision::Unchanged
    ));
    assert!(matches!(
        state.observe(changed_chain.clone(), Some(&applied)),
        ReloadDecision::Detected
    ));
    assert!(matches!(
        state.observe(changed_chain, Some(&applied)),
        ReloadDecision::Apply
    ));
    // Ошибка чтения пары сбрасывает подтверждение, не заменяя active config.
    state.clear_pending();
    assert!(matches!(
        state.observe(changed_leaf.clone(), Some(&applied)),
        ReloadDecision::Detected
    ));
    assert!(matches!(
        state.observe(changed_leaf, Some(&applied)),
        ReloadDecision::Apply
    ));
}

fn key_source(name: &str) -> KeySourceIdentity {
    #[cfg(unix)]
    {
        KeySourceIdentity::Unix {
            canonical_path: PathBuf::from(name),
            device: 1,
            inode: 1,
            length: 1,
            modified: None,
        }
    }
    #[cfg(not(unix))]
    {
        KeySourceIdentity::Portable {
            canonical_path: PathBuf::from(name),
            length: 1,
            modified: None,
        }
    }
}

#[cfg(unix)]
#[test]
fn key_symlink_target_change_requires_two_polls() {
    use std::os::unix::fs::symlink;

    let directory =
        std::env::temp_dir().join(format!("cheenhub-key-source-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("test directory is created");
    let first = directory.join("first-key.pem");
    let second = directory.join("second-key.pem");
    let link = directory.join("current-key.pem");
    std::fs::write(&first, b"same length key").expect("first key is written");
    std::fs::write(&second, b"same length key").expect("second key is written");
    symlink(&first, &link).expect("first key symlink is created");
    let applied = CertificateIdentity {
        leaf_fingerprint: "leaf".to_owned(),
        chain_fingerprint: "chain".to_owned(),
        key_source: key_source_identity(link.to_str().expect("utf-8 path"))
            .expect("identity loads"),
    };
    std::fs::remove_file(&link).expect("old symlink is removed");
    symlink(&second, &link).expect("second key symlink is created");
    let changed = CertificateIdentity {
        leaf_fingerprint: "leaf".to_owned(),
        chain_fingerprint: "chain".to_owned(),
        key_source: key_source_identity(link.to_str().expect("utf-8 path"))
            .expect("identity loads"),
    };
    assert_ne!(applied.key_source, changed.key_source);
    let mut state = ReloadState::default();
    assert!(matches!(
        state.observe(changed.clone(), Some(&applied)),
        ReloadDecision::Detected
    ));
    assert!(matches!(
        state.observe(changed, Some(&applied)),
        ReloadDecision::Apply
    ));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn key_metadata_change_requires_two_polls() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-key-source-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("test directory is created");
    let key = directory.join("key.pem");
    std::fs::write(&key, b"key").expect("key is written");
    let applied_source =
        key_source_identity(key.to_str().expect("utf-8 path")).expect("identity loads");
    std::fs::write(&key, b"key with changed metadata").expect("key is replaced");
    let changed_source =
        key_source_identity(key.to_str().expect("utf-8 path")).expect("identity loads");
    assert_ne!(applied_source, changed_source);
    let applied = CertificateIdentity {
        leaf_fingerprint: "leaf".to_owned(),
        chain_fingerprint: "chain".to_owned(),
        key_source: applied_source,
    };
    let changed = CertificateIdentity {
        leaf_fingerprint: "leaf".to_owned(),
        chain_fingerprint: "chain".to_owned(),
        key_source: changed_source,
    };
    let mut state = ReloadState::default();
    assert!(matches!(
        state.observe(changed.clone(), Some(&applied)),
        ReloadDecision::Detected
    ));
    assert!(matches!(
        state.observe(changed, Some(&applied)),
        ReloadDecision::Apply
    ));
    let _ = std::fs::remove_dir_all(directory);
}
