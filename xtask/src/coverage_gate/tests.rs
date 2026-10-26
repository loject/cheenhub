use std::collections::BTreeMap;

use super::{CoverageLines, check_for_os, parse_report, record_for_os};

fn lines(covered: u64, count: u64) -> CoverageLines {
    CoverageLines { covered, count }
}

#[test]
fn aggregates_valid_llvm_line_totals() {
    let report = r#"{"data":[{"totals":{"lines":{"count":100,"covered":80}}},{"totals":{"lines":{"count":50,"covered":45}}}]}"#;

    let lines = parse_report(report).expect("valid coverage report");

    assert_eq!(
        lines,
        CoverageLines {
            covered: 125,
            count: 150
        }
    );
}

#[test]
fn rejects_empty_or_invalid_report_totals() {
    for report in [
        r#"{"data":[]}"#,
        r#"{"data":[{"totals":{"lines":{"count":0,"covered":0}}}]}"#,
        r#"{"data":[{"totals":{"lines":{"count":10,"covered":11}}}]}"#,
        r#"{"data":[{"totals":{}}]}"#,
        "invalid json",
    ] {
        assert!(
            parse_report(report).is_err(),
            "unexpected valid report: {report}"
        );
    }
}

#[test]
fn refuses_push_without_baseline_for_host_os() {
    let mut baselines = BTreeMap::new();
    baselines.insert("linux".to_owned(), lines(80, 100));

    let error = check_for_os(&baselines, "windows", lines(90, 100))
        .expect_err("Windows baseline is required");

    assert!(error.contains("windows"));
    assert!(error.contains("record"));
}

#[test]
fn compares_exact_ratios_without_rounding() {
    let mut baselines = BTreeMap::new();
    baselines.insert("linux".to_owned(), lines(95, 100));

    assert!(check_for_os(&baselines, "linux", lines(190, 200)).is_ok());
    assert!(check_for_os(&baselines, "linux", lines(951, 1000)).is_ok());
    assert!(check_for_os(&baselines, "linux", lines(949, 1000)).is_err());
}

#[test]
fn records_new_host_without_changing_other_platform() {
    let mut baselines = BTreeMap::new();
    baselines.insert("linux".to_owned(), lines(90, 100));

    record_for_os(&mut baselines, "windows", lines(80, 100)).expect("first baseline for Windows");

    assert_eq!(baselines["linux"], lines(90, 100));
    assert_eq!(baselines["windows"], lines(80, 100));
}

#[test]
fn refuses_to_record_lower_coverage() {
    let mut baselines = BTreeMap::new();
    baselines.insert("linux".to_owned(), lines(90, 100));

    assert!(record_for_os(&mut baselines, "linux", lines(89, 100)).is_err());
    assert_eq!(baselines["linux"], lines(90, 100));
}
