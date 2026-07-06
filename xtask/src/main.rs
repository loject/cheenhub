#![warn(missing_docs)]
//! Вспомогательные команды разработки и релизов CheenHub.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

type XtaskResult<T> = Result<T, String>;

const ROOT_MANIFEST: &str = "Cargo.toml";

mod release_artifacts;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> XtaskResult<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return print_usage();
    };

    match command.as_str() {
        "line-stats" => run_line_stats(),
        "release-artifacts" => release_artifacts::run(args.collect()),
        "release-version" => run_release_version(args.collect()),
        "-h" | "--help" | "help" => print_usage(),
        unknown => Err(format!("unknown xtask command: {unknown}")),
    }
}

fn print_usage() -> XtaskResult<()> {
    println!(
        "Usage:\n  cargo run -p xtask -- line-stats\n  cargo run -p xtask -- release-version check\n  cargo run -p xtask -- release-version print-tag\n  cargo run -p xtask -- release-version tag [<release-tag>]\n  cargo run -p xtask -- release-artifacts collect <windows|linux|android> <source-dir> <output-dir> <release-tag>\n  cargo run -p xtask -- release-artifacts publish <release-tag> <asset-dir>"
    );
    Ok(())
}

fn run_line_stats() -> XtaskResult<()> {
    let repo_root = git_output(["rev-parse", "--show-toplevel"])?;
    env::set_current_dir(repo_root.trim())
        .map_err(|error| format!("failed to enter repository root: {error}"))?;

    let tracked_files = git_output(["ls-files"])?;
    let mut totals = BTreeMap::new();

    for file in tracked_files.lines() {
        let path = Path::new(file);
        let Some(kind) = line_stat_kind(path) else {
            continue;
        };
        let contents = read_file(path)?;
        *totals.entry(kind).or_insert(0) += contents.lines().count();
    }

    let mut rows: Vec<_> = totals.into_iter().collect();
    let total_lines: usize = rows.iter().map(|(_, lines)| *lines).sum();
    rows.sort_by(|(left_kind, left_lines), (right_kind, right_lines)| {
        right_lines
            .cmp(left_lines)
            .then_with(|| left_kind.cmp(right_kind))
    });

    for (kind, lines) in rows {
        println!("{kind:<12} {lines}");
    }
    println!("{:<12} {total_lines}", "total");

    Ok(())
}

fn line_stat_kind(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_string_lossy();
    if file_name.eq_ignore_ascii_case("Dockerfile") {
        return Some("dockerfile");
    }
    if file_name == ".dockerignore" || file_name == ".gitignore" {
        return Some("gitignore");
    }

    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "rs" => Some("rust"),
        "toml" => Some("toml"),
        "js" => Some("javascript"),
        "css" => Some("css"),
        "html" => Some("html"),
        "md" => Some("markdown"),
        "json" => Some("json"),
        "yml" | "yaml" => Some("yaml"),
        "sh" => Some("sh"),
        "svg" => Some("svg"),
        "webmanifest" => Some("webmanifest"),
        "lock" => Some("lockfile"),
        "conf" => Some("config"),
        "example" => Some("example"),
        _ => None,
    }
}

fn run_release_version(args: Vec<String>) -> XtaskResult<()> {
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
            update_workspace_version(Path::new(ROOT_MANIFEST), &release.version)?;
            let workspace = Workspace::read(Path::new(ROOT_MANIFEST))?;
            workspace.check_release_version()?;
            run_release_build()?;
            commit_release_changes(&release.tag)?;
            ensure_clean_worktree("before creating the git tag")?;
            create_git_tag(&release.tag)?;
            println!("Created git tag {}.", release.tag);
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

struct Workspace {
    version: String,
    members: Vec<PathBuf>,
}

impl Workspace {
    fn read(path: &Path) -> XtaskResult<Self> {
        let content = read_file(path)?;
        let version = read_key_in_section(&content, "workspace.package", "version")?
            .ok_or_else(|| "Cargo.toml [workspace.package] must define version.".to_owned())?;
        let members = read_workspace_members(&content)?;

        Ok(Self { version, members })
    }

    fn check_release_version(&self) -> XtaskResult<()> {
        for member in &self.members {
            let manifest_path = member.join(ROOT_MANIFEST);
            let content = read_file(&manifest_path)?;
            let package_section = read_section(&content, "package")
                .ok_or_else(|| format!("{} must define [package].", manifest_path.display()))?;

            if section_has_literal_key(package_section, "version") {
                return Err(format!(
                    "{} must inherit version from workspace.package.",
                    manifest_path.display()
                ));
            }

            if !section_has_workspace_key(package_section, "version") {
                return Err(format!(
                    "{} must contain version.workspace = true.",
                    manifest_path.display()
                ));
            }

            let package_version = cargo_package_version(&manifest_path)?;
            if package_version != self.version {
                return Err(format!(
                    "{} resolves to version {}, expected {}.",
                    manifest_path.display(),
                    package_version,
                    self.version
                ));
            }
        }

        Ok(())
    }
}

fn read_file(path: &Path) -> XtaskResult<String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn update_workspace_version(path: &Path, version: &str) -> XtaskResult<()> {
    let content = read_file(path)?;
    let updated = replace_workspace_version(&content, version)?;
    if updated == content {
        println!("Cargo.toml already uses release version {version}.");
        return Ok(());
    }

    fs::write(path, updated)
        .map_err(|error| format!("failed to update {}: {error}", path.display()))?;
    println!("Updated Cargo.toml workspace version to {version}.");
    Ok(())
}

fn replace_workspace_version(content: &str, version: &str) -> XtaskResult<String> {
    let mut updated = String::with_capacity(content.len());
    let mut in_workspace_package = false;
    let mut replaced = false;

    for line in content.split_inclusive('\n') {
        let newline = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let body = line.trim_end_matches(['\r', '\n']);
        let stripped = strip_comment(body).trim();

        if is_section_header(stripped) {
            in_workspace_package = stripped == "[workspace.package]";
        }

        if in_workspace_package
            && stripped
                .split_once('=')
                .is_some_and(|(name, _)| name.trim() == "version")
        {
            let indent: String = body
                .chars()
                .take_while(|value| value.is_whitespace())
                .collect();
            updated.push_str(&indent);
            updated.push_str("version = \"");
            updated.push_str(version);
            updated.push('"');
            updated.push_str(newline);
            replaced = true;
            continue;
        }

        updated.push_str(line);
    }

    if !replaced {
        return Err("Cargo.toml [workspace.package] must define version.".to_owned());
    }

    Ok(updated)
}

fn read_section<'a>(content: &'a str, section: &str) -> Option<&'a str> {
    let mut in_section = false;
    let mut section_start = 0;
    let header = format!("[{section}]");
    let mut offset = 0;

    for line in content.split_inclusive('\n') {
        let trimmed = line.trim();
        if is_section_header(trimmed) {
            if in_section {
                return Some(&content[section_start..offset]);
            }
            in_section = trimmed == header;
            if in_section {
                section_start = offset + line.len();
            }
        }
        offset += line.len();
    }

    if in_section {
        Some(&content[section_start..])
    } else {
        None
    }
}

fn read_key_in_section(content: &str, section: &str, key: &str) -> XtaskResult<Option<String>> {
    let Some(section_body) = read_section(content, section) else {
        return Ok(None);
    };

    for line in section_body.lines() {
        let stripped = strip_comment(line).trim();
        let Some((name, value)) = stripped.split_once('=') else {
            continue;
        };
        if name.trim() == key {
            return parse_quoted_string(value.trim())
                .map(Some)
                .map_err(|error| format!("{section}.{key} must be a string: {error}"));
        }
    }

    Ok(None)
}

fn read_workspace_members(content: &str) -> XtaskResult<Vec<PathBuf>> {
    let Some(workspace) = read_section(content, "workspace") else {
        return Err("Cargo.toml must define [workspace].".to_owned());
    };

    let mut members = Vec::new();
    let mut in_members = false;
    for line in workspace.lines() {
        let stripped = strip_comment(line).trim();
        if stripped.starts_with("members") && stripped.contains('[') {
            in_members = true;
        }
        if in_members {
            members.extend(read_quoted_strings(stripped).into_iter().map(PathBuf::from));
            if stripped.contains(']') {
                break;
            }
        }
    }

    if members.is_empty() {
        return Err("Cargo.toml [workspace] must define members.".to_owned());
    }

    Ok(members)
}

fn read_quoted_strings(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find('"') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('"') else {
            break;
        };
        values.push(rest[..end].to_owned());
        rest = &rest[end + 1..];
    }
    values
}

fn section_has_literal_key(section: &str, key: &str) -> bool {
    section.lines().any(|line| {
        let stripped = strip_comment(line).trim();
        stripped
            .split_once('=')
            .is_some_and(|(name, _)| name.trim() == key)
    })
}

fn section_has_workspace_key(section: &str, key: &str) -> bool {
    let expected = format!("{key}.workspace");
    section.lines().any(|line| {
        let stripped = strip_comment(line).trim();
        stripped.split_once('=').is_some_and(|(name, value)| {
            name.trim() == expected && value.trim().eq_ignore_ascii_case("true")
        })
    })
}

fn cargo_package_version(manifest_path: &Path) -> XtaskResult<String> {
    let output = checked_command(
        Command::new("cargo")
            .arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--no-deps")
            .arg("--manifest-path")
            .arg(manifest_path),
    )?;
    read_first_json_string_field(&output, "version").ok_or_else(|| {
        format!(
            "failed to read package version from cargo metadata for {}.",
            manifest_path.display()
        )
    })
}

fn read_first_json_string_field(content: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":\"");
    let start = content.find(&needle)? + needle.len();
    let rest = &content[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn create_git_tag(tag: &str) -> XtaskResult<()> {
    let existing = Command::new("git")
        .args(["rev-parse", "-q", "--verify"])
        .arg(format!("refs/tags/{tag}"))
        .output()
        .map_err(|error| format!("failed to check git tag {tag}: {error}"))?;
    if existing.status.success() {
        return Err(format!("Git tag {tag} already exists."));
    }

    checked_command(Command::new("git").arg("tag").arg(tag))?;
    Ok(())
}

fn run_release_build() -> XtaskResult<()> {
    println!("Running cargo build before creating the git tag.");
    checked_status(Command::new("cargo").arg("build"))?;
    println!("cargo build finished.");
    Ok(())
}

fn ensure_clean_worktree(stage: &str) -> XtaskResult<()> {
    let status = git_status()?;
    if status.trim().is_empty() {
        return Ok(());
    }

    Err(format!(
        "repository has uncommitted changes {stage}:\n{}",
        status.trim_end()
    ))
}

fn commit_release_changes(tag: &str) -> XtaskResult<()> {
    let status = git_status()?;
    if status.trim().is_empty() {
        println!("release preparation did not create repository changes.");
        return Ok(());
    }

    println!("Committing release preparation changes before creating the git tag.");
    checked_command(Command::new("git").args(["add", "-A"]))?;
    checked_command(
        Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(format!("chore: prepare release {tag}")),
    )?;
    println!("Committed release preparation changes for {tag}.");
    Ok(())
}

fn git_status() -> XtaskResult<String> {
    git_output(["status", "--porcelain"])
}

fn git_output<const N: usize>(args: [&str; N]) -> XtaskResult<String> {
    checked_command(Command::new("git").args(args))
}

fn checked_status(command: &mut Command) -> XtaskResult<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if !status.success() {
        return Err(format!("{program} failed with status {status}."));
    }

    Ok(())
}

fn checked_command(command: &mut Command) -> XtaskResult<String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_quoted_string(value: &str) -> XtaskResult<String> {
    let value = value.trim_end_matches(',').trim();
    if !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("expected quoted string, got {value}"));
    }

    Ok(value.trim_matches('"').to_owned())
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
}

fn is_section_header(line: &str) -> bool {
    line.starts_with('[') && line.ends_with(']')
}

#[cfg(test)]
mod tests {
    use super::{release_target_from_tag, replace_workspace_version};

    #[test]
    fn normalizes_release_tag_to_workspace_version() {
        let release = release_target_from_tag("v0.13.0").expect("valid release tag");

        assert_eq!(release.tag, "v0.13.0");
        assert_eq!(release.version, "0.13.0");
    }

    #[test]
    fn accepts_release_version_without_prefix() {
        let release = release_target_from_tag("0.13.0").expect("valid release version");

        assert_eq!(release.tag, "v0.13.0");
        assert_eq!(release.version, "0.13.0");
    }

    #[test]
    fn replaces_workspace_package_version_only() {
        let content = "[workspace]\nmembers = [\"xtask\"]\n\n[workspace.package]\nversion = \"0.12.0\"\nedition = \"2024\"\n\n[package]\nversion = \"ignored\"\n";

        let updated = replace_workspace_version(content, "0.13.0").expect("updated manifest");

        assert!(updated.contains("[workspace.package]\nversion = \"0.13.0\"\nedition = \"2024\""));
        assert!(updated.contains("[package]\nversion = \"ignored\""));
    }
}
