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
            println!("Selected release tag: {}", release.tag);
            if git_output(["tag", "--list"])?
                .lines()
                .any(|tag| tag == release.tag)
            {
                return Err(format!(
                    "release tag {} already exists; choose a new version",
                    release.tag
                ));
            }
            ensure_clean_worktree("before updating the release version")?;
            crate::release_notes::ReleaseNotes::read(&release.tag)?;
            println!("Release notes ready: {}", release.tag);
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
    let refs = git_output([
        "for-each-ref",
        "--sort=-refname",
        "--sort=-creatordate",
        "--format=%(refname:short)",
        "refs/tags",
    ])?;
    let latest_tag = newest_release_tag_from_refs(&refs);
    let suggestions = release_suggestions(&workspace.version, &refs)?;

    println!(
        "Workspace version: {}",
        release_tag_from_version(&workspace.version)
    );
    if let Some(tag) = latest_tag {
        println!("Latest release tag: {tag}");
    } else {
        println!("No release tags found; deriving the RC from workspace version.");
    }
    println!(
        "  1) Minor  -> {}",
        release_tag_from_version(&suggestions.minor)
    );
    println!(
        "  2) Patch  -> {}",
        release_tag_from_version(&suggestions.patch)
    );
    println!(
        "  3) Major  -> {}",
        release_tag_from_version(&suggestions.major)
    );
    println!("  4) Custom");
    println!(
        "  5) Minor RC -> {}",
        release_tag_from_version(&suggestions.rc)
    );
    print!("Release type [1-5]: ");
    io::stdout()
        .flush()
        .map_err(|error| format!("failed to flush release type prompt: {error}"))?;

    let mut choice = String::new();
    io::stdin()
        .read_line(&mut choice)
        .map_err(|error| format!("failed to read release type: {error}"))?;

    match choice.trim().to_ascii_lowercase().as_str() {
        "1" | "minor" => release_target_from_tag(&suggestions.minor),
        "2" | "patch" => release_target_from_tag(&suggestions.patch),
        "3" | "major" => release_target_from_tag(&suggestions.major),
        "4" | "custom" => prompt_custom_release_target(),
        "5" | "rc" | "minor-rc" => release_target_from_tag(&suggestions.rc),
        value => Err(format!(
            "unknown release type {value:?}; expected 1/minor, 2/patch, 3/major, 4/custom, or 5/minor-rc."
        )),
    }
}

/// Выбирает первый корректный релизный тег из списка, упорядоченного Git по дате.
fn newest_release_tag_from_refs(refs: &str) -> Option<&str> {
    refs.lines().find(|tag| {
        tag.strip_prefix('v')
            .is_some_and(|version| parse_release_version(version).is_ok())
    })
}

/// Согласованные варианты меню с учётом выпущенных стабильных версий.
struct ReleaseSuggestions {
    minor: String,
    patch: String,
    major: String,
    rc: String,
}

/// Продолжает только RC-серию, стабильная версия которой ещё не выпущена.
///
/// Числовой порядок версий важнее даты создания тегов; устаревший workspace
/// не должен предлагать уже выпущенный релиз или возобновлять закрытую RC-серию.
fn release_suggestions(workspace_version: &str, refs: &str) -> XtaskResult<ReleaseSuggestions> {
    let workspace = parse_release_version(workspace_version)?;
    let versions: Vec<_> = refs
        .lines()
        .filter_map(|tag| tag.strip_prefix('v'))
        .filter_map(|version| parse_release_version(version).ok())
        .collect();
    let stable = versions
        .iter()
        .filter(|version| version.3.is_none())
        .map(|&(major, minor, patch, _)| (major, minor, patch))
        .max();
    let workspace_base = (workspace.0, workspace.1, workspace.2);
    let baseline = stable.unwrap_or(workspace_base).max(workspace_base);
    let active_rc = versions
        .into_iter()
        .chain(std::iter::once(workspace))
        .filter(|&(major, minor, patch, rc)| {
            let base = (major, minor, patch);
            rc.is_some() && base >= baseline && stable.is_none_or(|stable| base > stable)
        })
        .max();
    let minor = match active_rc {
        Some((major, minor, patch, _)) => format!("{major}.{minor}.{patch}"),
        None => format!("{}.{}.0", baseline.0, baseline.1 + 1),
    };
    let rc_number = active_rc
        .and_then(|version| version.3)
        .map_or(1, |number| number + 1);

    Ok(ReleaseSuggestions {
        rc: format!("{minor}-rc.{rc_number}"),
        minor,
        patch: format!("{}.{}.{}", baseline.0, baseline.1, baseline.2 + 1),
        major: format!("{}.0.0", baseline.0 + 1),
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

/// Разбирает версию выпуска и необязательный положительный номер RC.
pub(super) fn parse_release_version(version: &str) -> XtaskResult<(u64, u64, u64, Option<u64>)> {
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
