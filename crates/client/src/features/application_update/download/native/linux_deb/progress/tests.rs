use super::parse_percentage;

#[test]
fn reads_install_progress_with_architecture_and_colons_in_description() {
    assert_eq!(
        parse_percentage("pmstatus:cheenhub:amd64:42.5:Unpacking: CheenHub"),
        Some(43)
    );
}

#[test]
fn ignores_logs_downloads_errors_and_invalid_percentages() {
    for line in [
        "Unpacking cheenhub",
        "dlstatus:1:50:Downloading",
        "pmerror:cheenhub:50:Failed",
        "pmstatus:cheenhub:NaN:Unpacking",
        "pmstatus:cheenhub:101:Unpacking",
        "pmstatus:cheenhub:-1:Unpacking",
        "pmstatus:cheenhub:50",
    ] {
        assert_eq!(parse_percentage(line), None, "{line}");
    }
}

#[test]
fn reads_start_and_completion() {
    assert_eq!(parse_percentage("pmstatus:cheenhub:0:Preparing"), Some(0));
    assert_eq!(
        parse_percentage("pmstatus:cheenhub:100:Installed"),
        Some(100)
    );
}
