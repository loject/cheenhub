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
