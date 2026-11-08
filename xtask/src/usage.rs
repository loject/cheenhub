//! Справка по командам xtask; выполнение команд находится в их модулях.

use crate::XtaskResult;

/// Выводит справку, не изменяя файлы и состояние репозитория.
pub(super) fn print_usage() -> XtaskResult<()> {
    println!(
        "Release notes: cargo run -p xtask -- release-notes <check <tag>|export <tag> <directory>>"
    );
    println!(concat!(
        "Usage:\n",
        "  cargo run -p xtask -- build-ffmpeg\n",
        "  cargo run -p xtask -- coverage-gate <check|record>\n",
        "  cargo run -p xtask -- dev-env ensure [--file <path>]\n",
        "  cargo run -p xtask -- edge-contract [--image <local-nginx-image>]\n",
        "  cargo run -p xtask -- ensure-cargo-cli <crate> <binary> <version>\n",
        "  cargo run -p xtask -- line-stats\n",
        "  cargo run -p xtask -- release-version check\n",
        "  cargo run -p xtask -- release-version print-tag\n",
        "  cargo run -p xtask -- release-version tag [<release-tag>]\n",
        "  cargo run -p xtask -- release-artifacts collect <windows|linux|android> <source-dir> <output-dir> <release-tag>\n",
        "  cargo run -p xtask -- release-artifacts publish <release-tag> <asset-dir>"
    ));
    Ok(())
}
