use super::deb_version_matches;

#[test]
fn accepts_debian_revision_for_release_version() {
    assert!(deb_version_matches("0.24.1-1", "0.24.1"));
    assert!(deb_version_matches("1:0.24.1-1", "0.24.1"));
    assert!(deb_version_matches("0.24.1+build1", "0.24.1"));
}

#[test]
fn rejects_different_release_version() {
    assert!(!deb_version_matches("0.24.0-1", "0.24.1"));
    assert!(!deb_version_matches("0.25.0", "0.24.1"));
}
