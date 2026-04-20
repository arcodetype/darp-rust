use std::collections::BTreeMap;
use std::path::PathBuf;

use darp::config::{Config, Environment};

fn config_with_domain(name: &str, location: &str) -> Config {
    let mut config = Config::default();
    config.add_domain(name, location).unwrap();
    config
}

// ---------------------------------------------------------------------------
// parse_bool
// ---------------------------------------------------------------------------

#[test]
fn parse_bool_truthy_values() {
    let c = Config::default();
    for s in &[
        "true", "TRUE", "True", "1", "yes", "YES", "y", "Y", "on", "ON",
    ] {
        assert!(c.parse_bool(s).unwrap(), "expected true for '{}'", s);
    }
}

#[test]
fn parse_bool_falsy_values() {
    let c = Config::default();
    for s in &[
        "false", "FALSE", "False", "0", "no", "NO", "n", "N", "off", "OFF",
    ] {
        assert!(!c.parse_bool(s).unwrap(), "expected false for '{}'", s);
    }
}

#[test]
fn parse_bool_trims_whitespace() {
    let c = Config::default();
    assert!(c.parse_bool("  true  ").unwrap());
    assert!(!c.parse_bool("\tfalse\n").unwrap());
}

#[test]
fn parse_bool_rejects_invalid() {
    let c = Config::default();
    assert!(c.parse_bool("maybe").is_err());
    assert!(c.parse_bool("").is_err());
    assert!(c.parse_bool("2").is_err());
}

// ---------------------------------------------------------------------------
// resolve_host_path
// ---------------------------------------------------------------------------

#[test]
fn resolve_host_path_pwd_token() {
    let c = Config::default();
    let current = PathBuf::from("/my/project");
    let domain = PathBuf::from("/my");
    let result = c
        .resolve_host_path("{pwd}/data", &current, &domain)
        .unwrap();
    assert_eq!(result, PathBuf::from("/my/project/data"));
}

#[test]
fn resolve_host_path_home_token() {
    let c = Config::default();
    let current = PathBuf::from("/any");
    let domain = PathBuf::from("/any");
    let result = c
        .resolve_host_path("{home}/.cache", &current, &domain)
        .unwrap();
    // Should contain the actual home dir, not the literal token
    let result_str = result.to_string_lossy();
    assert!(!result_str.contains("{home}"));
    assert!(result_str.ends_with("/.cache"));
}

#[test]
fn resolve_host_path_no_tokens() {
    let c = Config::default();
    let current = PathBuf::from("/any");
    let domain = PathBuf::from("/any");
    let result = c
        .resolve_host_path("/absolute/path", &current, &domain)
        .unwrap();
    assert_eq!(result, PathBuf::from("/absolute/path"));
}

#[test]
fn resolve_host_path_domain_token() {
    let c = Config::default();
    let current = PathBuf::from("/my/project/svc");
    let domain = PathBuf::from("/my/project");
    let result = c
        .resolve_host_path("{domain}/shared", &current, &domain)
        .unwrap();
    assert_eq!(result, PathBuf::from("/my/project/shared"));
}

// ---------------------------------------------------------------------------
// add/rm pre_config
// ---------------------------------------------------------------------------

#[test]
fn add_pre_config_and_duplicate_rejected() {
    let mut c = Config::default();
    c.add_pre_config("/path/to/config.json", None).unwrap();

    assert!(c.pre_config.as_ref().unwrap().len() == 1);
    assert!(c.add_pre_config("/path/to/config.json", None).is_err());
}

#[test]
fn add_pre_config_with_repo_location() {
    let mut c = Config::default();
    c.add_pre_config("/path/config.json", Some("/path/repo"))
        .unwrap();

    let entry = &c.pre_config.as_ref().unwrap()[0];
    assert_eq!(entry.location, "/path/config.json");
    assert_eq!(entry.repo_location.as_deref(), Some("/path/repo"));
}

#[test]
fn rm_pre_config_removes_entry() {
    let mut c = Config::default();
    c.add_pre_config("/a.json", None).unwrap();
    c.add_pre_config("/b.json", None).unwrap();

    c.rm_pre_config("/a.json").unwrap();
    assert_eq!(c.pre_config.as_ref().unwrap().len(), 1);
    assert_eq!(c.pre_config.as_ref().unwrap()[0].location, "/b.json");
}

#[test]
fn rm_pre_config_sets_none_when_last_removed() {
    let mut c = Config::default();
    c.add_pre_config("/only.json", None).unwrap();
    c.rm_pre_config("/only.json").unwrap();
    assert!(c.pre_config.is_none());
}

#[test]
fn rm_pre_config_errors_on_missing() {
    let mut c = Config::default();
    c.add_pre_config("/exists.json", None).unwrap();
    assert!(c.rm_pre_config("/nope.json").is_err());
}

#[test]
fn rm_pre_config_errors_when_empty() {
    let mut c = Config::default();
    assert!(c.rm_pre_config("/anything.json").is_err());
}

// ---------------------------------------------------------------------------
// Domain lifecycle: add, set properties, remove
// ---------------------------------------------------------------------------

#[test]
fn add_domain_duplicate_rejected() {
    let mut c = config_with_domain("proj", "/tmp/proj");
    assert!(c.add_domain("proj", "/tmp/other").is_err());
}

#[test]
fn rm_domain_success() {
    let mut c = config_with_domain("proj", "/tmp/proj");
    c.rm_domain("proj").unwrap();
    assert!(c.domains.as_ref().unwrap().is_empty());
}

#[test]
fn rm_domain_missing_errors() {
    let mut c = config_with_domain("proj", "/tmp/proj");
    assert!(c.rm_domain("nope").is_err());
}

#[test]
fn rm_domain_no_domains_errors() {
    let mut c = Config::default();
    assert!(c.rm_domain("anything").is_err());
}

// ---------------------------------------------------------------------------
// Domain-level set + rm round-trips
// ---------------------------------------------------------------------------

#[test]
fn set_and_rm_domain_serve_command() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_domain_serve_command("d", "npm start").unwrap();

    let dom = &c.domains.as_ref().unwrap()["d"];
    assert_eq!(dom.serve_command.as_deref(), Some("npm start"));

    c.rm_domain_serve_command("d").unwrap();
    let dom = &c.domains.as_ref().unwrap()["d"];
    assert!(dom.serve_command.is_none());
}

#[test]
fn rm_domain_serve_command_errors_when_unset() {
    let mut c = config_with_domain("d", "/tmp/d");
    assert!(c.rm_domain_serve_command("d").is_err());
}

#[test]
fn set_domain_default_environment_validates_env_exists() {
    let mut c = config_with_domain("d", "/tmp/d");
    // No environments configured — should fail
    assert!(c.set_domain_default_environment("d", "go").is_err());

    // Add an environment, then it should work
    let mut envs = BTreeMap::new();
    envs.insert("go".to_string(), Environment::default());
    c.environments = Some(envs);
    c.set_domain_default_environment("d", "go").unwrap();
    assert_eq!(
        c.domains.as_ref().unwrap()["d"]
            .default_environment
            .as_deref(),
        Some("go")
    );
}

// ---------------------------------------------------------------------------
// Domain portmap add/rm
// ---------------------------------------------------------------------------

#[test]
fn add_and_rm_domain_portmap() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_domain_portmap("d", "8080", "80").unwrap();

    let pm = c.domains.as_ref().unwrap()["d"]
        .host_portmappings
        .as_ref()
        .unwrap();
    assert_eq!(pm.get("8080").unwrap(), "80");

    // Duplicate rejected
    assert!(c.add_domain_portmap("d", "8080", "90").is_err());

    // Remove
    c.rm_domain_portmap("d", "8080").unwrap();
    let pm = c.domains.as_ref().unwrap()["d"]
        .host_portmappings
        .as_ref()
        .unwrap();
    assert!(pm.is_empty());
}

#[test]
fn rm_domain_portmap_errors_when_missing() {
    let mut c = config_with_domain("d", "/tmp/d");
    // No portmappings at all
    assert!(c.rm_domain_portmap("d", "8080").is_err());
}

// ---------------------------------------------------------------------------
// Domain variable add/rm
// ---------------------------------------------------------------------------

#[test]
fn add_and_rm_domain_variable() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_domain_variable("d", "DB_HOST", "localhost").unwrap();

    let vars = c.domains.as_ref().unwrap()["d"].variables.as_ref().unwrap();
    assert_eq!(vars.get("DB_HOST").unwrap(), "localhost");

    // Duplicate rejected
    assert!(c.add_domain_variable("d", "DB_HOST", "other").is_err());

    // Remove
    c.rm_domain_variable("d", "DB_HOST").unwrap();
    let vars = c.domains.as_ref().unwrap()["d"].variables.as_ref().unwrap();
    assert!(vars.is_empty());
}

// ---------------------------------------------------------------------------
// Domain volume add/rm
// ---------------------------------------------------------------------------

#[test]
fn add_and_rm_domain_volume() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_domain_volume("d", "/data", "/host/data").unwrap();

    let vols = c.domains.as_ref().unwrap()["d"].volumes.as_ref().unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].container, "/data");
    assert_eq!(vols[0].host, "/host/data");

    // Exact duplicate rejected
    assert!(c.add_domain_volume("d", "/data", "/host/data").is_err());

    // Different host is OK (same container)
    c.add_domain_volume("d", "/data", "/other/data").unwrap();
    assert_eq!(
        c.domains.as_ref().unwrap()["d"]
            .volumes
            .as_ref()
            .unwrap()
            .len(),
        2
    );

    // Remove specific volume
    c.rm_domain_volume("d", "/data", "/host/data").unwrap();
    let vols = c.domains.as_ref().unwrap()["d"].volumes.as_ref().unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].host, "/other/data");
}

#[test]
fn rm_domain_volume_errors_when_no_match() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_domain_volume("d", "/data", "/host/data").unwrap();
    assert!(c.rm_domain_volume("d", "/data", "/wrong/host").is_err());
}

// ---------------------------------------------------------------------------
// rm_group and rm_service
// ---------------------------------------------------------------------------

#[test]
fn rm_group_success() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_serve_command("d", "g", "cmd").unwrap(); // auto-creates group
    c.rm_group("d", "g").unwrap();

    let groups = c.domains.as_ref().unwrap()["d"].groups.as_ref().unwrap();
    assert!(!groups.contains_key("g"));
}

#[test]
fn rm_group_missing_errors() {
    let mut c = config_with_domain("d", "/tmp/d");
    // No groups at all
    assert!(c.rm_group("d", "g").is_err());
}

#[test]
fn rm_service_success() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_serve_command("d", "g", "svc", "cmd").unwrap(); // auto-creates group + service
    c.rm_service("d", "g", "svc").unwrap();

    let services = c.domains.as_ref().unwrap()["d"].groups.as_ref().unwrap()["g"]
        .services
        .as_ref()
        .unwrap();
    assert!(!services.contains_key("svc"));
}

#[test]
fn rm_service_missing_errors() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_serve_command("d", "g", "cmd").unwrap();
    // Group exists but no services
    assert!(c.rm_service("d", "g", "svc").is_err());
}

// ---------------------------------------------------------------------------
// load_merged with pre_config chain (temp files)
// ---------------------------------------------------------------------------

#[test]
fn load_merged_merges_pre_config() {
    let dir = std::env::temp_dir().join("darp_test_merge");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Parent config with an environment
    let parent_path = dir.join("parent.json");
    std::fs::write(
        &parent_path,
        r#"{"environments": {"go": {"serve_command": "air"}}}"#,
    )
    .unwrap();

    // Leaf config that references parent
    let leaf_path = dir.join("config.json");
    let leaf_content = format!(
        r#"{{"pre_config": [{{"location": "{}"}}], "engine": "docker"}}"#,
        parent_path.display()
    );
    std::fs::write(&leaf_path, leaf_content).unwrap();

    let config = Config::load_merged(&leaf_path).unwrap();

    // Leaf's engine is present
    assert_eq!(config.engine.as_deref(), Some("docker"));
    // Parent's environment is merged in
    let envs = config.environments.as_ref().unwrap();
    assert!(envs.contains_key("go"));
    assert_eq!(envs["go"].serve_command.as_deref(), Some("air"));
    // pre_config is stripped from merged result
    assert!(config.pre_config.is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_merged_detects_domain_conflicts() {
    let dir = std::env::temp_dir().join("darp_test_conflict");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let parent_a = dir.join("a.json");
    std::fs::write(&parent_a, r#"{"domains": {"proj": {"location": "/a"}}}"#).unwrap();

    let parent_b = dir.join("b.json");
    std::fs::write(&parent_b, r#"{"domains": {"proj": {"location": "/b"}}}"#).unwrap();

    let leaf_path = dir.join("config.json");
    let leaf = format!(
        r#"{{"pre_config": [{{"location": "{}"}}, {{"location": "{}"}}]}}"#,
        parent_a.display(),
        parent_b.display()
    );
    std::fs::write(&leaf_path, leaf).unwrap();

    let result = Config::load_merged(&leaf_path);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("proj"),
        "error should name the conflicting domain: {err}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_merged_leaf_overrides_parent() {
    let dir = std::env::temp_dir().join("darp_test_override");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let parent_path = dir.join("parent.json");
    std::fs::write(&parent_path, r#"{"engine": "podman"}"#).unwrap();

    let leaf_path = dir.join("config.json");
    let leaf = format!(
        r#"{{"pre_config": [{{"location": "{}"}}], "engine": "docker"}}"#,
        parent_path.display()
    );
    std::fs::write(&leaf_path, leaf).unwrap();

    let config = Config::load_merged(&leaf_path).unwrap();
    assert_eq!(config.engine.as_deref(), Some("docker")); // leaf wins

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Config save + load round-trip
// ---------------------------------------------------------------------------

#[test]
fn save_and_load_round_trip() {
    let dir = std::env::temp_dir().join("darp_test_roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");

    let mut config = Config::default();
    config.add_domain("myproject", "/tmp/myproject").unwrap();
    config.engine = Some("docker".to_string());
    config.save(&path).unwrap();

    let loaded = Config::load(&path).unwrap();
    assert_eq!(loaded.engine.as_deref(), Some("docker"));
    assert!(loaded.domains.as_ref().unwrap().contains_key("myproject"));

    // Verify no null values in serialized output
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(
        !raw.contains(": null"),
        "serialized config should not contain null values"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// `*field` override parsing and validation
// ---------------------------------------------------------------------------

#[test]
fn load_merged_parses_asterisk_keys() {
    let dir = std::env::temp_dir().join("darp_test_asterisk_parse");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("config.json");
    std::fs::write(
        &path,
        r#"{
            "domains": {
                "proj": {
                    "location": "/tmp/proj",
                    "*volumes": [{"host": "/a", "container": "/a"}],
                    "*serve_command": null
                }
            }
        }"#,
    )
    .unwrap();

    let cfg = Config::load_merged(&path).unwrap();
    let d = cfg.domains.as_ref().unwrap().get("proj").unwrap();

    // Asterisk volumes parsed into volumes_override = Some(Some(vec))
    let vol_over = d.volumes_override.as_ref().expect("volumes_override set");
    let vols = vol_over
        .as_ref()
        .expect("volumes_override is Some(Some(..))");
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].host, "/a");

    // Asterisk-null serve_command parsed into Some(None)
    let sc_over = d
        .serve_command_override
        .as_ref()
        .expect("serve_command_override set");
    assert!(
        sc_over.is_none(),
        "expected Some(None) for *serve_command: null"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_with_both_field_and_asterisk_field_errors() {
    let dir = std::env::temp_dir().join("darp_test_asterisk_double");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("config.json");
    std::fs::write(
        &path,
        r#"{
            "domains": {
                "proj": {
                    "location": "/tmp/proj",
                    "volumes": [{"host": "/a", "container": "/a"}],
                    "*volumes": [{"host": "/b", "container": "/b"}]
                }
            }
        }"#,
    )
    .unwrap();

    let result = Config::load_merged(&path);
    assert!(result.is_err(), "expected error on double declaration");
    let err = result.unwrap_err().to_string();
    assert!(err.contains("volumes"), "error should name field: {}", err);
    assert!(err.contains("proj"), "error should name location: {}", err);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_roundtrip_preserves_asterisk_fields() {
    let dir = std::env::temp_dir().join("darp_test_asterisk_roundtrip");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.json");

    // Write an initial config containing both forms of override.
    std::fs::write(
        &path,
        r#"{
            "domains": {
                "proj": {
                    "location": "/tmp/proj",
                    "*volumes": [{"host": "/x", "container": "/x"}],
                    "*serve_command": null
                }
            }
        }"#,
    )
    .unwrap();

    // Load → save → load, ensure both override forms survive.
    let loaded = Config::load_merged(&path).unwrap();
    let resave_path = dir.join("resaved.json");
    loaded.save(&resave_path).unwrap();

    let raw = std::fs::read_to_string(&resave_path).unwrap();
    assert!(
        raw.contains("\"*volumes\""),
        "*volumes key should be preserved: {}",
        raw
    );
    assert!(
        raw.contains("\"*serve_command\""),
        "*serve_command key should be preserved: {}",
        raw
    );
    assert!(
        raw.contains("\"*serve_command\": null"),
        "*serve_command null value should be preserved: {}",
        raw
    );

    let reloaded = Config::load_merged(&resave_path).unwrap();
    let d = reloaded.domains.as_ref().unwrap().get("proj").unwrap();
    assert!(d.volumes_override.is_some());
    assert!(d.serve_command_override.is_some());
    assert!(d.serve_command_override.as_ref().unwrap().is_none());

    let _ = std::fs::remove_dir_all(&dir);
}
