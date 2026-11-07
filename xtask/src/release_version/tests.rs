use super::{newest_release_tag_from_refs, release_suggestions, release_target_from_tag};

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
    let next = release_suggestions("0.39.0", "v0.39.0-rc.1\n").unwrap();

    assert_eq!(next.rc, "0.39.0-rc.2");
}

#[test]
fn next_minor_rc_starts_new_series_after_stable_release() {
    let next = release_suggestions("0.39.0", "v0.39.0\n").unwrap();

    assert_eq!(next.rc, "0.40.0-rc.1");
}

#[test]
fn next_minor_rc_uses_workspace_rc_when_repository_has_no_tags() {
    let next = release_suggestions("0.39.0-rc.1", "").unwrap();

    assert_eq!(next.rc, "0.39.0-rc.2");
}

#[test]
fn published_stable_release_closes_later_created_rc_series() {
    let next = release_suggestions(
        "0.39.0-rc.3",
        "v0.39.0-rc.3\nv0.39.0-rc.2\nv0.39.0\nv0.38.3\n",
    )
    .unwrap();

    assert_eq!(next.minor, "0.40.0");
    assert_eq!(next.rc, "0.40.0-rc.1");
    assert_eq!(next.patch, "0.39.1");
    assert_eq!(next.major, "1.0.0");
}

#[test]
fn unpublished_rc_series_can_be_promoted_or_continued() {
    let next = release_suggestions("0.40.0-rc.2", "v0.40.0-rc.2\nv0.39.0\n").unwrap();

    assert_eq!(next.minor, "0.40.0");
    assert_eq!(next.rc, "0.40.0-rc.3");
}

#[test]
fn version_order_overrides_tag_creation_order_for_release_suggestions() {
    let next = release_suggestions(
        "0.39.0-rc.3",
        "native-preview\nv0.38.9\nv0.40.0-rc.2\nv0.40.0-rc.10\nv0.39.0\nv6.0\n",
    )
    .unwrap();

    assert_eq!(next.minor, "0.40.0");
    assert_eq!(next.rc, "0.40.0-rc.11");
}

#[test]
fn newer_stable_tag_prevents_suggestions_from_stale_workspace() {
    let next = release_suggestions("0.39.0-rc.3", "v0.40.0\nv0.39.0-rc.9\n").unwrap();

    assert_eq!(next.minor, "0.41.0");
    assert_eq!(next.rc, "0.41.0-rc.1");
    assert_eq!(next.patch, "0.40.1");
}

#[test]
fn newest_release_tag_skips_non_version_tags() {
    let tag = newest_release_tag_from_refs("native-preview\nv0.39.0-rc.1\nv0.39.0\n");

    assert_eq!(tag, Some("v0.39.0-rc.1"));
}
