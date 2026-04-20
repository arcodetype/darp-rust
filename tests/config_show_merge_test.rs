//! End-to-end tests for `darp config show` hierarchical merging.
//!
//! Exercises the full pipeline: on-disk JSON → `Config::load_merged` →
//! `service_context_from_cwd` → `ResolvedSettings::resolve` → pretty-printed JSON.

use std::path::PathBuf;
use std::process::Command;

fn darp_bin() -> PathBuf {
    let output = Command::new("cargo")
        .args(["build", "--quiet"])
        .output()
        .expect("failed to build darp");
    assert!(
        output.status.success(),
        "cargo build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("target/debug/darp")
}

fn run_show(bin: &PathBuf, darp_root: &std::path::Path, cwd: &std::path::Path) -> String {
    let output = Command::new(bin)
        .env("DARP_ROOT", darp_root)
        .current_dir(cwd)
        .args(["config", "show"])
        .output()
        .expect("failed to run darp config show");
    assert!(
        output.status.success(),
        "darp config show failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// Canonicalize so macOS `/tmp` → `/private/tmp` matches domain lookups.
fn canonical(p: &std::path::Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap()
}

#[test]
fn config_show_merges_volumes_across_domain_group_service() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let root_path = canonical(root.path());

    // Filesystem layout: <root>/proj/mygrp/mysvc/
    let domain_dir = root_path.join("proj");
    let service_dir = domain_dir.join("mygrp").join("mysvc");
    std::fs::create_dir_all(&service_dir).unwrap();

    let config_json = format!(
        r#"{{
            "domains": {{
                "myproj": {{
                    "location": "{loc}",
                    "volumes": [{{"host": "/dom", "container": "/dom"}}],
                    "groups": {{
                        "mygrp": {{
                            "volumes": [{{"host": "/grp", "container": "/grp"}}],
                            "services": {{
                                "mysvc": {{
                                    "volumes": [{{"host": "/svc", "container": "/svc"}}]
                                }}
                            }}
                        }}
                    }}
                }}
            }}
        }}"#,
        loc = domain_dir.display()
    );
    std::fs::write(root_path.join("config.json"), config_json).unwrap();

    let stdout = run_show(&bin, &root_path, &service_dir);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");

    let vols = parsed["volumes"]
        .as_array()
        .expect("volumes should be an array");
    assert_eq!(vols.len(), 3, "expected 3 merged volumes, got: {}", stdout);

    // Walk order: env → domain → group → service
    assert_eq!(vols[0]["host"], "/dom");
    assert_eq!(vols[1]["host"], "/grp");
    assert_eq!(vols[2]["host"], "/svc");
}

#[test]
fn config_show_asterisk_volumes_at_service_discards_parents() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let root_path = canonical(root.path());

    let domain_dir = root_path.join("proj");
    let service_dir = domain_dir.join("mygrp").join("mysvc");
    std::fs::create_dir_all(&service_dir).unwrap();

    let config_json = format!(
        r#"{{
            "domains": {{
                "myproj": {{
                    "location": "{loc}",
                    "volumes": [{{"host": "/dom", "container": "/dom"}}],
                    "groups": {{
                        "mygrp": {{
                            "volumes": [{{"host": "/grp", "container": "/grp"}}],
                            "services": {{
                                "mysvc": {{
                                    "*volumes": [{{"host": "/svc-only", "container": "/svc-only"}}]
                                }}
                            }}
                        }}
                    }}
                }}
            }}
        }}"#,
        loc = domain_dir.display()
    );
    std::fs::write(root_path.join("config.json"), config_json).unwrap();

    let stdout = run_show(&bin, &root_path, &service_dir);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");

    let vols = parsed["volumes"]
        .as_array()
        .expect("volumes should be an array");
    assert_eq!(
        vols.len(),
        1,
        "expected only service's 1 volume, got: {}",
        stdout
    );
    assert_eq!(vols[0]["host"], "/svc-only");
}

#[test]
fn config_show_merges_variables_with_service_winning_conflicts() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();
    let root_path = canonical(root.path());

    let domain_dir = root_path.join("proj");
    let service_dir = domain_dir.join("mygrp").join("mysvc");
    std::fs::create_dir_all(&service_dir).unwrap();

    let config_json = format!(
        r#"{{
            "domains": {{
                "myproj": {{
                    "location": "{loc}",
                    "variables": {{"PORT": "8000", "DOM": "1"}},
                    "groups": {{
                        "mygrp": {{
                            "variables": {{"PORT": "9000", "GRP": "1"}},
                            "services": {{
                                "mysvc": {{
                                    "variables": {{"PORT": "9100", "SVC": "1"}}
                                }}
                            }}
                        }}
                    }}
                }}
            }}
        }}"#,
        loc = domain_dir.display()
    );
    std::fs::write(root_path.join("config.json"), config_json).unwrap();

    let stdout = run_show(&bin, &root_path, &service_dir);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("stdout is valid JSON");

    let vars = parsed["variables"]
        .as_object()
        .expect("variables should be an object");
    assert_eq!(
        vars["PORT"], "9100",
        "service should win conflict: {}",
        stdout
    );
    assert_eq!(vars["DOM"], "1");
    assert_eq!(vars["GRP"], "1");
    assert_eq!(vars["SVC"], "1");
    assert_eq!(vars.len(), 4);
}
