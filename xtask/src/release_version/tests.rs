use super::{newest_release_tag_from_refs, next_minor_rc_version, release_target_from_tag};

#[test]
fn normalizes_release_tag_to_workspace_version() {
    let release = release_target_from_tag("v0.13.0").expect("valid release tag");

    assert_eq!(release.tag, "v0.13.0");
    assert_eq!(release.version, "0.13.0");
}

#[test]
fn accepts_release_version_without_prefix() {
    let release = release_target_from_tag("0.13.0").expect("valid release version");

    assert_eq!(release.tag, "v0.13.0");
    assert_eq!(release.version, "0.13.0");
}

#[test]
fn next_minor_rc_advances_latest_tag_even_with_stable_workspace_version() {
    let next =
        next_minor_rc_version("0.39.0", Some("v0.39.0-rc.1")).expect("valid release candidate");

    assert_eq!(next, "0.39.0-rc.2");
}

#[test]
fn next_minor_rc_starts_new_series_after_stable_release() {
    let next = next_minor_rc_version("0.39.0", Some("v0.39.0")).expect("valid stable release");

    assert_eq!(next, "0.40.0-rc.1");
}

#[test]
fn next_minor_rc_uses_workspace_rc_when_repository_has_no_tags() {
    let next =
        next_minor_rc_version("0.39.0-rc.1", None).expect("valid workspace release candidate");

    assert_eq!(next, "0.39.0-rc.2");
}

#[test]
fn newest_release_tag_skips_non_version_tags() {
    let tag = newest_release_tag_from_refs("native-preview\nv0.39.0-rc.1\nv0.39.0\n");

    assert_eq!(tag, Some("v0.39.0-rc.1"));
}
