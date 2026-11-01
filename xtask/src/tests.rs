use super::{push_git_release_command, replace_workspace_version};

#[test]
fn pushes_main_and_the_created_tag_to_origin_atomically() {
    let command = push_git_release_command("v0.28.6");
    let arguments = command
        .get_args()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    assert_eq!(command.get_program(), "git");
    assert_eq!(
        arguments,
        [
            "push",
            "--atomic",
            "origin",
            "refs/heads/main:refs/heads/main",
            "refs/tags/v0.28.6:refs/tags/v0.28.6",
        ]
    );
}

#[test]
fn replaces_workspace_package_version_only() {
    let content = "[workspace]\nmembers = [\"xtask\"]\n\n[workspace.package]\nversion = \"0.12.0\"\nedition = \"2024\"\n\n[package]\nversion = \"ignored\"\n";

    let updated = replace_workspace_version(content, "0.13.0").expect("updated manifest");

    assert!(updated.contains("[workspace.package]\nversion = \"0.13.0\"\nedition = \"2024\""));
    assert!(updated.contains("[package]\nversion = \"ignored\""));
}
