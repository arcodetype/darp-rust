use darp::config::Config;

fn config_with_domain(name: &str, location: &str) -> Config {
    let mut c = Config::default();
    c.add_domain(name, location).unwrap();
    c
}

// ---------------------------------------------------------------------------
// Environment-level property lifecycle (set + rm for all 5 fields)
// ---------------------------------------------------------------------------

#[test]
fn env_serve_command_lifecycle() {
    let mut c = Config::default();
    // set_serve_command auto-creates the environment
    c.set_serve_command("go", "air").unwrap();

    let env = &c.environments.as_ref().unwrap()["go"];
    assert_eq!(env.serve_command.as_deref(), Some("air"));

    c.rm_serve_command("go").unwrap();
    assert!(
        c.environments.as_ref().unwrap()["go"]
            .serve_command
            .is_none()
    );

    // rm when already None errors
    assert!(c.rm_serve_command("go").is_err());
}

#[test]
fn env_shell_command_lifecycle() {
    let mut c = Config::default();
    c.set_serve_command("go", "air").unwrap(); // create env first
    c.set_shell_command("go", "zsh").unwrap();

    assert_eq!(
        c.environments.as_ref().unwrap()["go"]
            .shell_command
            .as_deref(),
        Some("zsh")
    );

    c.rm_shell_command("go").unwrap();
    assert!(
        c.environments.as_ref().unwrap()["go"]
            .shell_command
            .is_none()
    );
    assert!(c.rm_shell_command("go").is_err());
}

#[test]
fn env_image_repository_lifecycle() {
    let mut c = Config::default();
    c.set_serve_command("go", "air").unwrap();
    c.set_image_repository("go", "ghcr.io/org").unwrap();

    assert_eq!(
        c.environments.as_ref().unwrap()["go"]
            .image_repository
            .as_deref(),
        Some("ghcr.io/org")
    );

    c.rm_image_repository("go").unwrap();
    assert!(c.rm_image_repository("go").is_err());
}

#[test]
fn env_platform_lifecycle() {
    let mut c = Config::default();
    c.set_serve_command("go", "air").unwrap();
    c.set_platform("go", "linux/amd64").unwrap();

    assert_eq!(
        c.environments.as_ref().unwrap()["go"].platform.as_deref(),
        Some("linux/amd64")
    );

    c.rm_platform("go").unwrap();
    assert!(c.rm_platform("go").is_err());
}

#[test]
fn env_default_container_image_lifecycle() {
    let mut c = Config::default();
    c.set_serve_command("go", "air").unwrap();
    c.set_default_container_image("go", "darp-go").unwrap();

    assert_eq!(
        c.environments.as_ref().unwrap()["go"]
            .default_container_image
            .as_deref(),
        Some("darp-go")
    );

    c.rm_default_container_image("go").unwrap();
    assert!(c.rm_default_container_image("go").is_err());
}

#[test]
fn env_operations_error_on_missing_env() {
    let mut c = Config::default();
    assert!(c.set_shell_command("nope", "zsh").is_err());
    assert!(c.rm_serve_command("nope").is_err());
}

// ---------------------------------------------------------------------------
// Environment-level portmap/variable/volume lifecycle
// ---------------------------------------------------------------------------

#[test]
fn env_portmap_lifecycle() {
    let mut c = Config::default();
    c.add_env_portmap("go", "3000", "3000").unwrap();

    let pm = c.environments.as_ref().unwrap()["go"]
        .host_portmappings
        .as_ref()
        .unwrap();
    assert_eq!(pm.get("3000").unwrap(), "3000");

    // duplicate rejected
    assert!(c.add_env_portmap("go", "3000", "4000").is_err());

    c.rm_env_portmap("go", "3000").unwrap();
    // rm missing key errors
    assert!(c.rm_env_portmap("go", "3000").is_err());
}

#[test]
fn env_variable_lifecycle() {
    let mut c = Config::default();
    c.add_env_variable("go", "GOPROXY", "direct").unwrap();

    let vars = c.environments.as_ref().unwrap()["go"]
        .variables
        .as_ref()
        .unwrap();
    assert_eq!(vars.get("GOPROXY").unwrap(), "direct");

    assert!(c.add_env_variable("go", "GOPROXY", "other").is_err());

    c.rm_env_variable("go", "GOPROXY").unwrap();
    assert!(c.rm_env_variable("go", "GOPROXY").is_err());
}

#[test]
fn env_volume_lifecycle() {
    let mut c = Config::default();
    c.add_volume("go", "/cache", "/host/cache").unwrap();

    let vols = c.environments.as_ref().unwrap()["go"]
        .volumes
        .as_ref()
        .unwrap();
    assert_eq!(vols.len(), 1);

    // exact duplicate rejected
    assert!(c.add_volume("go", "/cache", "/host/cache").is_err());

    c.rm_volume("go", "/cache", "/host/cache").unwrap();
    assert!(c.rm_volume("go", "/cache", "/host/cache").is_err());
}

#[test]
fn env_rm_errors_when_no_environments() {
    let mut c = Config::default();
    assert!(c.rm_env_portmap("nope", "3000").is_err());
    assert!(c.rm_env_variable("nope", "KEY").is_err());
    assert!(c.rm_volume("nope", "/c", "/h").is_err());
}

// ---------------------------------------------------------------------------
// Domain-level remaining rm_ functions
// ---------------------------------------------------------------------------

#[test]
fn rm_domain_shell_command_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_domain_shell_command("d", "zsh").unwrap();
    c.rm_domain_shell_command("d").unwrap();
    assert!(c.rm_domain_shell_command("d").is_err());
}

#[test]
fn rm_domain_image_repository_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_domain_image_repository("d", "ghcr.io").unwrap();
    c.rm_domain_image_repository("d").unwrap();
    assert!(c.rm_domain_image_repository("d").is_err());
}

#[test]
fn rm_domain_platform_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_domain_platform("d", "linux/arm64").unwrap();
    c.rm_domain_platform("d").unwrap();
    assert!(c.rm_domain_platform("d").is_err());
}

#[test]
fn rm_domain_default_container_image_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_domain_default_container_image("d", "myimg").unwrap();
    c.rm_domain_default_container_image("d").unwrap();
    assert!(c.rm_domain_default_container_image("d").is_err());
}

#[test]
fn rm_domain_default_environment_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_serve_command("go", "air").unwrap(); // create env
    c.set_domain_default_environment("d", "go").unwrap();

    assert_eq!(
        c.domains.as_ref().unwrap()["d"]
            .default_environment
            .as_deref(),
        Some("go")
    );

    c.rm_domain_default_environment("d").unwrap();
    assert!(c.rm_domain_default_environment("d").is_err());
}

// ---------------------------------------------------------------------------
// Group-level rm_ functions
// ---------------------------------------------------------------------------

#[test]
fn rm_group_serve_command_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_serve_command("d", "g", "npm start").unwrap();
    c.rm_group_serve_command("d", "g").unwrap();
    assert!(c.rm_group_serve_command("d", "g").is_err());
}

#[test]
fn rm_group_shell_command_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_shell_command("d", "g", "bash").unwrap();
    c.rm_group_shell_command("d", "g").unwrap();
    assert!(c.rm_group_shell_command("d", "g").is_err());
}

#[test]
fn rm_group_image_repository_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_image_repository("d", "g", "ghcr.io").unwrap();
    c.rm_group_image_repository("d", "g").unwrap();
    assert!(c.rm_group_image_repository("d", "g").is_err());
}

#[test]
fn rm_group_platform_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_platform("d", "g", "linux/amd64").unwrap();
    c.rm_group_platform("d", "g").unwrap();
    assert!(c.rm_group_platform("d", "g").is_err());
}

#[test]
fn rm_group_default_container_image_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_group_default_container_image("d", "g", "img")
        .unwrap();
    c.rm_group_default_container_image("d", "g").unwrap();
    assert!(c.rm_group_default_container_image("d", "g").is_err());
}

#[test]
fn rm_group_default_environment_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_serve_command("go", "air").unwrap(); // create env
    c.set_group_default_environment("d", "g", "go").unwrap();

    let grp = &c.domains.as_ref().unwrap()["d"].groups.as_ref().unwrap()["g"];
    assert_eq!(grp.default_environment.as_deref(), Some("go"));

    c.rm_group_default_environment("d", "g").unwrap();
    assert!(c.rm_group_default_environment("d", "g").is_err());
}

#[test]
fn rm_group_portmap_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_group_portmap("d", "g", "8080", "80").unwrap();
    c.rm_group_portmap("d", "g", "8080").unwrap();
    assert!(c.rm_group_portmap("d", "g", "8080").is_err());
}

#[test]
fn rm_group_variable_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_group_variable("d", "g", "KEY", "VAL").unwrap();
    c.rm_group_variable("d", "g", "KEY").unwrap();
    assert!(c.rm_group_variable("d", "g", "KEY").is_err());
}

#[test]
fn rm_group_volume_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_group_volume("d", "g", "/data", "/host/data").unwrap();
    c.rm_group_volume("d", "g", "/data", "/host/data").unwrap();
    assert!(c.rm_group_volume("d", "g", "/data", "/host/data").is_err());
}

// ---------------------------------------------------------------------------
// Service-level rm_ and set_ functions
// ---------------------------------------------------------------------------

#[test]
fn service_portmap_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_portmap("d", "g", "svc", "9090", "90").unwrap();

    // duplicate rejected
    assert!(c.add_portmap("d", "g", "svc", "9090", "99").is_err());

    c.rm_portmap("d", "g", "svc", "9090").unwrap();
    assert!(c.rm_portmap("d", "g", "svc", "9090").is_err());
}

#[test]
fn service_variable_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_variable("d", "g", "svc", "PORT", "3000").unwrap();

    assert!(c.add_variable("d", "g", "svc", "PORT", "4000").is_err());

    c.rm_variable("d", "g", "svc", "PORT").unwrap();
    assert!(c.rm_variable("d", "g", "svc", "PORT").is_err());
}

#[test]
fn service_volume_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.add_service_volume("d", "g", "svc", "/app", "/host/app")
        .unwrap();

    assert!(
        c.add_service_volume("d", "g", "svc", "/app", "/host/app")
            .is_err()
    );

    c.rm_service_volume("d", "g", "svc", "/app", "/host/app")
        .unwrap();
    assert!(
        c.rm_service_volume("d", "g", "svc", "/app", "/host/app")
            .is_err()
    );
}

#[test]
fn service_serve_command_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_serve_command("d", "g", "svc", "npm start")
        .unwrap();

    let svc = &c.domains.as_ref().unwrap()["d"].groups.as_ref().unwrap()["g"]
        .services
        .as_ref()
        .unwrap()["svc"];
    assert_eq!(svc.serve_command.as_deref(), Some("npm start"));

    c.rm_service_serve_command("d", "g", "svc").unwrap();
    assert!(c.rm_service_serve_command("d", "g", "svc").is_err());
}

#[test]
fn service_default_environment_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_serve_command("go", "air").unwrap(); // create env
    c.set_service_default_environment("d", "g", "svc", "go")
        .unwrap();

    let svc = &c.domains.as_ref().unwrap()["d"].groups.as_ref().unwrap()["g"]
        .services
        .as_ref()
        .unwrap()["svc"];
    assert_eq!(svc.default_environment.as_deref(), Some("go"));

    c.rm_service_default_environment("d", "g", "svc").unwrap();
    assert!(c.rm_service_default_environment("d", "g", "svc").is_err());
}

#[test]
fn service_shell_command_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_shell_command("d", "g", "svc", "bash")
        .unwrap();
    c.rm_service_shell_command("d", "g", "svc").unwrap();
    assert!(c.rm_service_shell_command("d", "g", "svc").is_err());
}

#[test]
fn service_image_repository_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_image_repository("d", "g", "svc", "ghcr.io")
        .unwrap();
    c.rm_service_image_repository("d", "g", "svc").unwrap();
    assert!(c.rm_service_image_repository("d", "g", "svc").is_err());
}

#[test]
fn service_platform_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_platform("d", "g", "svc", "linux/arm64")
        .unwrap();
    c.rm_service_platform("d", "g", "svc").unwrap();
    assert!(c.rm_service_platform("d", "g", "svc").is_err());
}

#[test]
fn service_default_container_image_lifecycle() {
    let mut c = config_with_domain("d", "/tmp/d");
    c.set_service_default_container_image("d", "g", "svc", "img")
        .unwrap();
    c.rm_service_default_container_image("d", "g", "svc")
        .unwrap();
    assert!(
        c.rm_service_default_container_image("d", "g", "svc")
            .is_err()
    );
}
