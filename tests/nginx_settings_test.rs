//! Configurable nginx directives: the `nginx` map's validation, cascade, and
//! rendering into `nginx.conf` and the generated vhosts.

use std::collections::BTreeMap;

use darp::commands::{
    PlannedService, build_host_proxy_vhost, build_vhost_container_conf, plan_deployment,
    validate_nginx_directives,
};
use darp::config::{Config, DarpPaths, resolve_nginx_directives, validate_nginx_directive};
use darp::engine::{Engine, EngineKind};
use darp::os::{DEFAULT_HTTP_DIRECTIVES, render_nginx_conf};

fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn planned(service: &str, nginx: &[(&str, &str)]) -> PlannedService {
    PlannedService {
        domain: "dom".to_string(),
        group: ".".to_string(),
        service: service.to_string(),
        connection_type: "http".to_string(),
        proxy_port: 50100,
        debug_port: 13000,
        canonical_hostname: format!("{service}.dom.test"),
        aliases: vec![],
        nginx: map(nginx),
    }
}

fn config(json: &str) -> Config {
    serde_json::from_str(json).expect("config should deserialize")
}

/// Resolve the directives for `dom/<group>/<service>` straight from a config.
fn resolved(cfg: &Config, group: &str, service: &str) -> BTreeMap<String, String> {
    let domain = &cfg.domains.as_ref().unwrap()["dom"];
    let group = domain.groups.as_ref().and_then(|g| g.get(group));
    let service = group
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(service));
    resolve_nginx_directives(domain, group, service)
}

/// How many lines set `name`, to check a directive appears exactly once.
fn count_directive(conf: &str, name: &str) -> usize {
    conf.lines()
        .filter(|l| l.trim_start().starts_with(&format!("{name} ")))
        .count()
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn accepts_ordinary_directives() {
    for (name, value) in [
        ("proxy_read_timeout", "300s"),
        ("proxy_buffering", "off"),
        ("large_client_header_buffers", "4 16k"),
        ("add_header", "X-Debug \"yes\""),
        ("client_max_body_size", "0"),
        ("client_max_body_size", "50m"),
        ("proxy_buffer_size", "16K"),
    ] {
        assert!(
            validate_nginx_directive(name, value).is_ok(),
            "{name} {value}"
        );
    }
}

#[test]
fn rejects_malformed_names() {
    for name in [
        "",
        "Proxy_read_timeout",
        "proxy-read-timeout",
        "1abc",
        "a b",
        "_x",
    ] {
        assert!(validate_nginx_directive(name, "on").is_err(), "{name:?}");
    }
}

#[test]
fn rejects_directives_darp_manages() {
    for name in [
        "listen",
        "server_name",
        "location",
        "proxy_pass",
        "include",
        "server",
    ] {
        let err = validate_nginx_directive(name, "x").unwrap_err().to_string();
        assert!(err.contains("managed by darp"), "{err}");
    }
}

#[test]
fn rejects_values_that_could_escape_the_directive() {
    for value in [
        "",
        "  ",
        "1m; evil",
        "on }",
        "{",
        "on # comment",
        "on\nevil on",
        "on\r",
    ] {
        assert!(
            validate_nginx_directive("proxy_buffering", value).is_err(),
            "{value:?}"
        );
    }
}

#[test]
fn size_directives_require_a_unit() {
    for bad in ["50", "1024", "00", "10mb", "1.5m", "-1"] {
        assert!(
            validate_nginx_directive("client_max_body_size", bad).is_err(),
            "{bad:?}"
        );
    }
    let err = validate_nginx_directive("client_max_body_size", "50")
        .unwrap_err()
        .to_string();
    assert!(err.contains("bytes"), "{err}");
    assert!(err.contains("'50m'"), "{err}");
}

#[test]
fn unit_rule_only_applies_to_size_directives() {
    // A bare number is perfectly normal for most directives.
    assert!(validate_nginx_directive("keepalive_requests", "1000").is_ok());
}

#[test]
fn bare_json_numbers_load_as_digits() {
    let cfg = config(r#"{ "nginx": { "client_max_body_size": 0, "keepalive_requests": 1000 } }"#);
    let nginx = cfg.nginx.as_ref().unwrap();
    assert_eq!(nginx["client_max_body_size"], "0");
    assert_eq!(nginx["keepalive_requests"], "1000");
    assert!(validate_nginx_directives(&cfg, &[]).is_ok());
}

#[test]
fn bare_json_size_without_unit_loads_but_fails_validation() {
    // The config must still load so unrelated commands keep working; deploy is
    // where the mistake gets reported.
    let cfg = config(r#"{ "nginx": { "client_max_body_size": 50 } }"#);
    let err = validate_nginx_directives(&cfg, &[])
        .unwrap_err()
        .to_string();
    assert!(err.contains("bytes"), "{err}");
}

#[test]
fn non_scalar_json_value_is_rejected_on_load() {
    assert!(serde_json::from_str::<Config>(r#"{ "nginx": { "gzip": true } }"#).is_err());
    assert!(serde_json::from_str::<Config>(r#"{ "nginx": { "gzip": ["on"] } }"#).is_err());
}

#[test]
fn deploy_validation_reports_global_and_every_service_together() {
    let cfg = config(r#"{ "nginx": { "proxy_pass": "x" } }"#);
    let services = [
        planned("a", &[("proxy_read_timeout", "300s")]),
        planned("b", &[("client_max_body_size", "1m; evil")]),
    ];

    let err = validate_nginx_directives(&cfg, &services)
        .unwrap_err()
        .to_string();
    assert!(err.contains("global"), "{err}");
    assert!(err.contains("dom/./b"), "{err}");
    assert!(!err.contains("dom/./a"), "{err}");
}

// ---------------------------------------------------------------------------
// Cascade: domain → group → service
// ---------------------------------------------------------------------------

const CASCADE: &str = r#"{
    "domains": {
        "dom": {
            "location": "/nowhere",
            "nginx": { "client_max_body_size": "10m", "proxy_read_timeout": "120s" },
            "groups": {
                ".": {
                    "services": {
                        "inherits": {},
                        "own": { "nginx": { "client_max_body_size": 0, "proxy_buffering": "off" } },
                        "replaced": { "*nginx": { "proxy_buffering": "off" } },
                        "cleared": { "*nginx": null }
                    }
                },
                "big": {
                    "nginx": { "client_max_body_size": "200m" },
                    "services": { "svc": {} }
                }
            }
        }
    }
}"#;

#[test]
fn service_inherits_domain_map() {
    assert_eq!(
        resolved(&config(CASCADE), ".", "inherits"),
        map(&[
            ("client_max_body_size", "10m"),
            ("proxy_read_timeout", "120s")
        ])
    );
}

#[test]
fn service_keys_merge_over_domain() {
    assert_eq!(
        resolved(&config(CASCADE), ".", "own"),
        map(&[
            ("client_max_body_size", "0"),
            ("proxy_buffering", "off"),
            ("proxy_read_timeout", "120s"),
        ])
    );
}

#[test]
fn group_keys_merge_over_domain() {
    assert_eq!(
        resolved(&config(CASCADE), "big", "svc"),
        map(&[
            ("client_max_body_size", "200m"),
            ("proxy_read_timeout", "120s")
        ])
    );
}

#[test]
fn star_override_replaces_inherited_map() {
    assert_eq!(
        resolved(&config(CASCADE), ".", "replaced"),
        map(&[("proxy_buffering", "off")])
    );
}

#[test]
fn null_override_clears_inherited_map() {
    assert!(resolved(&config(CASCADE), ".", "cleared").is_empty());
}

#[test]
fn unconfigured_service_folder_still_inherits() {
    assert_eq!(
        resolved(&config(CASCADE), ".", "not-in-config"),
        map(&[
            ("client_max_body_size", "10m"),
            ("proxy_read_timeout", "120s")
        ])
    );
}

#[test]
fn plan_deployment_carries_resolved_map() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("darp");
    let projects = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    for folder in ["inherits", "own"] {
        std::fs::create_dir_all(projects.join(folder)).unwrap();
    }
    let paths = DarpPaths {
        _darp_root: root.clone(),
        config_path: root.join("config.json"),
        portmap_path: root.join("portmap.json"),
        dnsmasq_dir: root.join("dnsmasq.d"),
        vhost_container_conf: root.join("vhost_container.conf"),
        hosts_container_path: root.join("hosts_container"),
        nginx_conf_path: root.join("nginx.conf"),
        container_host_ip_path: root.join("container_host_ip"),
    };
    let cfg = config(&CASCADE.replace("/nowhere", &projects.to_string_lossy()));

    let mut services = plan_deployment(&paths, &cfg).unwrap();
    services.sort_by(|a, b| a.service.cmp(&b.service));

    assert_eq!(services[0].service, "inherits");
    assert_eq!(services[0].nginx["client_max_body_size"], "10m");
    assert_eq!(services[1].service, "own");
    assert_eq!(services[1].nginx["client_max_body_size"], "0");
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

#[test]
fn vhost_has_no_extra_directives_when_map_is_empty() {
    let vhost = build_host_proxy_vhost("a.dom.test", "gw", 50100, &BTreeMap::new());
    assert!(!vhost.contains("{directives}"));
    assert!(vhost.contains("proxy_set_header Connection $connection_upgrade;\n    }\n}\n"));
}

#[test]
fn vhost_renders_directives_inside_location() {
    let vhost = build_host_proxy_vhost(
        "a.dom.test",
        "gw",
        50100,
        &map(&[
            ("client_max_body_size", "0"),
            ("proxy_read_timeout", "300s"),
        ]),
    );
    let location = vhost.find("location / {").unwrap();
    let body_size = vhost
        .find("        client_max_body_size 0;\n")
        .expect(&vhost);
    let timeout = vhost
        .find("        proxy_read_timeout 300s;\n")
        .expect(&vhost);
    let location_end = vhost.rfind("    }\n}").unwrap();
    assert!(location < body_size && body_size < location_end, "{vhost}");
    assert!(location < timeout && timeout < location_end, "{vhost}");
}

#[test]
fn every_hostname_of_a_service_gets_its_directives() {
    let mut svc = planned("a", &[("client_max_body_size", "50m")]);
    svc.aliases = vec!["local.example.org".to_string()];
    let conf = build_vhost_container_conf(&[svc, planned("b", &[])], "gw");
    assert_eq!(conf.matches("client_max_body_size 50m;").count(), 2);
    assert_eq!(conf.matches("server {").count(), 3);
}

#[test]
fn nginx_conf_renders_every_default_once() {
    let conf = render_nginx_conf(None).unwrap();
    assert!(!conf.contains("{http_directives}"));
    assert!(!conf.contains("darp.d"));
    for (name, value) in DEFAULT_HTTP_DIRECTIVES {
        assert!(conf.contains(&format!("        {name} {value};")), "{name}");
        assert_eq!(count_directive(&conf, name), 1, "{name}");
    }
}

#[test]
fn nginx_conf_override_replaces_default_instead_of_duplicating() {
    let conf = render_nginx_conf(Some(&map(&[("client_max_body_size", "0")]))).unwrap();
    assert!(conf.contains("        client_max_body_size 0;"));
    assert_eq!(count_directive(&conf, "client_max_body_size"), 1);
}

#[test]
fn nginx_conf_appends_new_directives_inside_http() {
    let conf = render_nginx_conf(Some(&map(&[("proxy_read_timeout", "300s")]))).unwrap();
    let directive = conf.find("        proxy_read_timeout 300s;").expect(&conf);
    assert!(directive > conf.find("http {").unwrap());
    assert!(directive < conf.find("include /etc/nginx/http.d/*.conf;").unwrap());
}

#[test]
fn nginx_conf_rejects_invalid_override() {
    assert!(render_nginx_conf(Some(&map(&[("client_max_body_size", "1m; evil")]))).is_err());
}

// ---------------------------------------------------------------------------
// nginx -t preflight: needs a running engine, like tests/cli_smoke_test.rs.
//   cargo test --test nginx_settings_test -- --ignored
//   DARP_TEST_ENGINE=podman cargo test --test nginx_settings_test -- --ignored
// ---------------------------------------------------------------------------

fn test_engine() -> Engine {
    let kind = match std::env::var("DARP_TEST_ENGINE").as_deref() {
        Ok("podman") => EngineKind::Podman,
        _ => EngineKind::Docker,
    };
    Engine::new(kind, &Config::default()).unwrap()
}

/// Stage configs under target/ (visible to the engine VM) and run `nginx -t`.
fn check(engine: &Engine, nginx_conf: &str, vhost_conf: &str) -> anyhow::Result<()> {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let nginx_path = dir.path().join("nginx.conf");
    let vhost_path = dir.path().join("vhost_container.conf");
    std::fs::write(&nginx_path, nginx_conf).unwrap();
    std::fs::write(&vhost_path, vhost_conf).unwrap();
    engine.test_nginx_config(&nginx_path, &vhost_path)
}

#[test]
#[ignore]
fn preflight_accepts_rendered_config_with_overrides() {
    let engine = test_engine();
    let nginx_conf = render_nginx_conf(Some(&map(&[
        ("client_max_body_size", "0"),
        ("proxy_read_timeout", "300s"),
    ])))
    .unwrap();
    let vhost = build_vhost_container_conf(
        &[planned(
            "a",
            &[("proxy_buffering", "off"), ("client_max_body_size", "50m")],
        )],
        engine.host_gateway(),
    );
    check(&engine, &nginx_conf, &vhost).expect("nginx -t should pass");
}

#[test]
#[ignore]
fn preflight_reports_unknown_directive() {
    let engine = test_engine();
    let nginx_conf = render_nginx_conf(None).unwrap();
    let vhost = build_vhost_container_conf(
        &[planned("a", &[("proxy_read_timout", "300s")])],
        engine.host_gateway(),
    );
    let err = check(&engine, &nginx_conf, &vhost).unwrap_err().to_string();
    assert!(err.contains("unknown directive"), "{err}");
}

// ---------------------------------------------------------------------------
// nginx_maps: what gates the deploy preflight and what doctor validates
// ---------------------------------------------------------------------------

#[test]
fn nginx_maps_is_empty_without_user_settings() {
    let cfg = config(
        r#"{ "domains": { "dom": { "location": "/x", "nginx": {},
             "groups": { ".": { "services": { "a": { "*nginx": null } } } } } } }"#,
    );
    assert!(cfg.nginx_maps().is_empty());
}

#[test]
fn nginx_maps_finds_every_level_with_labels() {
    let labels: Vec<String> = config(CASCADE)
        .nginx_maps()
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    assert_eq!(
        labels,
        vec!["dom", "dom/./own", "dom/./replaced", "dom/big"]
    );

    let global = config(r#"{ "nginx": { "proxy_buffering": "off" } }"#);
    assert_eq!(global.nginx_maps()[0].0, "global");
}
