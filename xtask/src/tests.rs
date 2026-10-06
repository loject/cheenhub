use super::{push_git_tag_command, release_target_from_tag, replace_workspace_version};

#[test]
fn pushes_only_the_created_tag_to_origin() {
    let command = push_git_tag_command("v0.28.6");
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(command.get_program(), "git");
    assert_eq!(
        arguments,
        ["push", "origin", "refs/tags/v0.28.6:refs/tags/v0.28.6"]
    );
}

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
fn replaces_workspace_package_version_only() {
    let content = "[workspace]\nmembers = [\"xtask\"]\n\n[workspace.package]\nversion = \"0.12.0\"\nedition = \"2024\"\n\n[package]\nversion = \"ignored\"\n";

    let updated = replace_workspace_version(content, "0.13.0").expect("updated manifest");

    assert!(updated.contains("[workspace.package]\nversion = \"0.13.0\"\nedition = \"2024\""));
    assert!(updated.contains("[package]\nversion = \"ignored\""));
}
