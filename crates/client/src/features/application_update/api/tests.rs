use std::cmp::Ordering;

use super::{compare_versions, normalize_release_version};

#[test]
fn normalizes_github_release_tags() {
    assert_eq!(normalize_release_version("v0.8.1"), "0.8.1");
    assert_eq!(normalize_release_version("V1.2.3"), "1.2.3");
}

#[test]
fn compares_semver_like_versions() {
    assert_eq!(compare_versions("0.8.1", "0.8.0"), Ordering::Greater);
    assert_eq!(compare_versions("0.8.0", "0.8.0"), Ordering::Equal);
    assert_eq!(compare_versions("0.7.9", "0.8.0"), Ordering::Less);
}
