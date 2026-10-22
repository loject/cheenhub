use super::container_evidence;

#[test]
fn detects_container_markers_when_cgroup_v2_hides_container_path() {
    assert!(container_evidence(true, "0::/\n"));
}

#[test]
fn detects_container_cgroup_paths() {
    for path in [
        "1:cpu:/docker/abc",
        "0::/system.slice/docker-abc.scope",
        "0::/kubepods.slice/kubepods-burstable.slice",
        "0::/machine.slice/libpod-abc.scope",
    ] {
        assert!(container_evidence(false, path), "{path}");
    }
}

#[test]
fn ordinary_host_services_do_not_enable_docker_collection() {
    for path in [
        "",
        "0::/",
        "0::/system.slice/cheenhub.service",
        "0::/system.slice/docker.service",
    ] {
        assert!(!container_evidence(false, path), "{path}");
    }
}
