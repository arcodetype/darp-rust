//! `dns_port`: moving darp-masq's DNS server off host port 53.
//!
//! Covers the published ports, the `/etc/resolver/test` contents, the checks deploy
//! uses to decide whether to touch either, the warning for a darp-masq that won't
//! start, and `darp config set|rm dns-port`. The default (unset) cases pin two
//! things: a stock install — darp-masq on 53, no `port` line — is left alone, and the
//! undo that makes `config rm dns-port` work: a darp-masq off 53 is replaced and a
//! `port` line removed, whoever put them there.

use std::path::{Path, PathBuf};
use std::process::Command;

use darp::commands::plan_deployment;
use darp::config::{Config, DEBUG_PORT_BASE, DEFAULT_DNS_PORT, DarpPaths};
use darp::engine::{
    darp_masq_bind_label, darp_masq_holds_target_port, darp_masq_move_warning,
    darp_masq_port_conflict_hint, darp_masq_ports_match, darp_masq_publish_args,
    darp_masq_start_warning,
};
use darp::os::{
    ensure_resolver_step, render_resolver_conf, resolver_conf_needs_sync, resolver_conf_port,
    resolver_conf_synced,
};

/// Docker 24's stderr when darp-masq's default publish finds 53 taken (exit 125).
const DOCKER_53_BIND_ERROR: &str = "docker: Error response from daemon: driver failed programming \
     external connectivity on endpoint darp-masq (def67f3d): Bind for 0.0.0.0:53 failed: port is \
     already allocated.\n";

/// The same failure for a `dns_port` that is already taken.
const DOCKER_5300_BIND_ERROR: &str = "docker: Error response from daemon: driver failed programming \
     external connectivity on endpoint darp-masq (d285ea9d): Bind for 127.0.0.1:5300 failed: port \
     is already allocated.\n";

/// `docker port darp-masq` for a darp-masq darp started with its default `53:53`.
const DOCKER_DEFAULT_PORTS: &str = "53/tcp -> 0.0.0.0:53\n\
                                    53/tcp -> [::]:53\n\
                                    53/udp -> 0.0.0.0:53\n\
                                    53/udp -> [::]:53\n";

/// `docker port darp-masq` once darp-masq is published on 127.0.0.1:5300.
const DOCKER_5300_PORTS: &str = "53/tcp -> 127.0.0.1:5300\n53/udp -> 127.0.0.1:5300\n";

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

fn run_darp(bin: &Path, darp_root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(bin)
        .env("DARP_ROOT", darp_root)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run darp {:?}: {}", args, e))
}

fn stored_dns_port(darp_root: &Path) -> Option<u16> {
    Config::load(&darp_root.join("config.json"))
        .unwrap()
        .dns_port
}

// ---- Config ----

#[test]
fn custom_dns_port_is_none_unless_moved_off_53() {
    let config = |dns_port| Config {
        dns_port,
        ..Config::default()
    };

    assert_eq!(config(None).custom_dns_port(), None);
    assert_eq!(config(Some(DEFAULT_DNS_PORT)).custom_dns_port(), None);
    assert_eq!(config(Some(5300)).custom_dns_port(), Some(5300));
}

#[test]
fn dns_port_round_trips_and_stays_out_of_a_default_config() {
    let config: Config = serde_json::from_str(r#"{"dns_port": 5300}"#).unwrap();
    assert_eq!(config.dns_port, Some(5300));

    let json = serde_json::to_string(&Config::default()).unwrap();
    assert!(!json.contains("dns_port"));
}

#[test]
fn dns_port_merges_from_a_pre_config_and_the_leaf_overrides_it() {
    let root = tempfile::tempdir().unwrap();
    let team = root.path().join("team.json");
    std::fs::write(&team, r#"{"dns_port": 5300}"#).unwrap();

    let leaf = root.path().join("config.json");
    let pre_config = format!(r#""pre_config": [{{"location": "{}"}}]"#, team.display());

    std::fs::write(&leaf, format!("{{{pre_config}}}")).unwrap();
    let merged = Config::load_merged(&leaf).unwrap();
    assert_eq!(merged.custom_dns_port(), Some(5300));

    std::fs::write(&leaf, format!(r#"{{{pre_config}, "dns_port": 53}}"#)).unwrap();
    let merged = Config::load_merged(&leaf).unwrap();
    assert_eq!(merged.custom_dns_port(), None);
}

// ---- darp-masq ports ----

#[test]
fn default_publish_args_are_unchanged() {
    assert_eq!(darp_masq_publish_args(None), ["53:53/udp", "53:53/tcp"]);
}

#[test]
fn custom_port_publishes_on_loopback_only() {
    assert_eq!(
        darp_masq_publish_args(Some(5300)),
        ["127.0.0.1:5300:53/udp", "127.0.0.1:5300:53/tcp"]
    );
}

#[test]
fn bind_label_names_where_darp_masq_is_published() {
    assert_eq!(darp_masq_bind_label(None), "port 53");
    assert_eq!(darp_masq_bind_label(Some(5300)), "127.0.0.1:5300");
}

#[test]
fn darp_started_default_container_matches_the_default() {
    assert!(darp_masq_ports_match(DOCKER_DEFAULT_PORTS, None));
}

#[test]
fn default_tolerates_any_host_ip_on_port_53() {
    assert!(darp_masq_ports_match(
        "53/udp -> 127.0.0.1:53\n53/tcp -> 127.0.0.1:53\n",
        None
    ));
}

#[test]
fn default_leaves_alone_a_container_with_no_readable_bindings() {
    assert!(darp_masq_ports_match("", None));
}

#[test]
fn default_replaces_a_container_moved_off_53() {
    assert!(!darp_masq_ports_match(DOCKER_5300_PORTS, None));
}

#[test]
fn custom_port_matches_loopback_udp_and_tcp() {
    assert!(darp_masq_ports_match(DOCKER_5300_PORTS, Some(5300)));
}

#[test]
fn custom_port_replaces_the_default_container() {
    assert!(!darp_masq_ports_match(DOCKER_DEFAULT_PORTS, Some(5300)));
}

#[test]
fn custom_port_replaces_a_different_port() {
    assert!(!darp_masq_ports_match(DOCKER_5300_PORTS, Some(5353)));
}

#[test]
fn custom_port_needs_both_protocols() {
    assert!(!darp_masq_ports_match(
        "53/udp -> 127.0.0.1:5300\n",
        Some(5300)
    ));
}

#[test]
fn custom_port_rejects_an_extra_binding() {
    let ports = format!("{DOCKER_5300_PORTS}53/udp -> 0.0.0.0:5300\n");
    assert!(!darp_masq_ports_match(&ports, Some(5300)));
}

#[test]
fn custom_port_rejects_all_interfaces() {
    assert!(!darp_masq_ports_match(
        "53/udp -> 0.0.0.0:5300\n53/tcp -> 0.0.0.0:5300\n",
        Some(5300)
    ));
}

#[test]
fn custom_port_replaces_a_container_with_no_readable_bindings() {
    assert!(!darp_masq_ports_match("", Some(5300)));
}

// ---- darp-masq start failures ----

#[test]
fn start_warning_names_where_darp_masq_failed_to_start() {
    assert!(darp_masq_start_warning(DOCKER_53_BIND_ERROR, None).contains("port 53"));
    assert!(darp_masq_start_warning(DOCKER_5300_BIND_ERROR, Some(5300)).contains("127.0.0.1:5300"));
}

#[test]
fn start_warning_carries_the_engine_error() {
    let warning = darp_masq_start_warning(DOCKER_53_BIND_ERROR, None);
    assert!(warning.contains("Bind for 0.0.0.0:53 failed: port is already allocated."));
}

#[test]
fn port_53_start_failure_hints_at_dns_port() {
    let warning = darp_masq_start_warning(DOCKER_53_BIND_ERROR, None);
    assert!(warning.contains("darp config set dns-port <port>"));
}

#[test]
fn custom_port_start_failure_has_no_dns_port_hint() {
    let warning = darp_masq_start_warning(DOCKER_5300_BIND_ERROR, Some(5300));
    assert!(!warning.contains("darp config set dns-port"));
}

#[test]
fn start_warning_survives_empty_stderr() {
    let warning = darp_masq_start_warning("  \n", Some(5300));
    assert!(warning.contains("no error output"));
}

// ---- /etc/resolver/test ----

#[cfg(unix)]
#[test]
fn resolver_step_that_succeeded_is_ok() {
    use std::os::unix::process::ExitStatusExt;
    let ok = std::process::ExitStatus::from_raw(0);
    assert!(ensure_resolver_step(ok, "/etc/resolver/test", "sudo tee").is_ok());
}

#[cfg(unix)]
#[test]
fn resolver_step_that_failed_names_the_file_and_the_step() {
    use std::os::unix::process::ExitStatusExt;
    let failed = std::process::ExitStatus::from_raw(1 << 8);
    let err = ensure_resolver_step(failed, "/etc/resolver/test", "sudo tee")
        .unwrap_err()
        .to_string();
    assert!(err.contains("/etc/resolver/test"), "{err}");
    assert!(err.contains("sudo tee"), "{err}");
}

#[test]
fn default_resolver_conf_is_unchanged() {
    assert_eq!(render_resolver_conf(None), "nameserver 127.0.0.1\n");
}

#[test]
fn custom_resolver_conf_names_the_port() {
    assert_eq!(
        render_resolver_conf(Some(5300)),
        "nameserver 127.0.0.1\nport 5300\n"
    );
}

#[test]
fn resolver_conf_port_reads_the_port_line() {
    assert_eq!(resolver_conf_port("nameserver 127.0.0.1\n"), None);
    assert_eq!(
        resolver_conf_port("nameserver 127.0.0.1\nport 5300\n"),
        Some(5300)
    );
    assert_eq!(
        resolver_conf_port("port   5300\nnameserver 127.0.0.1"),
        Some(5300)
    );
    assert_eq!(resolver_conf_port("nameserver 127.0.0.1\nport x\n"), None);
}

#[test]
fn deploy_never_touches_a_default_resolver_conf() {
    let default = render_resolver_conf(None);
    assert!(!resolver_conf_needs_sync(Some(&default), None));
    assert!(!resolver_conf_needs_sync(None, None));
    assert!(!resolver_conf_needs_sync(
        Some("nameserver 10.0.0.1\n"),
        None
    ));
}

#[test]
fn deploy_undoes_a_port_line_once_dns_port_is_unset() {
    let moved = render_resolver_conf(Some(5300));
    assert!(resolver_conf_needs_sync(Some(&moved), None));
}

#[test]
fn deploy_writes_the_port_line_for_a_custom_port() {
    let default = render_resolver_conf(None);
    let other = render_resolver_conf(Some(5353));
    assert!(resolver_conf_needs_sync(Some(&default), Some(5300)));
    assert!(resolver_conf_needs_sync(Some(&other), Some(5300)));
    assert!(resolver_conf_needs_sync(None, Some(5300)));
}

#[test]
fn deploy_leaves_a_resolver_conf_that_already_matches() {
    let moved = render_resolver_conf(Some(5300));
    assert!(!resolver_conf_needs_sync(Some(&moved), Some(5300)));
}

// ---- darp config set|rm dns-port ----

#[test]
fn config_set_and_rm_dns_port() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();

    let out = run_darp(&bin, root.path(), &["config", "set", "dns-port", "5300"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stored_dns_port(root.path()), Some(5300));
    assert!(String::from_utf8_lossy(&out.stdout).contains("127.0.0.1:5300"));

    let out = run_darp(&bin, root.path(), &["config", "rm", "dns-port"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stored_dns_port(root.path()), None);
}

#[test]
fn config_set_dns_port_rejects_invalid_ports() {
    let bin = darp_bin();
    let root = tempfile::tempdir().unwrap();

    for bad in ["0", "65536", "abc", "-1"] {
        let out = run_darp(&bin, root.path(), &["config", "set", "dns-port", bad]);
        assert!(!out.status.success(), "dns-port {bad} was accepted");
    }
    assert_eq!(stored_dns_port(root.path()), None);
}

// ---- Validation ----

#[test]
fn a_merged_dns_port_of_zero_is_rejected_but_can_still_be_removed() {
    let root = tempfile::tempdir().unwrap();
    let leaf = root.path().join("config.json");
    std::fs::write(&leaf, r#"{"dns_port": 0}"#).unwrap();

    let err = Config::load_merged(&leaf).unwrap_err().to_string();
    assert!(err.contains("dns_port"), "{err}");

    // `darp config` goes through Config::load, so the way out has to keep working.
    let bin = darp_bin();
    let out = run_darp(&bin, root.path(), &["config", "rm", "dns-port"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(Config::load_merged(&leaf).unwrap().dns_port.is_none());
}

// ---- Moving a running darp-masq ----

#[test]
fn a_replacement_can_start_beside_a_darp_masq_on_another_port() {
    assert!(!darp_masq_holds_target_port(
        DOCKER_DEFAULT_PORTS,
        Some(5300)
    ));
    assert!(!darp_masq_holds_target_port(DOCKER_5300_PORTS, None));
    assert!(!darp_masq_holds_target_port("", Some(5300)));
}

#[test]
fn a_replacement_cannot_start_beside_a_darp_masq_on_the_target_port() {
    let all_interfaces = "53/udp -> 0.0.0.0:5300\n53/tcp -> [::]:5300\n";
    assert!(darp_masq_holds_target_port(all_interfaces, Some(5300)));
    // The default has to match the port exactly, not by suffix: 5353 isn't 53.
    assert!(!darp_masq_holds_target_port(
        "53/udp -> 127.0.0.1:5353\n",
        None
    ));
}

#[test]
fn move_warning_says_nothing_was_lost_and_carries_the_engine_error() {
    let warning = darp_masq_move_warning(DOCKER_53_BIND_ERROR, None);
    assert!(warning.contains("left as they were"), "{warning}");
    assert!(!warning.contains("won't resolve"), "{warning}");
    assert!(warning.contains("port is already allocated"), "{warning}");
    assert!(warning.contains("darp config set dns-port"), "{warning}");
}

// ---- Which start failures suggest dns-port ----

#[test]
fn podman_and_docker_desktop_bind_failures_hint_at_dns_port() {
    for stderr in [
        "Error: rootlessport listen udp 0.0.0.0:53: bind: address already in use",
        "docker: Error response from daemon: Ports are not available: exposing port UDP 0.0.0.0:53 -> 0.0.0.0:0: listen udp 0.0.0.0:53: bind: address already in use.",
    ] {
        let warning = darp_masq_start_warning(stderr, None);
        assert!(warning.contains("darp config set dns-port"), "{warning}");
    }
}

#[test]
fn a_name_conflict_does_not_hint_at_dns_port() {
    let stderr = "docker: Error response from daemon: Conflict. The container name \"/darp-masq\" is already in use by container \"abc\".";
    let warning = darp_masq_start_warning(stderr, None);
    assert!(!warning.contains("darp config set dns-port"), "{warning}");
}

// ---- Keeping the rest of the resolver file ----

#[test]
fn a_custom_port_sync_ignores_other_options() {
    let file = "nameserver 127.0.0.1\nport 5300\ntimeout 2\n";
    assert!(!resolver_conf_needs_sync(Some(file), Some(5300)));
}

#[test]
fn a_custom_port_sync_requires_a_loopback_nameserver() {
    assert!(resolver_conf_needs_sync(
        Some("nameserver 10.0.0.1\nport 5300\n"),
        Some(5300)
    ));
    assert!(resolver_conf_needs_sync(Some("port 5300\n"), Some(5300)));
}

#[test]
fn syncing_a_custom_port_keeps_other_options() {
    assert_eq!(
        resolver_conf_synced(Some("nameserver 10.0.0.1\ntimeout 2\n"), Some(5300)),
        "nameserver 127.0.0.1\nport 5300\ntimeout 2\n"
    );
}

#[test]
fn syncing_a_new_port_replaces_the_old_one() {
    assert_eq!(
        resolver_conf_synced(Some("nameserver 127.0.0.1\nport 5353\n"), Some(5300)),
        "nameserver 127.0.0.1\nport 5300\n"
    );
}

#[test]
fn syncing_back_to_the_default_only_removes_the_port_line() {
    assert_eq!(
        resolver_conf_synced(Some("nameserver 10.0.0.1\nport 5300\ntimeout 2\n"), None),
        "nameserver 10.0.0.1\ntimeout 2\n"
    );
}

#[test]
fn syncing_always_leaves_a_nameserver() {
    assert_eq!(
        resolver_conf_synced(Some("port 5300\n"), None),
        "nameserver 127.0.0.1\n"
    );
    assert_eq!(
        resolver_conf_synced(None, Some(5300)),
        "nameserver 127.0.0.1\nport 5300\n"
    );
}

#[test]
fn a_synced_file_needs_no_further_sync() {
    let synced = resolver_conf_synced(Some("nameserver 10.0.0.1\ntimeout 2\n"), Some(5300));
    assert!(!resolver_conf_needs_sync(Some(&synced), Some(5300)));

    let undone = resolver_conf_synced(Some(&synced), None);
    assert!(!resolver_conf_needs_sync(Some(&undone), None));
}

// ---- Debug ports ----

#[test]
fn debug_ports_are_never_assigned_darp_masqs_port() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("darp");
    let domain = tmp.path().join("projects");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(domain.join("api")).unwrap();
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
    let config = |dns_port: Option<u16>| -> Config {
        let mut config: Config = serde_json::from_value(serde_json::json!({
            "domains": {"d": {"location": domain.to_string_lossy()}}
        }))
        .unwrap();
        config.dns_port = dns_port;
        config
    };
    let first_debug_port = DEBUG_PORT_BASE;

    let services = plan_deployment(&paths, &config(None)).unwrap();
    assert_eq!(services[0].debug_port, first_debug_port);

    let services = plan_deployment(&paths, &config(Some(first_debug_port))).unwrap();
    assert_ne!(services[0].debug_port, first_debug_port);
}

// ---- Out-of-range ports ----

#[test]
fn an_out_of_range_dns_port_is_an_error_and_never_overwrites_the_config() {
    let bin = darp_bin();
    for bad in ["70000", "-1", "\"5300\""] {
        let root = tempfile::tempdir().unwrap();
        let config = root.path().join("config.json");
        let original = format!(r#"{{"dns_port": {bad}, "podman_machine": "keep-me"}}"#);
        std::fs::write(&config, &original).unwrap();

        let err = Config::load_merged(&config).unwrap_err().to_string();
        assert!(err.contains("dns_port"), "{err}");

        let out = run_darp(
            &bin,
            root.path(),
            &["config", "set", "urls-in-hosts", "true"],
        );
        assert!(!out.status.success(), "dns_port {bad} was accepted");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("dns_port"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(std::fs::read_to_string(&config).unwrap(), original);
    }
}

#[test]
fn an_out_of_range_dns_port_from_a_pre_config_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    let team = root.path().join("team.json");
    std::fs::write(&team, r#"{"dns_port": 70000}"#).unwrap();
    let leaf = root.path().join("config.json");
    std::fs::write(
        &leaf,
        format!(
            r#"{{"pre_config": [{{"location": "{}"}}]}}"#,
            team.display()
        ),
    )
    .unwrap();

    let err = Config::load_merged(&leaf).unwrap_err().to_string();
    assert!(err.contains("dns_port must be a port"), "{err}");
}

// ---- Warning text ----

#[test]
fn port_conflict_hint_matches_how_the_system_resolves_test_names() {
    let macos = darp_masq_port_conflict_hint(true);
    assert!(macos.contains("darp config set dns-port"), "{macos}");
    assert!(!macos.contains("urls-in-hosts"), "{macos}");

    let other = darp_masq_port_conflict_hint(false);
    assert!(other.contains("darp config set dns-port"), "{other}");
    assert!(
        other.contains("darp config set urls-in-hosts true"),
        "{other}"
    );
}

#[test]
fn warnings_drop_dockers_help_pointer() {
    let stderr = "docker: Error response from daemon: port is already allocated\n\nRun 'docker run --help' for more information\n";
    let warning = darp_masq_start_warning(stderr, Some(5300));
    assert!(warning.contains("port is already allocated"), "{warning}");
    assert!(!warning.contains("--help"), "{warning}");
    assert!(!warning.contains("  \n"), "{warning:?}");
}
