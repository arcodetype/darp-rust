use std::io::Write;

use crate::config::{self, Config, DarpPaths};
use crate::engine::Engine;
use crate::os::OsIntegration;

pub fn cmd_deploy(
    paths: &DarpPaths,
    config: &Config,
    _os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    println!("Deploying Container Development\n");

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

    // NOTE: single braces now; we are NOT using `format!`, just `.replace()`
    let host_proxy_template = r#"server {
    listen 80;
    server_name {url};
    location / {
        proxy_pass http://{host_gateway}:{port}/;
        proxy_set_header Host $host;
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
                                domain_map: &mut serde_json::Map<String, serde_json::Value>,
                                hosts_container_lines: &mut Vec<String>|
         -> anyhow::Result<()> {
            let group_obj = domain_map
                .entry(group_name.to_string())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(group_map) = group_obj.as_object_mut() {
                group_map.insert(
                    folder_name.to_string(),
                    serde_json::Value::Number((*port_number).into()),
                );
            }

            let url = format!(
                "{folder}.{domain}.test",
                folder = folder_name,
                domain = domain_name
            );

            hosts_container_lines.push(format!("0.0.0.0   {url}\n"));

            let vhost = host_proxy_template
                .replace("{url}", &url)
                .replace("{host_gateway}", host_gateway)
                .replace("{port}", &port_number.to_string());

            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.vhost_container_conf)?
                .write_all(vhost.as_bytes())?;

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
                            &mut domain_map,
                            &mut hosts_container_lines,
                        )?;
                    }
                }
            }
        }

        portmap.insert(domain_name.clone(), serde_json::Value::Object(domain_map));
    }

    std::fs::write(&paths.hosts_container_path, hosts_container_lines.join(""))?;
    std::fs::write(&paths.portmap_path, serde_json::to_vec_pretty(&portmap)?)?;

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
