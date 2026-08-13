use crate::alias::{self, AliasValidationError};
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

/// Collect a service's extra URL aliases exactly as configured.
///
/// Service-level only — an alias names one specific project, so `urls` does not
/// cascade through group/domain/environment the way the resolved settings do.
/// Values are returned raw: trimming, lowercasing, and rejection all happen in
/// [`validate_plan_aliases`], so a malformed alias is reported with the text the
/// engineer actually wrote instead of being silently dropped or reshaped here.
fn resolve_deploy_urls(domain: &Domain, group_name: &str, service_name: &str) -> Vec<String> {
    domain
        .groups
        .as_ref()
        .and_then(|g| g.get(group_name))
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(service_name))
        .and_then(|s| s.urls.clone())
        .unwrap_or_default()
}

/// Render the nginx vhost that fronts one hostname for a service.
///
/// The Upgrade + Connection headers are harmless for plain HTTP and let WebSocket
/// clients (`ws://{svc}.{dom}.test`, and Vite HMR arriving on an alias hostname)
/// reach the upstream. The `$connection_upgrade` variable is defined in
/// `assets/nginx.conf`.
pub fn build_host_proxy_vhost(url: &str, host_gateway: &str, port: u16) -> String {
    const TEMPLATE: &str = r#"server {
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

    TEMPLATE
        .replace("{url}", url)
        .replace("{host_gateway}", host_gateway)
        .replace("{port}", &port.to_string())
}

/// One service discovered on disk, carrying everything the artifact writers need.
///
/// Deploy runs as discover → validate → generate: the whole plan is built and
/// checked before darp truncates `vhost_container.conf` or rewrites any other
/// artifact, so a rejected config leaves the previous deployment intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedService {
    pub domain: String,
    pub group: String,
    pub service: String,
    pub connection_type: String,
    pub proxy_port: u16,
    pub debug_port: u16,
    pub canonical_hostname: String,
    /// Raw config values until [`validate_plan_aliases`] replaces them with
    /// normalized, deduplicated hostnames.
    pub aliases: Vec<String>,
}

impl PlannedService {
    /// `domain/group/service` — the identity every consolidated error and warning
    /// prints, matching how groups appear in the config (`.` for the root group).
    pub fn label(&self) -> String {
        format!("{}/{}/{}", self.domain, self.group, self.service)
    }

    /// True when nginx routes this service by hostname. TCP services get no vhost —
    /// nginx can't route plain TCP by name — so they never participate in hostname
    /// collision checks.
    pub fn is_host_routed(&self) -> bool {
        self.connection_type != "tcp"
    }

    /// Canonical hostname in comparison form. Folder names are not hostname-validated
    /// — darp has always passed them through as-is — so only case and a trailing dot
    /// are normalized here, which is what DNS-correct comparison needs.
    fn canonical_key(&self) -> String {
        alias::normalize_for_comparison(&self.canonical_hostname)
    }

    /// Every hostname that reaches this service: canonical first, then aliases in
    /// configured order.
    fn hostnames(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(1 + self.aliases.len());
        out.push(self.canonical_hostname.clone());
        out.extend(self.aliases.iter().cloned());
        out
    }
}

/// Discover every service on disk and assign its ports, without touching a single
/// deployment artifact.
///
/// Reads the previous `portmap.json` so each service keeps its persisted debug
/// port; everything else is derived from the config and the filesystem.
pub fn plan_deployment(paths: &DarpPaths, config: &Config) -> anyhow::Result<Vec<PlannedService>> {
    let domains = match &config.domains {
        Some(d) if !d.is_empty() => d,
        _ => {
            eprintln!("Please configure a domain.");
            std::process::exit(1);
        }
    };

    let mut planned = Vec::<PlannedService>::new();
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

    for (domain_name, domain) in domains.iter() {
        let location = config::resolve_location(&domain.location)?;

        // Collect group names (excluding ".") to know which subdirs are groups vs services
        let group_names: std::collections::HashSet<String> = domain
            .groups
            .as_ref()
            .map(|g| g.keys().filter(|k| k.as_str() != ".").cloned().collect())
            .unwrap_or_default();

        let groups = domain.groups.as_ref();

        // Helper closure to plan one service folder
        let mut register_service =
            |folder_name: &str, group_name: &str, out: &mut Vec<PlannedService>| {
                let connection_type =
                    resolve_deploy_connection_type(domain, group_name, folder_name)
                        .unwrap_or_else(|| "http".to_string());

                // Reuse this service's previously-assigned debug port when still valid,
                // else assign the next free one (skipping reserved + well-known ports).
                let debug_port = config::choose_debug_port(
                    config::portmap_debug_port(&old_portmap, domain_name, group_name, folder_name),
                    debug_base,
                    &skip_debug_ports,
                    &mut reserved_debug_ports,
                    &mut next_debug_port,
                );

                let proxy_port = port_number;
                port_number += 1;

                out.push(PlannedService {
                    domain: domain_name.clone(),
                    group: group_name.to_string(),
                    service: folder_name.to_string(),
                    connection_type,
                    proxy_port,
                    debug_port,
                    canonical_hostname: format!(
                        "{folder}.{domain}.test",
                        folder = folder_name,
                        domain = domain_name
                    ),
                    // Extra hostnames that should reach this same service.
                    aliases: resolve_deploy_urls(domain, group_name, folder_name),
                });
            };

        // Scan "." group: direct children of domain location, excluding group subdirs
        if groups.is_none_or(|g| g.contains_key(".")) {
            if let Ok(entries) = std::fs::read_dir(&location) {
                for entry in entries {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        if !group_names.contains(&folder_name) {
                            register_service(&folder_name, ".", &mut planned);
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
                        register_service(&folder_name, group_name, &mut planned);
                    }
                }
            }
        }
    }

    Ok(planned)
}

/// Validate and normalize every configured alias in the plan, in place.
///
/// Aliases land verbatim in nginx `server_name` directives, hosts files, and
/// `portmap.json`, so a value carrying a scheme, port, path, or nginx delimiter is
/// rejected before anything is written. TCP services are validated too — they get
/// no vhost, but they do get hosts entries.
///
/// Every failure across the whole deployment is collected and reported together,
/// ordered by service then original alias text, so one `darp deploy` surfaces the
/// full list rather than one problem per run.
///
/// Surviving aliases are also deduplicated within the service — repeats, and any
/// alias equal to the service's own canonical URL, collapse to one entry in
/// first-seen order. They all point at the same upstream, so this is not a
/// collision; `pre_config` array merging routinely produces such repeats when a
/// personal config re-lists an alias the team config already set.
pub fn validate_plan_aliases(services: &mut [PlannedService]) -> anyhow::Result<()> {
    let mut failures: Vec<(String, AliasValidationError)> = Vec::new();

    for svc in services.iter_mut() {
        let mut normalized = Vec::with_capacity(svc.aliases.len());
        let mut seen = std::collections::HashSet::new();
        seen.insert(svc.canonical_key());
        for raw in &svc.aliases {
            match alias::validate_and_normalize_alias(raw) {
                Ok(hostname) => {
                    if seen.insert(hostname.clone()) {
                        normalized.push(hostname);
                    }
                }
                Err(e) => failures.push((svc.label(), e)),
            }
        }
        svc.aliases = normalized;
    }

    if failures.is_empty() {
        return Ok(());
    }

    failures.sort_by(|(a_label, a_err), (b_label, b_err)| {
        a_label.cmp(b_label).then(a_err.input.cmp(&b_err.input))
    });

    let mut msg =
        String::from("Cannot deploy because some URL aliases are not valid hostnames:\n\n");
    let mut current: Option<&str> = None;
    for (label, err) in &failures {
        if current != Some(label.as_str()) {
            msg.push_str(&format!("  {label}\n"));
            current = Some(label.as_str());
        }
        msg.push_str(&format!("    {err}\n"));
    }
    msg.push_str(
        "\nConfigure hostname-only values such as `local.zoo.org` and run `darp deploy` again.\n\
         \nNo deployment artifacts were changed.",
    );

    anyhow::bail!(msg)
}

/// Whether a registered hostname is a service's canonical URL or one of its aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostnameSource {
    Canonical,
    Alias,
}

impl HostnameSource {
    fn describe(self) -> &'static str {
        match self {
            Self::Canonical => "canonical URL",
            Self::Alias => "alias",
        }
    }
}

/// One claim on an HTTP/WebSocket hostname.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HostnameOwner {
    domain: String,
    group: String,
    service: String,
    source: HostnameSource,
    connection_type: String,
}

/// Reject any HTTP/WebSocket hostname claimed by more than one service.
///
/// nginx routes by `server_name`. When two server blocks declare the same name it
/// ignores the second and sends every request for that hostname to whichever
/// service was registered first — and registration order follows `read_dir`, which
/// is unstable, so the winner can change between deploys. Silently routing a
/// hostname to the wrong project is worse than refusing to deploy.
///
/// Catches alias-to-alias, alias-to-canonical, and canonical-to-canonical claims.
/// The last case includes two same-named folders in different groups, because a
/// canonical hostname is `{folder}.{domain}.test` with no group component.
///
/// TCP services are exempt: they get no vhost, and their assigned ports already
/// distinguish them, so sharing a hostname with each other or with an HTTP/WebSocket
/// service is safe.
pub fn detect_hostname_collisions(services: &[PlannedService]) -> anyhow::Result<()> {
    // BTreeMap keys the report by hostname in a stable order regardless of the
    // discovery order the collisions were found in.
    let mut registry: std::collections::BTreeMap<String, Vec<HostnameOwner>> =
        std::collections::BTreeMap::new();

    for svc in services.iter().filter(|s| s.is_host_routed()) {
        let mut claim = |hostname: String, source: HostnameSource| {
            registry.entry(hostname).or_default().push(HostnameOwner {
                domain: svc.domain.clone(),
                group: svc.group.clone(),
                service: svc.service.clone(),
                source,
                connection_type: svc.connection_type.clone(),
            });
        };

        claim(svc.canonical_key(), HostnameSource::Canonical);
        for a in &svc.aliases {
            claim(alias::normalize_for_comparison(a), HostnameSource::Alias);
        }
    }

    let mut msg = String::from(
        "Cannot deploy because some HTTP/WebSocket hostnames are assigned to multiple services:\n\n",
    );
    let mut found = false;
    for (hostname, owners) in registry.iter_mut() {
        if owners.len() < 2 {
            continue;
        }
        found = true;
        owners.sort();
        msg.push_str(&format!("  {hostname}\n"));
        for owner in owners.iter() {
            msg.push_str(&format!(
                "    {} for {}/{}/{} ({})\n",
                owner.source.describe(),
                owner.domain,
                owner.group,
                owner.service,
                owner.connection_type
            ));
        }
        msg.push('\n');
    }

    if !found {
        return Ok(());
    }

    msg.push_str(
        "Each HTTP/WebSocket hostname must map to exactly one service. Remove or rename the\n\
         conflicting aliases and run `darp deploy` again.\n\
         \nNo deployment artifacts were changed.",
    );

    anyhow::bail!(msg)
}

/// An alias darp generated a vhost for but did not make resolvable itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnresolvedAlias {
    pub domain: String,
    pub group: String,
    pub service: String,
    pub hostname: String,
}

/// Collect the aliases darp has generated proxy config for without providing host
/// name resolution.
///
/// darp's dnsmasq config is `address=/.test/127.0.0.1`, so a `.test` alias resolves
/// on its own. Anything else needs either `urls_in_hosts: true`, which lets darp
/// manage a hosts-file entry, or a DNS source the engineer runs themselves. Without
/// one of those, the alias gets an nginx vhost and a `darp urls` line but cannot be
/// reached from a host browser.
///
/// Returns nothing when `urls_in_hosts` is on — darp handles resolution then — and
/// covers TCP services too, whose clients need the name to resolve just as much.
pub fn collect_unresolved_aliases(
    services: &[PlannedService],
    urls_in_hosts: bool,
) -> Vec<UnresolvedAlias> {
    if urls_in_hosts {
        return Vec::new();
    }

    let mut entries: Vec<UnresolvedAlias> = services
        .iter()
        .flat_map(|svc| {
            svc.aliases
                .iter()
                .filter(|a| !alias::is_dnsmasq_wildcard_hostname(a))
                .map(|a| UnresolvedAlias {
                    domain: svc.domain.clone(),
                    group: svc.group.clone(),
                    service: svc.service.clone(),
                    hostname: a.clone(),
                })
        })
        .collect();

    // Discovery order comes from read_dir, so sort before reporting.
    entries.sort();
    entries.dedup();
    entries
}

/// Render one consolidated warning for every alias darp did not make resolvable, or
/// `None` when there is nothing to report.
///
/// Deliberately does not claim the names cannot resolve: an engineer may already run
/// their own DNS pointing them at 127.0.0.1, which is a perfectly good answer. It
/// says what darp did and did not configure, and names the one setting that changes
/// that.
pub fn render_unresolved_alias_warning(entries: &[UnresolvedAlias]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }

    let mut msg = String::from(
        "Warning: darp did not create host DNS mappings for these non-.test aliases:\n\n",
    );
    let mut current: Option<(&str, &str, &str)> = None;
    for entry in entries {
        let identity = (
            entry.domain.as_str(),
            entry.group.as_str(),
            entry.service.as_str(),
        );
        if current != Some(identity) {
            msg.push_str(&format!(
                "  {}/{}/{}\n",
                entry.domain, entry.group, entry.service
            ));
            current = Some(identity);
        }
        msg.push_str(&format!("    {}\n", entry.hostname));
    }
    msg.push_str(
        "\ndarp's dnsmasq configuration resolves .test names only. To let darp manage\n\
         resolution for these aliases, enable hosts-file synchronization:\n\
         \n  darp config set urls-in-hosts true\n\
         \nThe reverse-proxy configuration was generated, but these aliases require either\n\
         hosts-file synchronization or another DNS source that resolves them to 127.0.0.1.",
    );

    Some(msg)
}

/// Build the full `vhost_container.conf` — one nginx server block per hostname of
/// every host-routed service, all proxying to that service's upstream port.
///
/// Written in one shot rather than appended per service so a failure part-way
/// through planning can't leave a half-populated config behind.
pub fn build_vhost_container_conf(services: &[PlannedService], host_gateway: &str) -> String {
    let mut out = String::new();
    for svc in services {
        if !svc.is_host_routed() {
            // No nginx vhost — nginx can't route plain TCP by hostname. The service is
            // reached as {svc}.{dom}.test:{auto_port} with the port resolving via the
            // service container's -p {auto_port}:8002 mapping.
            continue;
        }
        for hostname in svc.hostnames() {
            out.push_str(&build_host_proxy_vhost(
                &hostname,
                host_gateway,
                svc.proxy_port,
            ));
        }
    }
    out
}

/// Build the `0.0.0.0 <hostname>` lines for `hosts_container` and, when
/// `urls_in_hosts` is on, the managed system-hosts block.
///
/// Every URL gets an entry — HTTP/WS clients reach the reverse proxy on port 80 via
/// this name; TCP clients reach localhost (the hostname is a loopback alias once
/// `urls_in_hosts` syncs `/etc/hosts`). Aliases are what make non-`.test` names
/// resolve at all, since dnsmasq only wildcards `.test`.
///
/// Deduplicated across services in first-seen order: a TCP and an HTTP service may
/// legitimately share a hostname, and two same-named folders in different groups
/// share a canonical hostname, but a hosts file wants one line either way.
pub fn build_hosts_lines(services: &[PlannedService]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();
    for svc in services {
        for hostname in svc.hostnames() {
            let key = alias::normalize_for_comparison(&hostname);
            if seen.insert(key) {
                lines.push(format!("0.0.0.0   {hostname}\n"));
            }
        }
    }
    lines
}

/// Build `portmap.json`. `run.rs` and `cmd_urls` read this back, so aliases are
/// recorded here in normalized form — mirroring how `type` is carried — rather than
/// being re-resolved from the config.
///
/// `domain_names` pre-seeds the configured domains so one whose location holds no
/// project folders still appears (as an empty object), the way it did when the
/// portmap was assembled domain-by-domain.
pub fn build_portmap(
    services: &[PlannedService],
    domain_names: &[String],
) -> serde_json::Map<String, serde_json::Value> {
    let mut portmap = serde_json::Map::new();
    for name in domain_names {
        portmap.insert(
            name.clone(),
            serde_json::Value::Object(serde_json::Map::new()),
        );
    }
    for svc in services {
        let mut entry = serde_json::Map::new();
        entry.insert(
            "port".to_string(),
            serde_json::Value::Number(svc.proxy_port.into()),
        );
        entry.insert(
            "type".to_string(),
            serde_json::Value::String(svc.connection_type.clone()),
        );
        entry.insert(
            "debug_port".to_string(),
            serde_json::Value::Number(svc.debug_port.into()),
        );
        if !svc.aliases.is_empty() {
            entry.insert(
                "urls".to_string(),
                serde_json::Value::Array(
                    svc.aliases
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        let domain_obj = portmap
            .entry(svc.domain.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        if let Some(domain_map) = domain_obj.as_object_mut() {
            let group_obj = domain_map
                .entry(svc.group.clone())
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            if let Some(group_map) = group_obj.as_object_mut() {
                group_map.insert(svc.service.clone(), serde_json::Value::Object(entry));
            }
        }
    }
    portmap
}

pub fn cmd_deploy(
    paths: &DarpPaths,
    config: &Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    println!("Deploying Container Development\n");

    let host_gateway = engine.host_gateway();

    // ---- Discover and validate the whole deployment before mutating anything ----
    let mut services = plan_deployment(paths, config)?;
    validate_plan_aliases(&mut services)?;
    detect_hostname_collisions(&services)?;

    // Collected here, reported at the end of the run so it isn't scrolled off by the
    // debug-port table and the container restarts.
    let urls_in_hosts = config.urls_in_hosts.unwrap_or(false);
    let unresolved = collect_unresolved_aliases(&services, urls_in_hosts);

    // ---- Generate artifacts and restart infrastructure ----

    // Refresh the embedded nginx.conf on every deploy so fixes to assets/nginx.conf
    // reach the reverse-proxy without a separate `darp install`.
    os.copy_nginx_conf()?;

    let gateway_ip =
        match engine::read_container_host_ip(&paths.container_host_ip_path, &engine.kind) {
            Some(ip) => ip,
            None => {
                let ip = engine.probe_host_gateway_ip()?;
                engine::write_container_host_ip(&paths.container_host_ip_path, &engine.kind, &ip)?;
                ip
            }
        };

    let hosts_container_lines = build_hosts_lines(&services);
    let domain_names: Vec<String> = config
        .domains
        .as_ref()
        .map(|d| d.keys().cloned().collect())
        .unwrap_or_default();
    let portmap = build_portmap(&services, &domain_names);

    std::fs::write(
        &paths.vhost_container_conf,
        build_vhost_container_conf(&services, host_gateway),
    )?;
    let hosts_content =
        build_container_hosts(&gateway_ip, engine.host_gateway(), &hosts_container_lines);
    std::fs::write(&paths.hosts_container_path, hosts_content)?;
    std::fs::write(&paths.portmap_path, serde_json::to_vec_pretty(&portmap)?)?;

    // Report assigned debug ports so each project's .vscode/launch.json "port" can be
    // set (once — ports are persisted). Also available anytime via `darp urls`.
    let mut debug_lines: Vec<(String, u16)> = services
        .iter()
        .map(|svc| {
            let label = if svc.group == "." {
                format!("{}.{}", svc.service, svc.domain)
            } else {
                format!("{}.{}.{}", svc.service, svc.group, svc.domain)
            };
            (label, svc.debug_port)
        })
        .collect();
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
    if urls_in_hosts {
        let os = OsIntegration::new(paths, config, &engine.kind);
        os.sync_system_hosts(&hosts_container_lines)?;

        if config.wsl.unwrap_or(false) {
            os.sync_windows_hosts(&hosts_container_lines)?;
        }
    }

    // Non-fatal: the aliases may already resolve through DNS the engineer manages.
    if let Some(warning) = render_unresolved_alias_warning(&unresolved) {
        eprintln!("\n{warning}");
    }

    Ok(())
}
