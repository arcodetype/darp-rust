use std::io::Write;

use crate::config::{self, Config, DarpPaths, Domain};
use crate::engine::{self, Engine};
use crate::os::OsIntegration;

/// Build the contents of `~/.darp/hosts_container` — loopback + host-gateway +
/// one `0.0.0.0 <url>` line per configured service URL.
pub fn build_container_hosts(gateway_ip: &str, gateway_name: &str, url_lines: &[String]) -> String {
    let mut out = String::new();
    out.push_str("127.0.0.1\tlocalhost\n");
    out.push_str("::1\tlocalhost ip6-localhost ip6-loopback\n");
    out.push_str(&format!("{gateway_ip}\t{gateway_name}\n"));
    out.push_str(&url_lines.join(""));
    out
}

/// Collect every host port declared in a `host_portmappings` anywhere in the config
/// (domain/group/service/environment). Debug-port assignment skips these so a debug
/// listener never clashes with a port darp publishes via `-p`. Templated keys (e.g.
/// `{debug_port}`) don't parse as numbers and are ignored.
fn collect_host_portmap_ports(config: &Config) -> std::collections::HashSet<u16> {
    let mut set = std::collections::HashSet::new();
    let mut add = |pm: &Option<std::collections::BTreeMap<String, String>>| {
        if let Some(pm) = pm {
            for host in pm.keys() {
                if let Ok(p) = host.parse::<u16>() {
                    set.insert(p);
                }
            }
        }
    };
    if let Some(domains) = &config.domains {
        for domain in domains.values() {
            add(&domain.host_portmappings);
            if let Some(groups) = &domain.groups {
                for group in groups.values() {
                    add(&group.host_portmappings);
                    if let Some(services) = &group.services {
                        for svc in services.values() {
                            add(&svc.host_portmappings);
                        }
                    }
                }
            }
        }
    }
    if let Some(envs) = &config.environments {
        for env in envs.values() {
            add(&env.host_portmappings);
        }
    }
    set
}

/// Collect every already-assigned `debug_port` from a previously-written portmap so
/// re-deploys keep each service's port stable (assignment order from `read_dir` is
/// otherwise unstable). Removed services free their port on the next deploy.
fn collect_debug_ports(portmap: &serde_json::Value) -> std::collections::HashSet<u16> {
    let mut set = std::collections::HashSet::new();
    if let Some(domains) = portmap.as_object() {
        for group_obj in domains.values() {
            if let Some(groups) = group_obj.as_object() {
                for svc_obj in groups.values() {
                    if let Some(services) = svc_obj.as_object() {
                        for entry in services.values() {
                            if let Some(p) = entry.get("debug_port").and_then(|v| v.as_u64()) {
                                set.insert(p as u16);
                            }
                        }
                    }
                }
            }
        }
    }
    set
}

/// Resolve connection_type by cascading service → group → domain. Environment-layer
/// overrides are not applied at deploy time (deploy does not operate within an environment).
/// Returns None if no layer sets it, in which case callers should treat as "http".
fn resolve_deploy_connection_type(
    domain: &Domain,
    group_name: &str,
    service_name: &str,
) -> Option<String> {
    let group = domain.groups.as_ref().and_then(|g| g.get(group_name));
    let service = group
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(service_name));

    service
        .and_then(|s| s.connection_type.clone())
        .or_else(|| group.and_then(|g| g.connection_type.clone()))
        .or_else(|| domain.connection_type.clone())
}

pub fn cmd_deploy(
    paths: &DarpPaths,
    config: &Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    println!("Deploying Container Development\n");

    // Refresh the embedded nginx.conf on every deploy so fixes to assets/nginx.conf
    // reach the reverse-proxy without a separate `darp install`.
    os.copy_nginx_conf()?;

    let host_gateway = engine.host_gateway();

    let domains = match &config.domains {
        Some(d) if !d.is_empty() => d,
        _ => {
            eprintln!("Please configure a domain.");
            std::process::exit(1);
        }
    };

    let mut hosts_container_lines = Vec::<String>::new();
    let mut portmap = serde_json::Map::new();

    let mut port_number = 50100u16;

    // Assign a stable, unique debug port per service.
    let old_portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));
    let debug_base = config.debug_port_base.unwrap_or(config::DEBUG_PORT_BASE);
    // Ports never handed out: well-known services + any host port darp itself publishes.
    let mut skip_debug_ports = config::well_known_skip_ports();
    skip_debug_ports.extend(collect_host_portmap_ports(config));
    // Seed "reserved" only with persisted ports we'll actually keep (in-range and not
    // skipped) so a kept port isn't reassigned to another service. Persisted ports below
    // the current base (e.g. an old 9003+ range) or now in the skip-list are dropped here
    // and get reassigned into range — auto-migrating on the next deploy.
    let mut reserved_debug_ports: std::collections::HashSet<u16> =
        collect_debug_ports(&old_portmap)
            .into_iter()
            .filter(|p| *p >= debug_base && !skip_debug_ports.contains(p))
            .collect();
    let mut next_debug_port = debug_base;

    // HTTP / WebSocket vhost. The Upgrade + Connection headers are harmless for plain HTTP
    // and allow WebSocket clients (ws://{svc}.{dom}.test) to reach the upstream. The
    // $connection_upgrade variable is defined in assets/nginx.conf.
    let host_proxy_template = r#"server {
    listen 80;
    server_name {url};
    location / {
        proxy_pass http://{host_gateway}:{port}/;
        proxy_set_header Host $host;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $connection_upgrade;
    }
}
"#;

    // Truncate vhost_container.conf at the start of each deploy so we don't
    // keep appending duplicate server blocks.
    std::fs::write(&paths.vhost_container_conf, b"")?;

    for (domain_name, domain) in domains.iter() {
        let location = config::resolve_location(&domain.location)?;
        let mut domain_map = serde_json::Map::new();

        // Collect group names (excluding ".") to know which subdirs are groups vs services
        let group_names: std::collections::HashSet<String> = domain
            .groups
            .as_ref()
            .map(|g| g.keys().filter(|k| k.as_str() != ".").cloned().collect())
            .unwrap_or_default();

        let groups = domain.groups.as_ref();

        // Helper closure to register a service folder
        let register_service = |folder_name: &str,
                                group_name: &str,
                                port_number: &mut u16,
                                next_debug_port: &mut u16,
                                reserved_debug_ports: &mut std::collections::HashSet<u16>,
                                domain_map: &mut serde_json::Map<String, serde_json::Value>,
                                hosts_container_lines: &mut Vec<String>|
         -> anyhow::Result<()> {
            let connection_type = resolve_deploy_connection_type(domain, group_name, folder_name)
                .unwrap_or_else(|| "http".to_string());

            // Reuse this service's previously-assigned debug port when still valid,
            // else assign the next free one (skipping reserved + well-known ports).
            let debug_port = config::choose_debug_port(
                config::portmap_debug_port(&old_portmap, domain_name, group_name, folder_name),
                debug_base,
                &skip_debug_ports,
                reserved_debug_ports,
                next_debug_port,
            );

            // Record port (and type) in portmap.json. run.rs and cmd_urls read this back.
            let mut entry = serde_json::Map::new();
            entry.insert(
                "port".to_string(),
                serde_json::Value::Number((*port_number).into()),
            );
            entry.insert(
                "type".to_string(),
                serde_json::Value::String(connection_type.clone()),
            );
            entry.insert(
                "debug_port".to_string(),
                serde_json::Value::Number(debug_port.into()),
            );
            let group_obj = domain_map
                .entry(group_name.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(group_map) = group_obj.as_object_mut() {
                group_map.insert(folder_name.to_string(), serde_json::Value::Object(entry));
            }

            let url = format!(
                "{folder}.{domain}.test",
                folder = folder_name,
                domain = domain_name
            );

            // Every service gets a hosts entry — HTTP/WS clients reach the reverse proxy
            // on port 80 via this name; TCP clients reach localhost (the hostname is a
            // loopback alias once urls_in_hosts syncs /etc/hosts).
            hosts_container_lines.push(format!("0.0.0.0   {url}\n"));

            match connection_type.as_str() {
                "tcp" => {
                    // No nginx vhost — nginx can't route plain TCP by hostname. The
                    // service is reached as {svc}.{dom}.test:{auto_port} with the port
                    // resolving via the service container's -p {auto_port}:8002 mapping.
                }
                _ => {
                    let vhost = host_proxy_template
                        .replace("{url}", &url)
                        .replace("{host_gateway}", host_gateway)
                        .replace("{port}", &port_number.to_string());

                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&paths.vhost_container_conf)?
                        .write_all(vhost.as_bytes())?;
                }
            }

            *port_number += 1;
            Ok(())
        };

        // Scan "." group: direct children of domain location, excluding group subdirs
        if groups.is_none_or(|g| g.contains_key(".")) {
            if let Ok(entries) = std::fs::read_dir(&location) {
                for entry in entries {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        if !group_names.contains(&folder_name) {
                            register_service(
                                &folder_name,
                                ".",
                                &mut port_number,
                                &mut next_debug_port,
                                &mut reserved_debug_ports,
                                &mut domain_map,
                                &mut hosts_container_lines,
                            )?;
                        }
                    }
                }
            }
        }

        // Scan named groups: subdirs within each group directory
        for group_name in &group_names {
            let group_path = location.join(group_name);
            if let Ok(entries) = std::fs::read_dir(&group_path) {
                for entry in entries {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        register_service(
                            &folder_name,
                            group_name,
                            &mut port_number,
                            &mut next_debug_port,
                            &mut reserved_debug_ports,
                            &mut domain_map,
                            &mut hosts_container_lines,
                        )?;
                    }
                }
            }
        }

        portmap.insert(domain_name.clone(), serde_json::Value::Object(domain_map));
    }

    let gateway_ip =
        match engine::read_container_host_ip(&paths.container_host_ip_path, &engine.kind) {
            Some(ip) => ip,
            None => {
                let ip = engine.probe_host_gateway_ip()?;
                engine::write_container_host_ip(&paths.container_host_ip_path, &engine.kind, &ip)?;
                ip
            }
        };

    let hosts_content =
        build_container_hosts(&gateway_ip, engine.host_gateway(), &hosts_container_lines);
    std::fs::write(&paths.hosts_container_path, hosts_content)?;
    std::fs::write(&paths.portmap_path, serde_json::to_vec_pretty(&portmap)?)?;

    // Report assigned debug ports so each project's .vscode/launch.json "port" can be
    // set (once — ports are persisted). Also available anytime via `darp urls`.
    let mut debug_lines: Vec<(String, u16)> = Vec::new();
    for (domain_name, groups) in portmap.iter() {
        if let Some(groups) = groups.as_object() {
            for (group_name, services) in groups {
                if let Some(services) = services.as_object() {
                    for (service_name, entry) in services {
                        if let Some(p) = entry.get("debug_port").and_then(|v| v.as_u64()) {
                            let label = if group_name == "." {
                                format!("{}.{}", service_name, domain_name)
                            } else {
                                format!("{}.{}.{}", service_name, group_name, domain_name)
                            };
                            debug_lines.push((label, p as u16));
                        }
                    }
                }
            }
        }
    }
    if !debug_lines.is_empty() {
        debug_lines.sort();
        println!("\nDebug ports (set as \"port\" in each project's .vscode/launch.json):");
        for (label, port) in debug_lines {
            println!("  {label} → {port}");
        }
    }

    // Restart reverse proxy and stop darp_* containers
    engine.restart_reverse_proxy(paths)?;
    engine.start_darp_masq(paths)?;
    engine.stop_running_darps()?;

    // Optionally sync /etc/hosts if urls_in_hosts is enabled
    if config.urls_in_hosts.unwrap_or(false) {
        let os = OsIntegration::new(paths, config, &engine.kind);
        os.sync_system_hosts(&hosts_container_lines)?;

        if config.wsl.unwrap_or(false) {
            os.sync_windows_hosts(&hosts_container_lines)?;
        }
    }

    Ok(())
}
