use darp::commands::build_container_hosts;
use darp::engine::{EngineKind, read_container_host_ip, write_container_host_ip};

// ---------------------------------------------------------------------------
// build_container_hosts — pure function, in-container /etc/hosts content
// ---------------------------------------------------------------------------

#[test]
fn build_container_hosts_includes_loopback_and_gateway() {
    let out = build_container_hosts(
        "172.17.0.1",
        "host.docker.internal",
        &["0.0.0.0\tapp.projects.test\n".to_string()],
    );
    assert!(out.contains("127.0.0.1\tlocalhost"));
    assert!(out.contains("::1\tlocalhost"));
    assert!(out.contains("172.17.0.1\thost.docker.internal"));
    assert!(out.contains("0.0.0.0\tapp.projects.test"));
}

#[test]
fn build_container_hosts_empty_url_list() {
    let out = build_container_hosts("172.17.0.1", "host.docker.internal", &[]);
    assert!(out.contains("host.docker.internal"));
    assert!(!out.contains("projects.test"));
}

#[test]
fn build_container_hosts_podman_gateway_name() {
    let out = build_container_hosts("10.88.0.1", "host.containers.internal", &[]);
    assert!(out.contains("10.88.0.1\thost.containers.internal"));
    assert!(!out.contains("host.docker.internal"));
}

// ---------------------------------------------------------------------------
// read_container_host_ip / write_container_host_ip — engine-tagged cache
// ---------------------------------------------------------------------------

#[test]
fn cache_roundtrip_matching_engine() {
    let path = std::env::temp_dir().join("darp_cache_test_roundtrip");
    let _ = std::fs::remove_file(&path);

    write_container_host_ip(&path, &EngineKind::Docker, "172.17.0.1").unwrap();
    let read = read_container_host_ip(&path, &EngineKind::Docker);
    assert_eq!(read.as_deref(), Some("172.17.0.1"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn cache_returns_none_on_engine_mismatch() {
    let path = std::env::temp_dir().join("darp_cache_test_mismatch");
    let _ = std::fs::remove_file(&path);

    write_container_host_ip(&path, &EngineKind::Docker, "172.17.0.1").unwrap();
    let read = read_container_host_ip(&path, &EngineKind::Podman);
    assert!(read.is_none());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn cache_returns_none_when_missing() {
    let path = std::env::temp_dir().join("darp_cache_test_missing_xyzzy");
    let _ = std::fs::remove_file(&path);
    assert!(read_container_host_ip(&path, &EngineKind::Docker).is_none());
}

#[test]
fn cache_returns_none_on_malformed_file() {
    let path = std::env::temp_dir().join("darp_cache_test_malformed");
    std::fs::write(&path, "just one line\n").unwrap();
    assert!(read_container_host_ip(&path, &EngineKind::Docker).is_none());
    let _ = std::fs::remove_file(&path);
}
