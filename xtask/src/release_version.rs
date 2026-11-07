//! Подготовка релизных версий, выбор следующего тега и интерактивное меню.

use std::env;
use std::io::{self, Write};
use std::path::Path;

use super::{
    ROOT_MANIFEST, Workspace, XtaskResult, commit_release_changes, create_git_tag,
    ensure_clean_worktree, git_output, push_git_release, run_release_build,
    update_workspace_version,
};

/// Выполняет проверку, вывод или создание релизного тега.
pub(super) fn run_release_version(args: Vec<String>) -> XtaskResult<()> {
    let action = args.first().map(String::as_str).unwrap_or("check");
    let repo_root = git_output(["rev-parse", "--show-toplevel"])?;
    env::set_current_dir(repo_root.trim())
        .map_err(|error| format!("failed to enter repository root: {error}"))?;

    match action {
        "check" => {
            let workspace = Workspace::read(Path::new(ROOT_MANIFEST))?;
            let expected_tag = release_tag_from_version(&workspace.version);
            workspace.check_release_version()?;
            println!("Release version check passed: {expected_tag}");
        }
        "print-tag" => {
            let workspace = Workspace::read(Path::new(ROOT_MANIFEST))?;
            let expected_tag = release_tag_from_version(&workspace.version);
            workspace.check_release_version()?;
            println!("{expected_tag}");
        }
        "tag" => {
            let release = release_target_from_args(&args[1..])?;
            ensure_clean_worktree("before updating the release version")?;
            crate::release_notes::ReleaseNotes::read(&release.tag)?;
            println!("Release notes validated: {}", release.tag);
            update_workspace_version(Path::new(ROOT_MANIFEST), &release.version)?;
            let workspace = Workspace::read(Path::new(ROOT_MANIFEST))?;
            workspace.check_release_version()?;
            run_release_build()?;
            commit_release_changes(&release.tag)?;
            ensure_clean_worktree("before creating the git tag")?;
            create_git_tag(&release.tag)?;
            push_git_release(&release.tag)?;
            println!(
                "Created git tag {} and pushed main and the tag to origin.",
                release.tag
            );
        }
        "-h" | "--help" | "help" => {
            println!(
                "Usage: cargo run -p xtask -- release-version <check|print-tag|tag [<release-tag>]>"
            );
        }
        unknown => {
            return Err(format!("unknown release-version action: {unknown}"));
        }
    }

    Ok(())
}

struct ReleaseTarget {
    tag: String,
    version: String,
}

fn release_target_from_args(args: &[String]) -> XtaskResult<ReleaseTarget> {
    match args {
        [] => prompt_release_target(),
        [tag] => release_target_from_tag(tag),
        _ => Err("release-version tag expects at most one <release-tag> argument.".to_owned()),
    }
}

fn prompt_release_target() -> XtaskResult<ReleaseTarget> {
    let workspace = Workspace::read(Path::new(ROOT_MANIFEST))?;
    let latest_tag = latest_release_tag()?;
    let (major, minor, patch, rc_number) = parse_release_version(&workspace.version)?;
    let minor_version = if rc_number.is_some() {
        format!("{major}.{minor}.{patch}")
    } else {
        format!("{major}.{}.0", minor + 1)
    };
    let patch_version = format!("{major}.{minor}.{}", patch + 1);
    let major_version = format!("{}.0.0", major + 1);
    let rc_version = next_minor_rc_version(&workspace.version, latest_tag.as_deref())?;

    println!(
        "Workspace version: {}",
        release_tag_from_version(&workspace.version)
    );
    if let Some(tag) = latest_tag.as_deref() {
        println!("Latest release tag: {tag}");
    } else {
        println!("No release tags found; deriving the RC from workspace version.");
    }
    println!(
        "  1) Minor  -> {}",
        release_tag_from_version(&minor_version)
    );
    println!(
        "  2) Patch  -> {}",
        release_tag_from_version(&patch_version)
    );
    println!(
        "  3) Major  -> {}",
        release_tag_from_version(&major_version)
    );
    println!("  4) Custom");
    println!("  5) Minor RC -> {}", release_tag_from_version(&rc_version));
    print!("Release type [1-5]: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("failed to flush release type prompt: {error}"))?;

    let mut choice = String::new();
    io::stdin()
        .read_line(&mut choice)
        .map_err(|error| format!("failed to read release type: {error}"))?;

    match choice.trim().to_ascii_lowercase().as_str() {
        "1" | "minor" => release_target_from_tag(&minor_version),
        "2" | "patch" => release_target_from_tag(&patch_version),
        "3" | "major" => release_target_from_tag(&major_version),
        "4" | "custom" => prompt_custom_release_target(),
        "5" | "rc" | "minor-rc" => release_target_from_tag(&rc_version),
        value => Err(format!(
            "unknown release type {value:?}; expected 1/minor, 2/patch, 3/major, 4/custom, or 5/minor-rc."
        )),
    }
}

fn latest_release_tag() -> XtaskResult<Option<String>> {
    let refs = git_output([
        "for-each-ref",
        "--sort=-refname",
        "--sort=-creatordate",
        "--format=%(refname:short)",
        "refs/tags",
    ])?;
    Ok(newest_release_tag_from_refs(&refs).map(str::to_owned))
}

/// Выбирает первый корректный релизный тег из списка, упорядоченного Git по дате.
fn newest_release_tag_from_refs(refs: &str) -> Option<&str> {
    refs.lines().find(|tag| {
        tag.strip_prefix('v')
            .is_some_and(|version| parse_release_version(version).is_ok())
    })
}

/// Предлагает следующий RC с приоритетом последнего RC-тега над версией workspace.
fn next_minor_rc_version(workspace_version: &str, latest_tag: Option<&str>) -> XtaskResult<String> {
    let (major, minor, patch, rc_number) = parse_release_version(workspace_version)?;
    if let Some(tag_version) = latest_tag.and_then(|tag| tag.strip_prefix('v'))
        && let Ok((tag_major, tag_minor, tag_patch, Some(number))) =
            parse_release_version(tag_version)
    {
        return Ok(format!(
            "{tag_major}.{tag_minor}.{tag_patch}-rc.{}",
            number + 1
        ));
    }

    Ok(match rc_number {
        Some(number) => format!("{major}.{minor}.{patch}-rc.{}", number + 1),
        None => format!("{major}.{}.0-rc.1", minor + 1),
    })
}

fn prompt_custom_release_target() -> XtaskResult<ReleaseTarget> {
    print!("New release tag (for example v0.13.0): ");
    io::stdout()
        .flush()
        .map_err(|error| format!("failed to flush release tag prompt: {error}"))?;

    let mut tag = String::new();
    io::stdin()
        .read_line(&mut tag)
        .map_err(|error| format!("failed to read release tag: {error}"))?;

    release_target_from_tag(&tag)
}

fn parse_release_version(version: &str) -> XtaskResult<(u64, u64, u64, Option<u64>)> {
    let (base_version, rc_number) = match version.split_once("-rc.") {
        Some((base, number)) => {
            let number = number.parse::<u64>().map_err(|error| {
                format!("invalid release candidate number in {version:?}: {error}")
            })?;
            if number == 0 {
                return Err(format!(
                    "release candidate number must be positive in {version:?}."
                ));
            }
            (base, Some(number))
        }
        None => (version, None),
    };
    let mut components = base_version.split('.');
    let major = parse_release_component("major", components.next(), version)?;
    let minor = parse_release_component("minor", components.next(), version)?;
    let patch = parse_release_component("patch", components.next(), version)?;

    if components.next().is_some() {
        return Err(format!(
            "workspace release version must use major.minor.patch with an optional -rc.N suffix, got {version:?}."
        ));
    }

    Ok((major, minor, patch, rc_number))
}

fn parse_release_component(name: &str, component: Option<&str>, version: &str) -> XtaskResult<u64> {
    let component = component.ok_or_else(|| {
        format!("workspace release version must use major.minor.patch format, got {version:?}.")
    })?;

    component.parse::<u64>().map_err(|error| {
        format!("invalid {name} component in release version {version:?}: {error}")
    })
}

/// Преобразует пользовательский тег в согласованные тег и версию workspace.
fn release_target_from_tag(input: &str) -> XtaskResult<ReleaseTarget> {
    let tag = input.trim();
    if tag.is_empty() {
        return Err("release tag cannot be empty.".to_owned());
    }
    if tag.chars().any(char::is_whitespace) {
        return Err(format!("release tag must not contain whitespace: {tag}"));
    }

    let version = tag
        .strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag);
    if version.is_empty() {
        return Err("release tag must contain a version after the v prefix.".to_owned());
    }
    if version.chars().any(|value| matches!(value, '/' | '\\')) {
        return Err(format!(
            "release version must not contain path separators: {version}"
        ));
    }

    Ok(ReleaseTarget {
        tag: release_tag_from_version(version),
        version: version.to_owned(),
    })
}

fn release_tag_from_version(version: &str) -> String {
    format!("v{version}")
}

#[cfg(test)]
mod tests;
