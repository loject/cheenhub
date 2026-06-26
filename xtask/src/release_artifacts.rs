//! Подготовка файлов релизных артефактов для публикации.

use std::fs;
use std::path::{Path, PathBuf};

use crate::XtaskResult;

pub(crate) fn run(args: Vec<String>) -> XtaskResult<()> {
    let action = args.first().map(String::as_str).unwrap_or("collect");
    match action {
        "collect" => collect_release_artifacts(&args[1..]),
        "-h" | "--help" | "help" => {
            println!(
                "Usage: cargo run -p xtask -- release-artifacts collect <windows|linux|android> <source-dir> <output-dir> <release-tag>"
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
            Self::Windows => &[
                ReleaseArtifact {
                    extension: "exe",
                    suffix: "windows-x64-setup.exe",
                },
                ReleaseArtifact {
                    extension: "msi",
                    suffix: "windows-x64.msi",
                },
            ],
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
