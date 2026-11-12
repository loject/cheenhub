use super::*;

#[test]
fn validates_archive_checksum_before_unpacking() {
    assert!(
        verify_bytes(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .is_ok()
    );
    assert!(
        verify_bytes(
            b"modified",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )
        .is_err()
    );
}

#[test]
fn cache_fingerprint_changes_when_sources_or_platform_change() {
    let initial = fingerprint(&[b"linux", b"source one"]);

    assert_ne!(initial, fingerprint(&[b"windows", b"source one"]));
    assert_ne!(initial, fingerprint(&[b"linux", b"source two"]));
    assert_ne!(fingerprint(&[b"a", b"bc"]), fingerprint(&[b"ab", b"c"]));
}

#[test]
fn windows_pkg_config_uses_only_the_static_prefix_and_pkgconf() {
    let mut command = std::process::Command::new("sh");

    configure_pkg_config_environment(
        &mut command,
        "/d/build/ffmpeg-static",
        "/mingw64/lib/pkgconfig",
        true,
    );

    let variables = command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        variables.get("PKG_CONFIG_PATH"),
        Some(&"/d/build/ffmpeg-static/lib/pkgconfig")
    );
    assert_eq!(
        variables.get("PKG_CONFIG_LIBDIR"),
        Some(&"/d/build/ffmpeg-static/lib/pkgconfig")
    );
    assert_eq!(variables.get("PKG_CONFIG"), Some(&"pkgconf"));
}

#[test]
fn linux_pkg_config_keeps_system_library_search_paths() {
    let mut command = std::process::Command::new("sh");

    configure_pkg_config_environment(
        &mut command,
        "/build/ffmpeg-static",
        "/usr/lib/pkgconfig",
        false,
    );

    let variables = command
        .get_envs()
        .filter_map(|(name, value)| Some((name.to_str()?, value?.to_str()?)))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        variables.get("PKG_CONFIG_PATH"),
        Some(&"/build/ffmpeg-static/lib/pkgconfig:/usr/lib/pkgconfig")
    );
    assert!(!variables.contains_key("PKG_CONFIG_LIBDIR"));
    assert!(!variables.contains_key("PKG_CONFIG"));
}
