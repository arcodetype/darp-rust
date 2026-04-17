// Tests that call service_context_from_cwd() change the process cwd, which
// is global state. Run this test file with --test-threads=1 to avoid races
// with other tests (the integration.yml workflow does this automatically).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use darp::config::{Config, DarpPaths, Environment, Group, Service, read_json, resolve_location};

/// Mutex to serialize tests that change cwd.
static CWD_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// DarpPaths::from_env
// ---------------------------------------------------------------------------

#[test]
fn darp_paths_from_env_uses_darp_root() {
    let dir = std::env::temp_dir().join("darp_test_paths");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Set DARP_ROOT env var for this test
    unsafe {
        std::env::set_var("DARP_ROOT", &dir);
    }
    let paths = DarpPaths::from_env().unwrap();
    unsafe {
        std::env::remove_var("DARP_ROOT");
    }

    assert_eq!(paths._darp_root, dir);
    assert_eq!(paths.config_path, dir.join("config.json"));
    assert_eq!(paths.portmap_path, dir.join("portmap.json"));
    assert_eq!(paths.dnsmasq_dir, dir.join("dnsmasq.d"));

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// read_json
// ---------------------------------------------------------------------------

#[test]
fn read_json_parses_valid_file() {
    let dir = std::env::temp_dir().join("darp_test_read_json");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("test.json");
    std::fs::write(&path, r#"{"engine": "docker"}"#).unwrap();

    let config: Config = read_json(&path).unwrap();
    assert_eq!(config.engine.as_deref(), Some("docker"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_json_errors_on_missing_file() {
    let result: Result<Config, _> = read_json(std::path::Path::new("/nonexistent/path.json"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// resolve_location
// ---------------------------------------------------------------------------

#[test]
fn resolve_location_replaces_home_token() {
    let result = resolve_location("{home}/.darp").unwrap();
    let s = result.to_string_lossy();
    assert!(!s.contains("{home}"));
    assert!(s.ends_with("/.darp"));
}

#[test]
fn resolve_location_passes_through_absolute_path() {
    let result = resolve_location("/tmp/projects").unwrap();
    assert_eq!(result, PathBuf::from("/tmp/projects"));
}

// ---------------------------------------------------------------------------
// find_domain_by_location — needs real dirs for canonicalize
// ---------------------------------------------------------------------------

#[test]
fn find_domain_by_location_matches() {
    let dir = std::env::temp_dir().join("darp_test_find_dom");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let canonical = std::fs::canonicalize(&dir).unwrap();
    let canonical_str = canonical.to_string_lossy().to_string();

    let mut config = Config::default();
    config.add_domain("proj", &dir.to_string_lossy()).unwrap();

    let result = config.find_domain_by_location(&canonical_str);
    assert!(result.is_some());
    let (name, domain) = result.unwrap();
    assert_eq!(name, "proj");
    assert_eq!(domain.location, dir.to_string_lossy().as_ref());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn find_domain_by_location_no_match() {
    let mut config = Config::default();
    config.add_domain("proj", "/tmp/nonexistent_xyzzy").unwrap();
    assert!(config.find_domain_by_location("/tmp/other").is_none());
}

#[test]
fn find_domain_by_location_no_domains() {
    let config = Config::default();
    assert!(config.find_domain_by_location("/tmp/any").is_none());
}

// ---------------------------------------------------------------------------
// find_context_by_cwd — takes a path, uses real dirs
// ---------------------------------------------------------------------------

#[test]
fn find_context_parent_is_domain() {
    // Setup: domain at /tmp/darp_test_ctx/projects, cwd = /tmp/darp_test_ctx/projects/myapp
    let base = std::env::temp_dir().join("darp_test_ctx_parent");
    let _ = std::fs::remove_dir_all(&base);
    let domain_dir = base.join("projects");
    let service_dir = domain_dir.join("myapp");
    std::fs::create_dir_all(&service_dir).unwrap();

    let mut config = Config::default();
    config
        .add_domain("projects", &domain_dir.to_string_lossy())
        .unwrap();

    // Add a "." group with a service
    let mut dot_group = Group::default();
    let mut services = BTreeMap::new();
    services.insert(
        "myapp".to_string(),
        Service {
            serve_command: Some("npm start".into()),
            ..Default::default()
        },
    );
    dot_group.services = Some(services);
    let mut groups = BTreeMap::new();
    groups.insert(".".to_string(), dot_group);
    config
        .domains
        .as_mut()
        .unwrap()
        .get_mut("projects")
        .unwrap()
        .groups = Some(groups);

    let result = config.find_context_by_cwd(&service_dir);
    assert!(result.is_some());
    let (domain_name, _domain, group_name, group_opt) = result.unwrap();
    assert_eq!(domain_name, "projects");
    assert_eq!(group_name, ".");
    // Group should be the "." group we created
    assert!(group_opt.is_some());
    let group = group_opt.unwrap();
    assert!(group.services.as_ref().unwrap().contains_key("myapp"));

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn find_context_grandparent_is_domain() {
    // Setup: domain at /tmp/darp_test_ctx2/projects
    //        group dir = /tmp/darp_test_ctx2/projects/backend
    //        cwd = /tmp/darp_test_ctx2/projects/backend/api
    let base = std::env::temp_dir().join("darp_test_ctx_grand");
    let _ = std::fs::remove_dir_all(&base);
    let domain_dir = base.join("projects");
    let group_dir = domain_dir.join("backend");
    let service_dir = group_dir.join("api");
    std::fs::create_dir_all(&service_dir).unwrap();

    let mut config = Config::default();
    config
        .add_domain("projects", &domain_dir.to_string_lossy())
        .unwrap();

    // Add a "backend" group
    let mut backend_group = Group::default();
    backend_group.serve_command = Some("cargo run".into());
    let mut groups = BTreeMap::new();
    groups.insert("backend".to_string(), backend_group);
    config
        .domains
        .as_mut()
        .unwrap()
        .get_mut("projects")
        .unwrap()
        .groups = Some(groups);

    let result = config.find_context_by_cwd(&service_dir);
    assert!(result.is_some());
    let (domain_name, _domain, group_name, group_opt) = result.unwrap();
    assert_eq!(domain_name, "projects");
    assert_eq!(group_name, "backend");
    assert!(group_opt.is_some());

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn find_context_no_matching_domain() {
    let dir = std::env::temp_dir().join("darp_test_ctx_nomatch");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let config = Config::default(); // no domains
    assert!(config.find_context_by_cwd(&dir).is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// service_context_from_cwd — uses real cwd, so we chdir into a temp dir
// ---------------------------------------------------------------------------

#[test]
fn service_context_from_cwd_resolves_env_cascade() {
    let _lock = CWD_LOCK.lock().unwrap();
    let base = std::env::temp_dir().join("darp_test_svc_ctx");
    let _ = std::fs::remove_dir_all(&base);
    let domain_dir = base.join("projects");
    let service_dir = domain_dir.join("myapp");
    std::fs::create_dir_all(&service_dir).unwrap();

    let mut config = Config::default();
    config
        .add_domain("projects", &domain_dir.to_string_lossy())
        .unwrap();

    // Set domain default_environment
    let mut envs = BTreeMap::new();
    envs.insert(
        "go".to_string(),
        Environment {
            serve_command: Some("air".into()),
            ..Default::default()
        },
    );
    config.environments = Some(envs);
    config
        .domains
        .as_mut()
        .unwrap()
        .get_mut("projects")
        .unwrap()
        .default_environment = Some("go".to_string());

    // Change cwd to service_dir
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&service_dir).unwrap();

    let ctx = config.service_context_from_cwd(None);

    // Restore cwd before assertions (in case of panic)
    std::env::set_current_dir(&original_dir).unwrap();

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.domain_name, "projects");
    assert_eq!(ctx.current_directory_name, "myapp");
    assert_eq!(ctx.environment_name.as_deref(), Some("go"));
    assert!(ctx.environment.is_some());
    assert_eq!(
        ctx.environment.unwrap().serve_command.as_deref(),
        Some("air")
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn service_context_from_cwd_cli_env_overrides_domain_default() {
    let _lock = CWD_LOCK.lock().unwrap();
    let base = std::env::temp_dir().join("darp_test_svc_ctx_cli");
    let _ = std::fs::remove_dir_all(&base);
    let domain_dir = base.join("projects");
    let service_dir = domain_dir.join("myapp");
    std::fs::create_dir_all(&service_dir).unwrap();

    let mut config = Config::default();
    config
        .add_domain("projects", &domain_dir.to_string_lossy())
        .unwrap();

    let mut envs = BTreeMap::new();
    envs.insert("go".to_string(), Environment::default());
    envs.insert(
        "node".to_string(),
        Environment {
            serve_command: Some("npm start".into()),
            ..Default::default()
        },
    );
    config.environments = Some(envs);
    config
        .domains
        .as_mut()
        .unwrap()
        .get_mut("projects")
        .unwrap()
        .default_environment = Some("go".to_string());

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&service_dir).unwrap();

    // CLI env should override domain default
    let ctx = config.service_context_from_cwd(Some("node".to_string()));

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.environment_name.as_deref(), Some("node"));
    assert_eq!(
        ctx.environment.unwrap().serve_command.as_deref(),
        Some("npm start")
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn service_context_from_cwd_service_default_env_beats_group_and_domain() {
    let _lock = CWD_LOCK.lock().unwrap();
    let base = std::env::temp_dir().join("darp_test_svc_ctx_svc_wins");
    let _ = std::fs::remove_dir_all(&base);
    let domain_dir = base.join("projects");
    let service_dir = domain_dir.join("myapp");
    std::fs::create_dir_all(&service_dir).unwrap();

    let mut config = Config::default();
    config
        .add_domain("projects", &domain_dir.to_string_lossy())
        .unwrap();

    // Register all three envs referenced below.
    let mut envs = BTreeMap::new();
    envs.insert("dom-env".to_string(), Environment::default());
    envs.insert("grp-env".to_string(), Environment::default());
    envs.insert(
        "svc-env".to_string(),
        Environment {
            serve_command: Some("svc-serve".into()),
            ..Default::default()
        },
    );
    config.environments = Some(envs);

    // Domain default = dom-env
    let domain = config
        .domains
        .as_mut()
        .unwrap()
        .get_mut("projects")
        .unwrap();
    domain.default_environment = Some("dom-env".to_string());

    // "." group with default = grp-env, and a service "myapp" with default = svc-env
    let mut dot_group = Group::default();
    dot_group.default_environment = Some("grp-env".to_string());
    let mut services = BTreeMap::new();
    services.insert(
        "myapp".to_string(),
        Service {
            default_environment: Some("svc-env".into()),
            ..Default::default()
        },
    );
    dot_group.services = Some(services);
    let mut groups = BTreeMap::new();
    groups.insert(".".to_string(), dot_group);
    domain.groups = Some(groups);

    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&service_dir).unwrap();

    // No -e flag: svc-env should win over grp-env and dom-env
    let ctx = config.service_context_from_cwd(None);

    std::env::set_current_dir(&original_dir).unwrap();

    assert!(ctx.is_some());
    let ctx = ctx.unwrap();
    assert_eq!(ctx.environment_name.as_deref(), Some("svc-env"));
    assert_eq!(
        ctx.environment.unwrap().serve_command.as_deref(),
        Some("svc-serve")
    );

    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// merge_values — array concatenation (the uncovered branch)
// ---------------------------------------------------------------------------

#[test]
fn load_merged_concatenates_array_values() {
    let dir = std::env::temp_dir().join("darp_test_array_merge");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Parent has a domain with one volume
    let parent_path = dir.join("parent.json");
    std::fs::write(
        &parent_path,
        r#"{
            "domains": {
                "proj": {
                    "location": "/tmp/proj",
                    "volumes": [{"container": "/a", "host": "/ha"}]
                }
            }
        }"#,
    )
    .unwrap();

    // Leaf adds another volume to the same domain via overlay
    // Because of merge_values semantics: arrays concatenate
    let leaf_path = dir.join("config.json");
    let leaf = format!(
        r#"{{
            "pre_config": [{{"location": "{}"}}],
            "domains": {{
                "proj": {{
                    "location": "/tmp/proj",
                    "volumes": [{{"container": "/b", "host": "/hb"}}]
                }}
            }}
        }}"#,
        parent_path.display()
    );
    std::fs::write(&leaf_path, leaf).unwrap();

    let config = Config::load_merged(&leaf_path).unwrap();
    let vols = config.domains.as_ref().unwrap()["proj"]
        .volumes
        .as_ref()
        .unwrap();
    // Arrays should be concatenated: parent's [/a] + leaf's [/b]
    assert_eq!(vols.len(), 2);

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Config::load creates file when missing
// ---------------------------------------------------------------------------

#[test]
fn load_creates_config_when_missing() {
    let dir = std::env::temp_dir().join("darp_test_load_missing");
    let _ = std::fs::remove_dir_all(&dir);
    // Don't create the directory — load should create it
    let path = dir.join("subdir").join("config.json");

    let config = Config::load(&path).unwrap();
    // Should be an empty default config
    assert!(config.engine.is_none());
    assert!(config.domains.is_none());
    // File should now exist with "{}"
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "{}");

    let _ = std::fs::remove_dir_all(&dir);
}
