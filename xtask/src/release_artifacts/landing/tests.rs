use super::*;

const TEMPLATE: &str = include_str!("../../../../crates/client/landing.html.template");

#[test]
fn updates_version_labels_and_all_download_links() {
    for version in ["1.2.3", "v1.2.3", "v1.2.3-abcdef0"] {
        let html = render(TEMPLATE, version).expect("valid version");
        let tag = format!("v{}", version.trim_start_matches('v'));

        assert!(html.contains(&format!("Открытый код  ·  {tag}  ·  бесплатно")));
        for platform in ["windows-x64-setup.exe", "linux-x64.deb", "android.apk"] {
            let url = format!(
                "https://github.com/loject/cheenhub/releases/download/{tag}/cheenhub-{tag}-{platform}"
            );
            assert_eq!(html.matches(&url).count(), 2, "{platform}: {version}");
        }
        assert!(!html.contains("@CHEENHUB_"));
        assert!(!html.contains("v0.33.2"));
        assert!(html.contains("<style>"));
        assert!(!html.contains("<script"));
    }
}

#[test]
fn rejects_invalid_versions_and_unresolved_markers() {
    for version in [
        "",
        "<script>",
        "1/2/3",
        "1.2.3&bad",
        "v1.2.3\n",
        "1.2",
        "1.2.3-",
        "vv1.2.3",
    ] {
        assert!(render(TEMPLATE, version).is_err(), "{version:?}");
    }
    assert!(render("fixed version", "1.2.3").is_err());
    assert!(render("@CHEENHUB_RELEASE_VERSION@ @CHEENHUB_UNKNOWN@", "1.2.3").is_err());
}

#[test]
fn preserves_previous_artifact_when_rendering_fails() {
    let directory = std::env::temp_dir().join(format!(
        "cheenhub-landing-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    fs::create_dir(&directory).expect("create isolated directory");
    let output = directory.join("landing.html");
    fs::write(&output, "previous artifact").expect("write previous artifact");

    let result = write_landing(
        &output,
        "@CHEENHUB_RELEASE_VERSION@ @CHEENHUB_UNKNOWN@",
        "1.2.3",
    );

    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(&output).expect("read artifact"),
        "previous artifact"
    );
    write_landing(&output, TEMPLATE, "v1.2.3").expect("replace artifact");
    assert!(
        fs::read_to_string(&output)
            .expect("read updated artifact")
            .contains("v1.2.3")
    );
    assert_eq!(fs::read_dir(&directory).expect("read directory").count(), 1);
    fs::remove_dir_all(directory).expect("remove isolated directory");
}
