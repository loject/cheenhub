//! Определение контейнерной среды независимо от наличия Docker socket на хосте.

/// Определяет контейнер по маркерам Docker/Podman и cgroup текущего процесса/PID 1.
///
/// Обычная установка Docker на хосте не переключает backend на Docker-сборщик.
/// Ошибка чтения необязательных cgroup-файлов не означает контейнерную среду.
pub fn running_in_container() -> bool {
    let marker = ["/.dockerenv", "/run/.containerenv"]
        .iter()
        .any(|path| std::path::Path::new(path).exists());
    let cgroups = ["/proc/self/cgroup", "/proc/1/cgroup"]
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n");
    container_evidence(marker, &cgroups)
}

fn container_evidence(marker: bool, cgroups: &str) -> bool {
    marker
        || cgroups.lines().any(|line| {
            let path = line.splitn(3, ':').nth(2).unwrap_or_default();
            path.split('/').any(|part| {
                part == "docker"
                    || part.starts_with("docker-")
                    || part.starts_with("kubepods")
                    || part.starts_with("libpod-")
            })
        })
}

#[cfg(test)]
mod tests;
