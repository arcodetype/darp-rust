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

// ---------------------------------------------------------------------------
// Hierarchical merge — collection fields accumulate across layers
// ---------------------------------------------------------------------------

fn vol(host: &str, container: &str) -> Volume {
    Volume {
        host: host.into(),
        container: container.into(),
    }
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn volumes_merge_across_domain_group_service() {
    let svc = Service {
        volumes: Some(vec![vol("/s", "/s")]),
        ..Default::default()
    };
    let grp = Group {
        volumes: Some(vec![vol("/g", "/g")]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![vol("/d", "/d")]),
        ..Default::default()
    };

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

    let vols = r.volumes.unwrap();
    // Walk order: env, domain, group, service → [d, g, s]
    assert_eq!(vols.len(), 3);
    assert_eq!(vols[0].host, "/d");
    assert_eq!(vols[1].host, "/g");
    assert_eq!(vols[2].host, "/s");
}

#[test]
fn variables_merge_service_wins_on_conflict() {
    let svc = Service {
        variables: Some(map(&[("PORT", "9100"), ("SVC", "1")])),
        ..Default::default()
    };
    let grp = Group {
        variables: Some(map(&[("PORT", "9000")])),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        variables: Some(map(&[("PORT", "8000"), ("DOM", "1")])),
        ..Default::default()
    };

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

    let vars = r.variables.unwrap();
    assert_eq!(vars.get("PORT").unwrap(), "9100"); // service wins on conflict
    assert_eq!(vars.get("DOM").unwrap(), "1"); // inherited from domain
    assert_eq!(vars.get("SVC").unwrap(), "1"); // service-only key kept
    assert_eq!(vars.len(), 3);
}

#[test]
fn host_portmappings_merge_across_levels() {
    let svc = Service {
        host_portmappings: Some(map(&[("8080", "80")])),
        ..Default::default()
    };
    let grp = Group {
        host_portmappings: Some(map(&[("9090", "90")])),
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
    assert_eq!(pm.len(), 2);
    assert_eq!(pm.get("8080").unwrap(), "80");
    assert_eq!(pm.get("9090").unwrap(), "90");
}

#[test]
fn environment_layer_contributes_when_unmerged() {
    let env = Environment {
        volumes: Some(vec![vol("/e", "/e")]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![vol("/d", "/d")]),
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

    let vols = r.volumes.unwrap();
    assert_eq!(vols.len(), 2);
    assert_eq!(vols[0].host, "/e"); // env walked first
    assert_eq!(vols[1].host, "/d");
}

// ---------------------------------------------------------------------------
// `*field` override — resets parent chain at the declaring layer
// ---------------------------------------------------------------------------

#[test]
fn volumes_override_at_service_discards_parents() {
    let svc = Service {
        volumes_override: Some(Some(vec![vol("/s", "/s")])),
        ..Default::default()
    };
    let grp = Group {
        volumes: Some(vec![vol("/g", "/g")]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![vol("/d", "/d")]),
        ..Default::default()
    };

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

    let vols = r.volumes.unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].host, "/s"); // only service's volume; dom+grp discarded
}

#[test]
fn volumes_override_at_group_keeps_service_append() {
    let svc = Service {
        volumes: Some(vec![vol("/s", "/s")]),
        ..Default::default()
    };
    let grp = Group {
        volumes_override: Some(Some(vec![vol("/g", "/g")])),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![vol("/d", "/d")]),
        ..Default::default()
    };

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

    let vols = r.volumes.unwrap();
    assert_eq!(vols.len(), 2);
    assert_eq!(vols[0].host, "/g"); // group's `*` reset, domain discarded
    assert_eq!(vols[1].host, "/s"); // service still appends
}

#[test]
fn volumes_override_at_domain_discards_environment() {
    let env = Environment {
        volumes: Some(vec![vol("/e", "/e")]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes_override: Some(Some(vec![vol("/d", "/d")])),
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

    let vols = r.volumes.unwrap();
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].host, "/d");
}

#[test]
fn volumes_override_null_at_service_discards_all() {
    let svc = Service {
        volumes_override: Some(None), // *volumes: null
        ..Default::default()
    };
    let grp = Group {
        volumes: Some(vec![vol("/g", "/g")]),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        volumes: Some(vec![vol("/d", "/d")]),
        ..Default::default()
    };

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

    assert!(r.volumes.is_none());
}

// ---------------------------------------------------------------------------
// Scalar semantics — first-wins by default, asterisk-null explicitly clears
// ---------------------------------------------------------------------------

#[test]
fn scalars_merge_service_wins() {
    let svc = Service {
        serve_command: Some("svc".into()),
        ..Default::default()
    };
    let grp = Group {
        serve_command: Some("grp".into()),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("dom".into()),
        ..Default::default()
    };

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

    assert_eq!(r.serve_command.as_deref(), Some("svc"));
}

#[test]
fn scalar_override_null_at_group_blocks_domain_inheritance() {
    let grp = Group {
        serve_command_override: Some(None), // *serve_command: null
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("air".into()),
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

    assert!(r.serve_command.is_none());
}

#[test]
fn scalar_override_null_at_group_service_still_wins() {
    let svc = Service {
        serve_command: Some("bun".into()),
        ..Default::default()
    };
    let grp = Group {
        serve_command_override: Some(None),
        ..Default::default()
    };
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("air".into()),
        ..Default::default()
    };

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

    // svc walks after grp → overwrites the null with "bun".
    assert_eq!(r.serve_command.as_deref(), Some("bun"));
}

#[test]
fn scalar_plain_null_is_ignored_falls_through() {
    // A service without a declared serve_command (no *override either) must fall
    // through to the domain's "air" — this is the first-wins fallback.
    let svc = Service::default();
    let dom = Domain {
        location: "/tmp".into(),
        serve_command: Some("air".into()),
        ..Default::default()
    };

    let r = ResolvedSettings::resolve(
        "d".into(),
        ".".into(),
        "s".into(),
        None,
        Some(&svc),
        None,
        &dom,
        None,
    );

    assert_eq!(r.serve_command.as_deref(), Some("air"));
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
