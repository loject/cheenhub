//! Локальная проверка покрытия строк перед отправкой изменений.
//!
//! Модуль измеряет покрытие тестами и сравнивает результат с сохранённым
//! эталоном только для текущей ОС. Обновление эталона — отдельная команда.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::XtaskResult;

const BASELINES_PATH: &str = "coverage/baselines.json";
const REPORT_PATH: &str = "target/llvm-cov/cheenhub-summary.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CoverageLines {
    covered: u64,
    count: u64,
}

/// Проверяет или обновляет эталон покрытия для текущей операционной системы.
///
/// При `check` снижение покрытия блокирует push. При `record` эталон
/// создаётся или повышается только после успешного измерения.
pub(super) fn run(args: Vec<String>) -> XtaskResult<()> {
    let action = match args.as_slice() {
        [action] if action == "check" || action == "record" => action.as_str(),
        _ => return Err("usage: cargo run -p xtask -- coverage-gate <check|record>".to_owned()),
    };

    let root = super::git_output(["rev-parse", "--show-toplevel"])?;
    env::set_current_dir(root.trim())
        .map_err(|error| format!("failed to enter repository root: {error}"))?;

    let platform = env::consts::OS;
    let path = Path::new(BASELINES_PATH);
    let mut baselines = read_baselines(path)?;

    // До дорогой инструментированной сборки проверяем, существует ли эталон.
    if action == "check" && !baselines.contains_key(platform) {
        return Err(missing_baseline_error(platform));
    }

    let measured = measure_coverage()?;
    match action {
        "check" => {
            check_for_os(&baselines, platform, measured)?;
            println!(
                "Coverage check passed for {platform}: {:.2}%",
                percentage(measured)
            );
        }
        "record" => {
            record_for_os(&mut baselines, platform, measured)?;
            let json = serde_json::to_string_pretty(&baselines)
                .map_err(|error| format!("failed to serialize coverage baselines: {error}"))?;
            fs::write(path, format!("{json}\n"))
                .map_err(|error| format!("failed to update {}: {error}", path.display()))?;
            println!(
                "Recorded {platform} coverage baseline: {:.2}%. Commit {} to retain it.",
                percentage(measured),
                path.display()
            );
        }
        _ => unreachable!("validated coverage action"),
    }

    Ok(())
}

fn read_baselines(path: &Path) -> XtaskResult<BTreeMap<String, CoverageLines>> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let baselines: BTreeMap<String, CoverageLines> = serde_json::from_str(&source)
        .map_err(|error| format!("invalid {}: {error}", path.display()))?;

    for (platform, lines) in &baselines {
        validate_lines(*lines).map_err(|error| format!("invalid {platform} baseline: {error}"))?;
    }

    Ok(baselines)
}

fn measure_coverage() -> XtaskResult<CoverageLines> {
    let report_path = Path::new(REPORT_PATH);
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to prepare coverage directory: {error}"))?;
    }

    println!("Measuring workspace line coverage (this may take a while)...");
    let status = Command::new("cargo")
        .args([
            "llvm-cov",
            "--workspace",
            "--all-targets",
            "--json",
            "--summary-only",
            "--output-path",
            REPORT_PATH,
        ])
        .status()
        .map_err(|error| {
            format!(
                "could not start cargo llvm-cov: {error}. Install cargo-llvm-cov and the llvm-tools-preview rustup component."
            )
        })?;
    if !status.success() {
        return Err(format!(
            "cargo llvm-cov failed (exit status: {status}); coverage gate rejected the push"
        ));
    }

    let report = fs::read_to_string(report_path)
        .map_err(|error| format!("failed to read {}: {error}", report_path.display()))?;
    parse_report(&report)
}

fn parse_report(source: &str) -> XtaskResult<CoverageLines> {
    let report: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| format!("invalid LLVM coverage JSON: {error}"))?;
    let datasets = report
        .get("data")
        .and_then(serde_json::Value::as_array)
        .filter(|datasets| !datasets.is_empty())
        .ok_or_else(|| "LLVM coverage report is missing data".to_owned())?;

    let mut total = CoverageLines {
        covered: 0,
        count: 0,
    };
    for dataset in datasets {
        let lines = dataset.get("totals").and_then(|totals| totals.get("lines"));
        let covered = lines
            .and_then(|lines| lines.get("covered"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "LLVM coverage report is missing lines.covered".to_owned())?;
        let count = lines
            .and_then(|lines| lines.get("count"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "LLVM coverage report is missing lines.count".to_owned())?;

        validate_lines(CoverageLines { covered, count })?;
        total.covered = total
            .covered
            .checked_add(covered)
            .ok_or_else(|| "LLVM covered-line count overflow".to_owned())?;
        total.count = total
            .count
            .checked_add(count)
            .ok_or_else(|| "LLVM total-line count overflow".to_owned())?;
    }

    validate_lines(total)?;
    Ok(total)
}

fn validate_lines(lines: CoverageLines) -> XtaskResult<()> {
    if lines.count == 0 || lines.covered > lines.count {
        return Err(format!(
            "invalid line coverage: {} covered out of {}",
            lines.covered, lines.count
        ));
    }
    Ok(())
}

fn missing_baseline_error(platform: &str) -> String {
    format!(
        "missing {platform} coverage baseline in {BASELINES_PATH}; run `cargo run -p xtask -- coverage-gate record` on {platform}, then commit the baseline"
    )
}

fn check_for_os(
    baselines: &BTreeMap<String, CoverageLines>,
    platform: &str,
    measured: CoverageLines,
) -> XtaskResult<()> {
    let baseline = baselines
        .get(platform)
        .copied()
        .ok_or_else(|| missing_baseline_error(platform))?;
    validate_lines(baseline)?;
    validate_lines(measured)?;

    // Сравнение целых долей не пропускает регрессии, скрытые округлением.
    if u128::from(measured.covered) * u128::from(baseline.count)
        < u128::from(baseline.covered) * u128::from(measured.count)
    {
        return Err(format!(
            "{platform} coverage decreased: {:.4}% < {:.4}% baseline; push rejected",
            percentage(measured),
            percentage(baseline)
        ));
    }
    Ok(())
}

fn record_for_os(
    baselines: &mut BTreeMap<String, CoverageLines>,
    platform: &str,
    measured: CoverageLines,
) -> XtaskResult<()> {
    validate_lines(measured)?;
    if baselines.contains_key(platform) {
        check_for_os(baselines, platform, measured)?;
    }
    baselines.insert(platform.to_owned(), measured);
    Ok(())
}

fn percentage(lines: CoverageLines) -> f64 {
    (lines.covered as f64) * 100.0 / (lines.count as f64)
}

#[cfg(test)]
mod tests;
