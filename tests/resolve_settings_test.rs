use std::collections::BTreeMap;

use darp::config::{Domain, Environment, Group, ResolvedSettings, Service, Volume};

fn bare_domain() -> Domain {
    Domain {
        location: "/tmp/test".into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// ResolvedSettings::resolve — cascade priority
// ---------------------------------------------------------------------------

#[test]
fn service_overrides_all() {
    let svc = Service {
        serve_command: Some("svc-cmd".into()),
        ..Default::default()
    };
    let grp = Group {
        serve_command: Some("grp-cmd".into()),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("dom-cmd".into()),
        ..Default::default()
    };
    let env = Environment {
        serve_command: Some("env-cmd".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        Some("e".into()),
        Some(&svc),
        Some(&grp),
        &dom,
        Some(&env),
    );

    assert_eq!(r.serve_command.as_deref(), Some("svc-cmd"));
}

#[test]
fn group_overrides_domain_and_env() {
    let grp = Group {
        shell_command: Some("grp-shell".into()),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        shell_command: Some("dom-shell".into()),
        ..Default::default()
    };
    let env = Environment {
        shell_command: Some("env-shell".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        "g".into(),
        "s".into(),
        Some("e".into()),
        None, // no service
        Some(&grp),
        &dom,
        Some(&env),
    );

    assert_eq!(r.shell_command.as_deref(), Some("grp-shell"));
}

#[test]
fn domain_overrides_env() {
    let dom = Domain {
        location: "/tmp".into(),
        platform: Some("linux/amd64".into()),
        ..Default::default()
    };
    let env = Environment {
        platform: Some("linux/arm64".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        Some("e".into()),
        None,
        None,
        &dom,
        Some(&env),
    );

    assert_eq!(r.platform.as_deref(), Some("linux/amd64"));
}

#[test]
fn env_is_fallback() {
    let dom = bare_domain(); // no serve_command
    let env = Environment {
        image_repository: Some("ghcr.io/test".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        Some("e".into()),
        None,
        None,
        &dom,
        Some(&env),
    );

    assert_eq!(r.image_repository.as_deref(), Some("ghcr.io/test"));
}

#[test]
fn all_none_yields_none() {
    let dom = bare_domain();

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert!(r.serve_command.is_none());
    assert!(r.shell_command.is_none());
    assert!(r.image_repository.is_none());
    assert!(r.platform.is_none());
    assert!(r.default_container_image.is_none());
    assert!(r.host_portmappings.is_none());
    assert!(r.variables.is_none());
    assert!(r.volumes.is_none());
}

#[test]
fn domain_only_no_other_layers() {
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("air".into()),
        shell_command: Some("zsh".into()),
        default_container_image: Some("myimg".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert_eq!(r.serve_command.as_deref(), Some("air"));
    assert_eq!(r.shell_command.as_deref(), Some("zsh"));
    assert_eq!(r.default_container_image.as_deref(), Some("myimg"));
}

#[test]
fn map_fields_first_wins_not_merged() {
    let mut svc_ports = BTreeMap::new();
    svc_ports.insert("8080".into(), "80".into());

    let mut grp_ports = BTreeMap::new();
    grp_ports.insert("9090".into(), "90".into());

    let svc = Service {
        host_portmappings: Some(svc_ports),
        ..Default::default()
    };
    let grp = Group {
        host_portmappings: Some(grp_ports),
        ..Default::default()
    };
    let dom = bare_domain();

    let r = ResolvedSettings::resolve(
        "d".into(),
        "g".into(),
        "s".into(),
        None,
        Some(&svc),
        Some(&grp),
        &dom,
        None,
    );

    let pm = r.host_portmappings.unwrap();
    assert_eq!(pm.len(), 1);
    assert_eq!(pm.get("8080").unwrap(), "80");
    assert!(!pm.contains_key("9090")); // group's map NOT merged in
}

#[test]
fn vec_fields_first_wins_not_merged() {
    let grp = Group {
        volumes: Some(vec![Volume {
            host: "/grp/data".into(),
            container: "/data".into(),
        }]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![Volume {
            host: "/dom/data".into(),
            container: "/data".into(),
        }]),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        "g".into(),
        "s".into(),
        None,
        None,
        Some(&grp),
        &dom,
        None,
    );

    let vols = r.volumes.unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].host, "/grp/data"); // group wins, domain not merged
}

// ---------------------------------------------------------------------------
// ResolvedSettings::resolve_full_image_name
// ---------------------------------------------------------------------------

#[test]
fn cli_image_no_repo() {
    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &bare_domain(),
        None,
    );

    assert_eq!(
        r.resolve_full_image_name(Some("alpine")),
        Some("alpine".into())
    );
}

#[test]
fn cli_image_with_repo() {
    let dom = Domain {
        location: "/tmp".into(),
        image_repository: Some("ghcr.io/org".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert_eq!(
        r.resolve_full_image_name(Some("myapp")),
        Some("ghcr.io/org:myapp".into())
    );
}

#[test]
fn default_image_no_repo() {
    let dom = Domain {
        location: "/tmp".into(),
        default_container_image: Some("darp-go".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert_eq!(r.resolve_full_image_name(None), Some("darp-go".into()));
}

#[test]
fn default_image_with_repo() {
    let dom = Domain {
        location: "/tmp".into(),
        default_container_image: Some("darp-go".into()),
        image_repository: Some("ghcr.io/org".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert_eq!(
        r.resolve_full_image_name(None),
        Some("ghcr.io/org:darp-go".into())
    );
}

#[test]
fn no_image_returns_none() {
    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &bare_domain(),
        None,
    );

    assert_eq!(r.resolve_full_image_name(None), None);
}

#[test]
fn cli_overrides_default() {
    let dom = Domain {
        location: "/tmp".into(),
        default_container_image: Some("default-img".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        None,
        None,
        &dom,
        None,
    );

    assert_eq!(
        r.resolve_full_image_name(Some("override")),
        Some("override".into())
    );
}
