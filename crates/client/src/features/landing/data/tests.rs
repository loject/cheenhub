use super::{
    android_apk_url_for_release, linux_deb_url_for_release, normalize_release_version_tag,
    windows_installer_url_for_release,
};

#[test]
fn release_version_tag_adds_missing_prefix() {
    assert_eq!(normalize_release_version_tag("0.18.1"), "v0.18.1");
}

#[test]
fn release_version_tag_preserves_existing_prefix() {
    assert_eq!(normalize_release_version_tag("v0.18.1"), "v0.18.1");
}

#[test]
fn windows_installer_url_uses_same_version_for_tag_and_filename() {
    assert_eq!(
        windows_installer_url_for_release("v0.18.1"),
        "https://github.com/loject/cheenhub/releases/download/v0.18.1/cheenhub-v0.18.1-windows-x64-setup.exe"
    );
}

#[test]
fn linux_deb_url_uses_same_version_for_tag_and_filename() {
    assert_eq!(
        linux_deb_url_for_release("v0.18.1"),
        "https://github.com/loject/cheenhub/releases/download/v0.18.1/cheenhub-v0.18.1-linux-x64.deb"
    );
}

#[test]
fn android_apk_url_uses_same_version_for_tag_and_filename() {
    assert_eq!(
        android_apk_url_for_release("v0.18.1"),
        "https://github.com/loject/cheenhub/releases/download/v0.18.1/cheenhub-v0.18.1-android.apk"
    );
}
