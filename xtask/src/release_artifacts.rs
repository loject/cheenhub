//! Подготовка файлов релизных артефактов для публикации.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::XtaskResult;

pub(crate) fn run(args: Vec<String>) -> XtaskResult<()> {
    let action = args.first().map(String::as_str).unwrap_or("collect");
    match action {
        "collect" => collect_release_artifacts(&args[1..]),
        "publish" => publish_release_artifacts(&args[1..]),
        "-h" | "--help" | "help" => {
            println!(
                "Usage:\n  cargo run -p xtask -- release-artifacts collect <windows|linux|android> <source-dir> <output-dir> <release-tag>\n  cargo run -p xtask -- release-artifacts publish <release-tag> <asset-dir>"
            );
            Ok(())
        }
        unknown => Err(format!("unknown release-artifacts action: {unknown}")),
    }
}

fn collect_release_artifacts(args: &[String]) -> XtaskResult<()> {
    if args.len() != 4 {
        return Err(
            "release-artifacts collect expects <platform> <source-dir> <output-dir> <release-tag>."
                .to_owned(),
        );
    }

    let platform = ReleaseArtifactPlatform::parse(&args[0])?;
    let source_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);
    let release_tag = &args[3];

    fs::create_dir_all(output_dir)
        .map_err(|error| format!("failed to create {}: {error}", output_dir.display()))?;

    for artifact in platform.artifacts() {
        let source = find_unique_artifact(source_dir, artifact.extension)?;
        let destination = output_dir.join(artifact.file_name(release_tag));
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        println!("{}", destination.display());
    }

    Ok(())
}

fn publish_release_artifacts(args: &[String]) -> XtaskResult<()> {
    if args.len() != 2 {
        return Err("release-artifacts publish expects <release-tag> <asset-dir>.".to_owned());
    }

    let release_tag = &args[0];
    let asset_dir = Path::new(&args[1]);
    let assets = release_asset_files(asset_dir)?;

    ensure_github_release_exists(release_tag)?;
    let mut existing_assets = github_release_asset_names(release_tag)?;

    for asset in assets {
        let asset_name = file_name(&asset)?;
        if existing_assets.contains(&asset_name) {
            println!("Skipping existing release asset {asset_name}.");
            continue;
        }

        match upload_github_release_asset(release_tag, &asset) {
            Ok(()) => {
                println!("Uploaded release asset {asset_name}.");
                existing_assets.insert(asset_name);
            }
            Err(error) => {
                existing_assets = github_release_asset_names(release_tag)?;
                if existing_assets.contains(&asset_name) {
                    println!(
                        "Skipping release asset {asset_name}; it appeared during upload retry."
                    );
                } else {
                    return Err(error);
                }
            }
        }
    }

    Ok(())
}

enum ReleaseArtifactPlatform {
    Windows,
    Linux,
    Android,
}

impl ReleaseArtifactPlatform {
    fn parse(value: &str) -> XtaskResult<Self> {
        match value {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "android" => Ok(Self::Android),
            unknown => Err(format!("unknown release artifact platform: {unknown}")),
        }
    }

    fn artifacts(&self) -> &'static [ReleaseArtifact] {
        match self {
            Self::Windows => &[ReleaseArtifact {
                extension: "exe",
                suffix: "windows-x64-setup.exe",
            }],
            Self::Linux => &[
                ReleaseArtifact {
                    extension: "deb",
                    suffix: "linux-x64.deb",
                },
                ReleaseArtifact {
                    extension: "appimage",
                    suffix: "linux-x64.AppImage",
                },
            ],
            Self::Android => &[ReleaseArtifact {
                extension: "apk",
                suffix: "android.apk",
            }],
        }
    }
}

struct ReleaseArtifact {
    extension: &'static str,
    suffix: &'static str,
}

impl ReleaseArtifact {
    fn file_name(&self, release_tag: &str) -> String {
        format!("cheenhub-{release_tag}-{}", self.suffix)
    }
}

fn find_unique_artifact(source_dir: &Path, extension: &str) -> XtaskResult<PathBuf> {
    if !source_dir.exists() {
        return Err(format!(
            "release artifact source directory does not exist: {}",
            source_dir.display()
        ));
    }

    let mut matches = Vec::new();
    collect_artifacts_by_extension(source_dir, extension, &mut matches)?;
    matches.sort();

    match matches.as_slice() {
        [artifact] => Ok(artifact.clone()),
        [] => Err(format!(
            "no .{extension} release artifact found in {}.",
            source_dir.display()
        )),
        artifacts => Err(format!(
            "expected one .{extension} release artifact in {}, found: {}",
            source_dir.display(),
            artifacts
                .iter()
                .map(|artifact| artifact.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn collect_artifacts_by_extension(
    directory: &Path,
    extension: &str,
    matches: &mut Vec<PathBuf>,
) -> XtaskResult<()> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("failed to read {} entry: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_artifacts_by_extension(&path, extension, matches)?;
            continue;
        }

        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            matches.push(path);
        }
    }

    Ok(())
}

fn release_asset_files(asset_dir: &Path) -> XtaskResult<Vec<PathBuf>> {
    if !asset_dir.exists() {
        return Err(format!(
            "release asset directory does not exist: {}",
            asset_dir.display()
        ));
    }

    let mut assets = Vec::new();
    let entries = fs::read_dir(asset_dir)
        .map_err(|error| format!("failed to read {}: {error}", asset_dir.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("failed to read {} entry: {error}", asset_dir.display()))?
            .path();
        if path.is_file() {
            assets.push(path);
        }
    }
    assets.sort();

    if assets.is_empty() {
        return Err(format!(
            "no release assets found in {}.",
            asset_dir.display()
        ));
    }

    Ok(assets)
}

fn ensure_github_release_exists(release_tag: &str) -> XtaskResult<()> {
    if github_release_exists(release_tag)? {
        return Ok(());
    }

    let notes = "Release artifacts are attached incrementally as GitHub Actions jobs finish.";
    let output = command_output(
        Command::new("gh")
            .args(["release", "create", release_tag])
            .args(["--title", release_tag])
            .args(["--notes", notes])
            .arg("--verify-tag"),
    )?;
    if output.status.success() {
        println!("Created GitHub release {release_tag}.");
        return Ok(());
    }

    if github_release_exists(release_tag)? {
        println!("GitHub release {release_tag} was created by another job.");
        return Ok(());
    }

    Err(format!(
        "failed to create GitHub release {release_tag}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn github_release_exists(release_tag: &str) -> XtaskResult<bool> {
    let output = command_output(Command::new("gh").args(["release", "view", release_tag]))?;
    Ok(output.status.success())
}

fn github_release_asset_names(release_tag: &str) -> XtaskResult<BTreeSet<String>> {
    let output = checked_command(
        Command::new("gh").args([
            "release",
            "view",
            release_tag,
            "--json",
            "assets",
            "--jq",
            ".assets[].name",
        ]),
        &format!("failed to list GitHub release {release_tag} assets"),
    )?;

    Ok(output
        .lines()
        .filter_map(|line| {
            let name = line.trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            }
        })
        .collect())
}

fn upload_github_release_asset(release_tag: &str, asset: &Path) -> XtaskResult<()> {
    checked_status(
        Command::new("gh")
            .args(["release", "upload", release_tag])
            .arg(asset),
        &format!("failed to upload release asset {}", asset.display()),
    )
}

fn file_name(path: &Path) -> XtaskResult<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("release asset path has no file name: {}", path.display()))
}

fn checked_status(command: &mut Command, context: &str) -> XtaskResult<()> {
    let output = command_output(command)?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{context}: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}

fn checked_command(command: &mut Command, context: &str) -> XtaskResult<String> {
    let output = command_output(command)?;
    if !output.status.success() {
        return Err(format!(
            "{context}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command_output(command: &mut Command) -> XtaskResult<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))
}
