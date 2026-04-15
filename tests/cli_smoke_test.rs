//! CLI integration tests that require a running container engine (Docker or Podman).
//!
//! The engine is selected via the `DARP_TEST_ENGINE` env var (defaults to "docker").
//! Run locally:
//!   cargo test --test cli_smoke_test -- --ignored --test-threads=1
//! With Podman:
//!   DARP_TEST_ENGINE=podman cargo test --test cli_smoke_test -- --ignored --test-threads=1

use std::path::PathBuf;
use std::process::Command;

/// Build darp in debug mode and return the binary path.
fn darp_bin() -> PathBuf {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .output()
        .expect("failed to build darp");
    assert!(output.status.success(), "cargo build failed");

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("target/debug/darp")
}

/// Which engine to test — reads DARP_TEST_ENGINE, defaults to "docker".
fn test_engine() -> String {
    std::env::var("DARP_TEST_ENGINE").unwrap_or_else(|_| "docker".to_string())
}

/// Run darp with the given args, using a custom DARP_ROOT.
fn run_darp(bin: &PathBuf, darp_root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(bin)
        .env("DARP_ROOT", darp_root)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run darp {:?}: {}", args, e))
}

/// Run darp with a custom cwd.
fn run_darp_in(
    bin: &PathBuf,
    darp_root: &std::path::Path,
    cwd: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(bin)
        .env("DARP_ROOT", darp_root)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run darp {:?}: {}", args, e))
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Helper: configure the engine in a fresh DARP_ROOT.
fn setup_engine(bin: &PathBuf, root: &std::path::Path) {
    let engine = test_engine();
    let output = run_darp(bin, root, &["config", "set", "engine", &engine]);
    assert!(
        output.status.success(),
        "set engine to {}: {}",
        engine,
        stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// Tests — marked #[ignore] so `cargo test` skips them by default.
// These require a running container engine.
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn smoke_help() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let output = run_darp(&bin, root.path(), &["--help"]);
    assert!(output.status.success());
    assert!(stdout(&output).contains("auto-reverse proxied"));
}

#[test]
#[ignore]
fn smoke_config_set_engine() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let engine = test_engine();

    let output = run_darp(&bin, root.path(), &["config", "set", "engine", &engine]);
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(stdout(&output).contains("Engine set"));

    let config_path = root.path().join("config.json");
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains(&format!("\"engine\": \"{}\"", engine)),
        "config should contain engine: {}",
        content
    );
}

#[test]
#[ignore]
fn smoke_deploy_and_urls() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    setup_engine(&bin, root.path());

    // Create a domain directory with a service subdirectory
    let domain_dir = root.path().join("projects");
    std::fs::create_dir_all(domain_dir.join("hello-world")).unwrap();

    // Add domain via -l
    let output = run_darp(
        &bin,
        root.path(),
        &[
            "config",
            "add",
            "dom",
            "portmap",
            "projects",
            "8080",
            "80",
            "-l",
            &domain_dir.to_string_lossy(),
        ],
    );
    assert!(
        output.status.success(),
        "add domain portmap: {}{}",
        stdout(&output),
        stderr(&output)
    );

    // Deploy
    let output = run_darp(&bin, root.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "deploy: {}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(stdout(&output).contains("Deploying"));

    // Verify portmap.json
    let portmap_content = std::fs::read_to_string(root.path().join("portmap.json")).unwrap();
    assert!(
        portmap_content.contains("hello-world"),
        "portmap should contain hello-world"
    );

    // Verify vhost_container.conf
    let vhost_content = std::fs::read_to_string(root.path().join("vhost_container.conf")).unwrap();
    assert!(vhost_content.contains("hello-world.projects.test"));

    // URLs
    let output = run_darp(&bin, root.path(), &["urls"]);
    assert!(output.status.success(), "urls: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("hello-world"), "urls: {}", out);
    assert!(out.contains("projects"), "urls: {}", out);

    // Cleanup
    let _ = run_darp(&bin, root.path(), &["uninstall"]);
}

#[test]
#[ignore]
fn smoke_doctor() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    setup_engine(&bin, root.path());

    let output = run_darp(&bin, root.path(), &["doctor"]);
    assert!(output.status.success(), "doctor: {}", stderr(&output));
    let out = stdout(&output);
    assert!(out.contains("Darp Doctor"));
    assert!(out.contains("Container engine"));
}

#[test]
#[ignore]
fn smoke_check_image_alpine() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    setup_engine(&bin, root.path());

    let output = run_darp(&bin, root.path(), &["check-image", "alpine"]);
    assert!(
        output.status.success(),
        "check-image: {}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(out.contains("Darp Image Check"));
    assert!(out.contains("sh is available"));
}

#[test]
#[ignore]
fn smoke_shell_dry_run() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let engine = test_engine();
    setup_engine(&bin, root.path());

    let domain_dir = root.path().join("projects");
    let service_dir = domain_dir.join("myapp");
    std::fs::create_dir_all(&service_dir).unwrap();

    run_darp(
        &bin,
        root.path(),
        &[
            "config",
            "set",
            "dom",
            "default-container-image",
            "projects",
            "alpine",
            "-l",
            &domain_dir.to_string_lossy(),
        ],
    );

    // Deploy to generate portmap
    let output = run_darp(&bin, root.path(), &["deploy"]);
    assert!(output.status.success(), "deploy: {}", stderr(&output));

    // Shell --dry-run from service directory
    let output = run_darp_in(&bin, root.path(), &service_dir, &["shell", "--dry-run"]);
    assert!(
        output.status.success(),
        "shell dry-run: {}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains(&engine) || out.contains("run"),
        "dry-run should contain engine command: {}",
        out
    );

    // Cleanup
    let _ = run_darp(&bin, root.path(), &["uninstall"]);
}
