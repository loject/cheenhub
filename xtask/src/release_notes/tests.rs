use super::*;

#[test]
fn missing_rc_notes_use_default_description() {
    let notes = ReleaseNotes::read("v987654321.0.0-rc.1").unwrap();

    assert!(!notes.github.is_empty());
}

#[test]
fn missing_stable_notes_remain_an_error() {
    assert!(ReleaseNotes::read("v987654321.0.0").is_err());
}

#[test]
fn missing_invalid_rc_notes_remain_an_error() {
    assert!(ReleaseNotes::read("v987654321.0.0-rc.0").is_err());
}

#[test]
fn extracts_independent_github_and_rustore_descriptions() {
    let notes = ReleaseNotes::parse(
        "v1.2.3",
        "# v1.2.3\n\n## GitHub\n\n### Новое\n- Все платформы.\n\n## RuStore\n\n• Только Android.\n",
    )
    .unwrap();

    assert_eq!(notes.github, "### Новое\n- Все платформы.");
    assert_eq!(notes.rustore, "• Только Android.");
}

#[test]
fn rejects_missing_empty_duplicate_and_unknown_sections() {
    for contents in [
        "# v1.2.3\n## GitHub\nОписание",
        "# v1.2.3\n## GitHub\n\n## RuStore\nОписание",
        "# v1.2.3\n## GitHub\nОписание\n## RuStore\n  ",
        "# v1.2.3\n## GitHub\nОписание\n## GitHub\nДубль\n## RuStore\nТекст",
        "# v1.2.3\n## GitHub\nОписание\n## Rustore\nТекст",
        "# v1.2.3\nНезадействованный текст\n## GitHub\nОписание\n## RuStore\nТекст",
    ] {
        assert!(
            ReleaseNotes::parse("v1.2.3", contents).is_err(),
            "{contents}"
        );
    }
}

#[test]
fn rejects_description_for_a_different_release() {
    let contents = "# v1.2.2\n## GitHub\nОписание\n## RuStore\nТекст";

    assert!(ReleaseNotes::parse("v1.2.3", contents).is_err());
}

#[test]
fn limits_rustore_description_by_unicode_characters() {
    let accepted = format!(
        "# v1.2.3\n## GitHub\nОписание\n## RuStore\n{}",
        "Я".repeat(5000)
    );
    let rejected = format!("{accepted}Я");

    assert!(ReleaseNotes::parse("v1.2.3", &accepted).is_ok());
    assert!(ReleaseNotes::parse("v1.2.3", &rejected).is_err());
}

#[test]
fn rejects_unsafe_or_unversioned_file_names() {
    for tag in [
        "../v1.2.3",
        "v1/2/3",
        "v1.2.3\\other",
        "dev",
        "v1.2.3\n",
        "v",
    ] {
        assert!(notes_path(tag).is_err(), "{tag:?}");
    }

    assert_eq!(
        notes_path("v1.2.3-rc.1").unwrap(),
        Path::new("releases/v1.2.3-rc.1.md")
    );
}
