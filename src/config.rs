use anyhow::{Result, anyhow};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let data = fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}

#[derive(Clone, Debug)]
pub struct DarpPaths {
    pub _darp_root: PathBuf,
    pub config_path: PathBuf,
    pub portmap_path: PathBuf,
    pub dnsmasq_dir: PathBuf,
    pub vhost_container_conf: PathBuf,
    pub hosts_container_path: PathBuf,
    pub nginx_conf_path: PathBuf,
    pub container_host_ip_path: PathBuf,
}

impl DarpPaths {
    pub fn from_env() -> Result<Self> {
        let home = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
        let darp_root_env = std::env::var("DARP_ROOT")
            .unwrap_or_else(|_| home.join(".darp").to_string_lossy().into_owned());
        let darp_root = PathBuf::from(darp_root_env);

        Ok(Self {
            _darp_root: darp_root.clone(),
            config_path: darp_root.join("config.json"),
            portmap_path: darp_root.join("portmap.json"),
            dnsmasq_dir: darp_root.join("dnsmasq.d"),
            vhost_container_conf: darp_root.join("vhost_container.conf"),
            hosts_container_path: darp_root.join("hosts_container"),
            nginx_conf_path: darp_root.join("nginx.conf"),
            container_host_ip_path: darp_root.join("container_host_ip"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreConfig {
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_config: Option<Vec<PreConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub podman_machine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domains: Option<std::collections::BTreeMap<String, Domain>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environments: Option<std::collections::BTreeMap<String, Environment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urls_in_hosts: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsl: Option<bool>,
}

pub fn resolve_location(location: &str) -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let resolved = location.replace("{home}", &home.to_string_lossy());
    Ok(PathBuf::from(resolved))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Domain {
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub groups: Option<BTreeMap<String, Group>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services: Option<BTreeMap<String, Service>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Service {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Environment {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serve_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_container_image: Option<String>,
}

/// A borrow-based view of the 8 cascadable fields from any config layer.
struct CascadeLayer<'a> {
    serve_command: Option<&'a str>,
    shell_command: Option<&'a str>,
    image_repository: Option<&'a str>,
    platform: Option<&'a str>,
    default_container_image: Option<&'a str>,
    host_portmappings: Option<&'a BTreeMap<String, String>>,
    variables: Option<&'a BTreeMap<String, String>>,
    volumes: Option<&'a Vec<Volume>>,
}

impl<'a> From<&'a Domain> for CascadeLayer<'a> {
    fn from(d: &'a Domain) -> Self {
        Self {
            serve_command: d.serve_command.as_deref(),
            shell_command: d.shell_command.as_deref(),
            image_repository: d.image_repository.as_deref(),
            platform: d.platform.as_deref(),
            default_container_image: d.default_container_image.as_deref(),
            host_portmappings: d.host_portmappings.as_ref(),
            variables: d.variables.as_ref(),
            volumes: d.volumes.as_ref(),
        }
    }
}

impl<'a> From<&'a Group> for CascadeLayer<'a> {
    fn from(g: &'a Group) -> Self {
        Self {
            serve_command: g.serve_command.as_deref(),
            shell_command: g.shell_command.as_deref(),
            image_repository: g.image_repository.as_deref(),
            platform: g.platform.as_deref(),
            default_container_image: g.default_container_image.as_deref(),
            host_portmappings: g.host_portmappings.as_ref(),
            variables: g.variables.as_ref(),
            volumes: g.volumes.as_ref(),
        }
    }
}

impl<'a> From<&'a Service> for CascadeLayer<'a> {
    fn from(s: &'a Service) -> Self {
        Self {
            serve_command: s.serve_command.as_deref(),
            shell_command: s.shell_command.as_deref(),
            image_repository: s.image_repository.as_deref(),
            platform: s.platform.as_deref(),
            default_container_image: s.default_container_image.as_deref(),
            host_portmappings: s.host_portmappings.as_ref(),
            variables: s.variables.as_ref(),
            volumes: s.volumes.as_ref(),
        }
    }
}

impl<'a> From<&'a Environment> for CascadeLayer<'a> {
    fn from(e: &'a Environment) -> Self {
        Self {
            serve_command: e.serve_command.as_deref(),
            shell_command: e.shell_command.as_deref(),
            image_repository: e.image_repository.as_deref(),
            platform: e.platform.as_deref(),
            default_container_image: e.default_container_image.as_deref(),
            host_portmappings: e.host_portmappings.as_ref(),
            variables: e.variables.as_ref(),
            volumes: e.volumes.as_ref(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ResolvedSettings {
    pub domain_name: String,
    pub group_name: String,
    pub service_name: String,
    pub environment_name: Option<String>,
    pub serve_command: Option<String>,
    pub shell_command: Option<String>,
    pub image_repository: Option<String>,
    pub platform: Option<String>,
    pub default_container_image: Option<String>,
    pub host_portmappings: Option<BTreeMap<String, String>>,
    pub variables: Option<BTreeMap<String, String>>,
    pub volumes: Option<Vec<Volume>>,
}

impl ResolvedSettings {
    /// Resolve cascading settings from service > group > domain > environment.
    /// The first layer to provide a value for each field wins.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        domain_name: String,
        group_name: String,
        service_name: String,
        environment_name: Option<String>,
        service: Option<&Service>,
        group: Option<&Group>,
        domain: &Domain,
        environment: Option<&Environment>,
    ) -> Self {
        let layers: [Option<CascadeLayer>; 4] = [
            service.map(CascadeLayer::from),
            group.map(CascadeLayer::from),
            Some(CascadeLayer::from(domain)),
            environment.map(CascadeLayer::from),
        ];

        Self {
            domain_name,
            group_name,
            service_name,
            environment_name,
            serve_command: layers
                .iter()
                .flatten()
                .find_map(|l| l.serve_command)
                .map(String::from),
            shell_command: layers
                .iter()
                .flatten()
                .find_map(|l| l.shell_command)
                .map(String::from),
            image_repository: layers
                .iter()
                .flatten()
                .find_map(|l| l.image_repository)
                .map(String::from),
            platform: layers
                .iter()
                .flatten()
                .find_map(|l| l.platform)
                .map(String::from),
            default_container_image: layers
                .iter()
                .flatten()
                .find_map(|l| l.default_container_image)
                .map(String::from),
            host_portmappings: layers
                .iter()
                .flatten()
                .find_map(|l| l.host_portmappings)
                .cloned(),
            variables: layers.iter().flatten().find_map(|l| l.variables).cloned(),
            volumes: layers.iter().flatten().find_map(|l| l.volumes).cloned(),
        }
    }

    /// Returns the resolved image name: image_repository:base_image, or just base_image.
    /// If cli_image is provided, it takes precedence over default_container_image.
    pub fn resolve_full_image_name(&self, cli_image: Option<&str>) -> Option<String> {
        let base = cli_image
            .map(String::from)
            .or_else(|| self.default_container_image.clone())?;

        match &self.image_repository {
            Some(repo) => Some(format!("{}:{}", repo, base)),
            None => Some(base),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub container: String,
    pub host: String,
}

fn strip_nulls(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        obj.retain(|_, v| !v.is_null());
        for v in obj.values_mut() {
            strip_nulls(v);
        }
    } else if let Some(arr) = value.as_array_mut() {
        for v in arr.iter_mut() {
            strip_nulls(v);
        }
    }
}

pub struct ServiceContext<'a> {
    pub current_dir: PathBuf,
    pub current_directory_name: String,
    pub domain_name: String,
    pub domain: &'a Domain,
    pub group_name: String,
    pub group: Option<&'a Group>,
    pub service: Option<&'a Service>,
    pub environment_name: Option<String>,
    pub environment: Option<&'a Environment>,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, b"{}")?;
            return Ok(Self::default());
        }

        maybe_migrate(path)?;

        let data = fs::read(path)?;
        let cfg = serde_json::from_slice(&data).unwrap_or_default();
        Ok(cfg)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut value = serde_json::to_value(self)?;
        strip_nulls(&mut value);
        let data = serde_json::to_vec_pretty(&value)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn add_pre_config(&mut self, location: &str, repo_location: Option<&str>) -> Result<()> {
        let entries = self.pre_config.get_or_insert_with(Vec::new);
        if entries.iter().any(|e| e.location == location) {
            return Err(anyhow!(
                "pre_config with location '{}' already exists",
                location
            ));
        }
        entries.push(PreConfig {
            location: location.to_string(),
            repo_location: repo_location.map(|s| s.to_string()),
        });
        Ok(())
    }

    pub fn rm_pre_config(&mut self, location: &str) -> Result<()> {
        let entries = self
            .pre_config
            .as_mut()
            .ok_or_else(|| anyhow!("No pre_config entries configured"))?;
        let before = entries.len();
        entries.retain(|e| e.location != location);
        if entries.len() == before {
            return Err(anyhow!(
                "pre_config with location '{}' does not exist",
                location
            ));
        }
        if entries.is_empty() {
            self.pre_config = None;
        }
        Ok(())
    }

    /// Build a full ServiceContext from the current working directory.
    /// Returns None when cwd isn't inside any configured domain.
    pub fn service_context_from_cwd(&self, env_cli: Option<String>) -> Option<ServiceContext<'_>> {
        let current_dir = std::env::current_dir().ok()?;
        let current_directory_name = current_dir.file_name()?.to_string_lossy().to_string();

        let (domain_name, domain, group_name, group) = self.find_context_by_cwd(&current_dir)?;
        let domain_name = domain_name.to_string();

        let service = group
            .and_then(|g| g.services.as_ref())
            .and_then(|s| s.get(&current_directory_name));

        let environment_name: Option<String> = env_cli
            .or_else(|| service.and_then(|s| s.default_environment.clone()))
            .or_else(|| group.and_then(|g| g.default_environment.clone()))
            .or_else(|| domain.default_environment.clone());

        let environment = environment_name
            .as_ref()
            .and_then(|name| self.environments.as_ref().and_then(|e| e.get(name)));

        Some(ServiceContext {
            current_dir,
            current_directory_name,
            domain_name,
            domain,
            group_name,
            group,
            service,
            environment_name,
            environment,
        })
    }

    /// Find domain, group, and service context from the current working directory.
    /// Returns (domain_name, domain, group_name, group_opt) or None.
    ///
    /// Detection logic:
    /// 1. If parent dir matches a domain location → group = ".", service = current_dir
    /// 2. If grandparent dir matches a domain location → group = parent_dir_name, service = current_dir
    pub fn find_context_by_cwd(
        &self,
        current_dir: &std::path::Path,
    ) -> Option<(&str, &Domain, String, Option<&Group>)> {
        let _current_dir_name = current_dir.file_name()?.to_string_lossy().to_string();

        let parent = current_dir.parent()?;
        let parent_canonical = fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
        let parent_key = parent_canonical.to_string_lossy().to_string();

        // Step 1: parent = domain → group "."
        if let Some((domain_name, domain)) = self.find_domain_by_location(&parent_key) {
            let group = domain.groups.as_ref().and_then(|g| g.get("."));
            return Some((domain_name, domain, ".".to_string(), group));
        }

        // Step 2: grandparent = domain → group = parent dir name
        let grandparent = parent.parent()?;
        let grandparent_canonical =
            fs::canonicalize(grandparent).unwrap_or_else(|_| grandparent.to_path_buf());
        let grandparent_key = grandparent_canonical.to_string_lossy().to_string();

        let parent_dir_name = parent.file_name()?.to_string_lossy().to_string();

        if let Some((domain_name, domain)) = self.find_domain_by_location(&grandparent_key) {
            let group = domain.groups.as_ref().and_then(|g| g.get(&parent_dir_name));
            return Some((domain_name, domain, parent_dir_name, group));
        }

        None
    }

    pub fn find_domain_by_location(&self, canonical_path: &str) -> Option<(&str, &Domain)> {
        self.domains
            .as_ref()?
            .iter()
            .find(|(_name, d)| {
                resolve_location(&d.location)
                    .map(|loc| {
                        let canonical = fs::canonicalize(&loc).unwrap_or(loc);
                        canonical.to_string_lossy() == canonical_path
                    })
                    .unwrap_or(false)
            })
            .map(|(name, domain)| (name.as_str(), domain))
    }

    pub fn parse_bool(&self, s: &str) -> Result<bool> {
        let v = s.trim().to_lowercase();
        match v.as_str() {
            "true" | "1" | "yes" | "y" | "on" => Ok(true),
            "false" | "0" | "no" | "n" | "off" => Ok(false),
            _ => Err(anyhow!(
                "Invalid boolean value: {} (expected TRUE/FALSE/yes/no/1/0)",
                s
            )),
        }
    }

    pub fn resolve_host_path(&self, template: &str, current_dir: &Path) -> Result<PathBuf> {
        const PSEUDO_PWD_TOKEN: &str = "{pwd}";
        const PSEUDO_HOME_TOKEN: &str = "{home}";

        let home = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;

        let s = template
            .replace(PSEUDO_PWD_TOKEN, &current_dir.to_string_lossy())
            .replace(PSEUDO_HOME_TOKEN, &home.to_string_lossy());

        Ok(PathBuf::from(s))
    }

    // --- domain/env helpers ---

    pub fn add_domain(&mut self, name: &str, location: &str) -> Result<()> {
        let domains = self.domains.get_or_insert_with(BTreeMap::new);

        if domains.contains_key(name) {
            return Err(anyhow!("Domain '{}' already exists.", name));
        }

        domains.insert(
            name.to_string(),
            Domain {
                location: location.to_string(),
                groups: None,
                default_environment: None,
                host_portmappings: None,
                variables: None,
                volumes: None,
                serve_command: None,
                shell_command: None,
                image_repository: None,
                platform: None,
                default_container_image: None,
            },
        );

        println!("created '{}' at {}", name, location);
        Ok(())
    }

    pub fn ensure_domain_exists(
        &mut self,
        domain_name: &str,
        location: Option<&str>,
    ) -> Result<()> {
        let domains = self.domains.get_or_insert_with(BTreeMap::new);
        if domains.contains_key(domain_name) {
            return Ok(());
        }
        // Explicit -l takes priority
        if let Some(loc) = location {
            return self.add_domain(domain_name, loc);
        }
        // Check pre_configs for the domain
        if let Some(pre_configs) = &self.pre_config {
            for pc in pre_configs {
                let resolved = resolve_location(&pc.location)?;
                if !resolved.exists() {
                    continue;
                }
                let data = fs::read(&resolved)?;
                let parent: Config = serde_json::from_slice(&data).unwrap_or_default();
                if let Some(parent_domains) = &parent.domains {
                    if let Some(dom) = parent_domains.get(domain_name) {
                        return self.add_domain(domain_name, &dom.location);
                    }
                }
            }
        }
        Err(anyhow!(
            "domain '{}' does not exist. Use -l <path> to create it.",
            domain_name
        ))
    }

    pub fn rm_domain(&mut self, name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("no domains configured"))?;

        if domains.remove(name).is_some() {
            println!("removed '{}'", name);
            Ok(())
        } else {
            Err(anyhow!("domain {} does not exist", name))
        }
    }

    // Domain-level default_environment

    pub fn set_domain_default_environment(
        &mut self,
        domain_name: &str,
        env_name: &str,
    ) -> Result<()> {
        // Ensure referenced environment exists
        let envs = self
            .environments
            .as_ref()
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;
        if !envs.contains_key(env_name) {
            return Err(anyhow!("Environment '{}' does not exist.", env_name));
        }

        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.default_environment = Some(env_name.to_string());
        Ok(())
    }

    pub fn rm_domain_default_environment(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.default_environment.is_none() {
            return Err(anyhow!(
                "Domain '{}' has no default_environment.",
                domain_name
            ));
        }

        domain.default_environment = None;
        Ok(())
    }

    // Domain-level serve_command

    pub fn set_domain_serve_command(&mut self, domain_name: &str, cmd: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.serve_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_domain_serve_command(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.serve_command.is_none() {
            return Err(anyhow!(
                "Domain '{}' has no custom serve_command.",
                domain_name
            ));
        }

        domain.serve_command = None;
        Ok(())
    }

    // Domain-level shell_command

    pub fn set_domain_shell_command(&mut self, domain_name: &str, cmd: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.shell_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_domain_shell_command(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.shell_command.is_none() {
            return Err(anyhow!(
                "Domain '{}' has no custom shell_command.",
                domain_name
            ));
        }

        domain.shell_command = None;
        Ok(())
    }

    // Domain-level image_repository

    pub fn set_domain_image_repository(&mut self, domain_name: &str, repo: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.image_repository = Some(repo.to_string());
        Ok(())
    }

    pub fn rm_domain_image_repository(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.image_repository.is_none() {
            return Err(anyhow!(
                "Domain '{}' has no custom image_repository.",
                domain_name
            ));
        }

        domain.image_repository = None;
        Ok(())
    }

    // Domain-level platform

    pub fn set_domain_platform(&mut self, domain_name: &str, platform: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.platform = Some(platform.to_string());
        Ok(())
    }

    pub fn rm_domain_platform(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.platform.is_none() {
            return Err(anyhow!("Domain '{}' has no custom platform.", domain_name));
        }

        domain.platform = None;
        Ok(())
    }

    // Domain-level default_container_image

    pub fn set_domain_default_container_image(
        &mut self,
        domain_name: &str,
        image: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        domain.default_container_image = Some(image.to_string());
        Ok(())
    }

    pub fn rm_domain_default_container_image(&mut self, domain_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        if domain.default_container_image.is_none() {
            return Err(anyhow!(
                "Domain '{}' has no default_container_image.",
                domain_name
            ));
        }

        domain.default_container_image = None;
        Ok(())
    }

    // Domain-level port mappings

    pub fn add_domain_portmap(
        &mut self,
        domain_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let maps = domain.host_portmappings.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(host_port) {
            return Err(anyhow!(
                "Portmapping on host side for domain '{}' ({}:____) already exists",
                domain_name,
                host_port
            ));
        }

        maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created portmapping for domain '{}' ({}:{})",
            domain_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_domain_portmap(&mut self, domain_name: &str, host_port: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let maps = domain.host_portmappings.as_mut().ok_or_else(|| {
            anyhow!(
                "No host_portmappings configured for domain '{}'",
                domain_name
            )
        })?;

        if maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Portmapping on host side for domain '{}' ({}:____) does not exist",
                domain_name,
                host_port
            ));
        }

        println!(
            "Removed portmapping for domain '{}' ({}:____)",
            domain_name, host_port
        );
        Ok(())
    }

    // Domain-level variables

    pub fn add_domain_variable(
        &mut self,
        domain_name: &str,
        name: &str,
        value: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let maps = domain.variables.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(name) {
            return Err(anyhow!(
                "Variable for domain '{}' ({}:____) already exists",
                domain_name,
                name
            ));
        }

        maps.insert(name.to_string(), value.to_string());
        println!(
            "Created variable for domain '{}' ({}:{})",
            domain_name, name, value
        );
        Ok(())
    }

    pub fn rm_domain_variable(&mut self, domain_name: &str, name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let maps = domain
            .variables
            .as_mut()
            .ok_or_else(|| anyhow!("No variables configured for domain '{}'", domain_name))?;

        if maps.remove(name).is_none() {
            return Err(anyhow!(
                "Variable for domain '{}' ({}:____) does not exist",
                domain_name,
                name
            ));
        }

        println!(
            "Removed variable for domain '{}' ({}:____)",
            domain_name, name
        );
        Ok(())
    }

    // Domain-level volumes

    pub fn add_domain_volume(
        &mut self,
        domain_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let vols = domain.volumes.get_or_insert_with(Vec::new);

        let new_vol = Volume {
            container: container_dir.to_string(),
            host: host_dir.to_string(),
        };

        if vols
            .iter()
            .any(|v| v.container == new_vol.container && v.host == new_vol.host)
        {
            return Err(anyhow!(
                "Volume mapping already exists for domain '{}': {} -> {}",
                domain_name,
                new_vol.host,
                new_vol.container
            ));
        }

        vols.push(new_vol);
        println!(
            "Added volume to domain '{}': {} -> {}",
            domain_name, host_dir, container_dir
        );
        Ok(())
    }

    pub fn rm_domain_volume(
        &mut self,
        domain_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let vols = domain
            .volumes
            .as_mut()
            .ok_or_else(|| anyhow!("No volumes configured for domain '{}'", domain_name))?;

        let before = vols.len();
        vols.retain(|v| !(v.container == container_dir && v.host == host_dir));

        if vols.len() == before {
            return Err(anyhow!(
                "No matching volume found in domain '{}' for host '{}' -> container '{}'",
                domain_name,
                host_dir,
                container_dir
            ));
        }

        println!(
            "Removed volume from domain '{}': {} -> {}",
            domain_name, host_dir, container_dir
        );
        Ok(())
    }

    // --- Group-level CRUD ---

    pub fn rm_group(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;

        if groups.remove(group_name).is_some() {
            println!(
                "Removed group '{}' from domain '{}'",
                group_name, domain_name
            );
            Ok(())
        } else {
            Err(anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            ))
        }
    }

    pub fn rm_service(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;

        if services.remove(service_name).is_some() {
            println!(
                "Removed service '{}' from group '{}' in domain '{}'",
                service_name, group_name, domain_name
            );
            Ok(())
        } else {
            Err(anyhow!(
                "service, {}, does not exist in group '{}' of domain {}",
                service_name,
                group_name,
                domain_name
            ))
        }
    }

    // Group-level default_environment

    pub fn set_group_default_environment(
        &mut self,
        domain_name: &str,
        group_name: &str,
        env_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.default_environment = Some(env_name.to_string());
        Ok(())
    }

    pub fn rm_group_default_environment(
        &mut self,
        domain_name: &str,
        group_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.default_environment.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no default_environment.",
                group_name,
                domain_name
            ));
        }

        group.default_environment = None;
        Ok(())
    }

    // Group-level serve_command

    pub fn set_group_serve_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        cmd: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.serve_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_group_serve_command(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.serve_command.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no custom serve_command.",
                group_name,
                domain_name
            ));
        }

        group.serve_command = None;
        Ok(())
    }

    // Group-level shell_command

    pub fn set_group_shell_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        cmd: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.shell_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_group_shell_command(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.shell_command.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no custom shell_command.",
                group_name,
                domain_name
            ));
        }

        group.shell_command = None;
        Ok(())
    }

    // Group-level image_repository

    pub fn set_group_image_repository(
        &mut self,
        domain_name: &str,
        group_name: &str,
        repo: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.image_repository = Some(repo.to_string());
        Ok(())
    }

    pub fn rm_group_image_repository(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.image_repository.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no custom image_repository.",
                group_name,
                domain_name
            ));
        }

        group.image_repository = None;
        Ok(())
    }

    // Group-level platform

    pub fn set_group_platform(
        &mut self,
        domain_name: &str,
        group_name: &str,
        platform: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.platform = Some(platform.to_string());
        Ok(())
    }

    pub fn rm_group_platform(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.platform.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no custom platform.",
                group_name,
                domain_name
            ));
        }

        group.platform = None;
        Ok(())
    }

    // Group-level default_container_image

    pub fn set_group_default_container_image(
        &mut self,
        domain_name: &str,
        group_name: &str,
        image: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        group.default_container_image = Some(image.to_string());
        Ok(())
    }

    pub fn rm_group_default_container_image(
        &mut self,
        domain_name: &str,
        group_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        if group.default_container_image.is_none() {
            return Err(anyhow!(
                "Group '{}' in domain '{}' has no default_container_image.",
                group_name,
                domain_name
            ));
        }

        group.default_container_image = None;
        Ok(())
    }

    // Group-level port mappings

    pub fn add_group_portmap(
        &mut self,
        domain_name: &str,
        group_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        let maps = group.host_portmappings.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(host_port) {
            return Err(anyhow!(
                "Portmapping on host side for group '{}' in domain '{}' ({}:____) already exists",
                group_name,
                domain_name,
                host_port
            ));
        }

        maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created portmapping for group '{}' in domain '{}' ({}:{})",
            group_name, domain_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_group_portmap(
        &mut self,
        domain_name: &str,
        group_name: &str,
        host_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        let maps = group.host_portmappings.as_mut().ok_or_else(|| {
            anyhow!(
                "No host_portmappings configured for group '{}' in domain '{}'",
                group_name,
                domain_name
            )
        })?;

        if maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Portmapping on host side for group '{}' in domain '{}' ({}:____) does not exist",
                group_name,
                domain_name,
                host_port
            ));
        }

        println!(
            "Removed portmapping for group '{}' in domain '{}' ({}:____)",
            group_name, domain_name, host_port
        );
        Ok(())
    }

    // Group-level variables

    pub fn add_group_variable(
        &mut self,
        domain_name: &str,
        group_name: &str,
        name: &str,
        value: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        let maps = group.variables.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(name) {
            return Err(anyhow!(
                "Variable for group '{}' in domain '{}' ({}:____) already exists",
                group_name,
                domain_name,
                name
            ));
        }

        maps.insert(name.to_string(), value.to_string());
        println!(
            "Created variable for group '{}' in domain '{}' ({}:{})",
            group_name, domain_name, name, value
        );
        Ok(())
    }

    pub fn rm_group_variable(
        &mut self,
        domain_name: &str,
        group_name: &str,
        name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        let maps = group.variables.as_mut().ok_or_else(|| {
            anyhow!(
                "No variables configured for group '{}' in domain '{}'",
                group_name,
                domain_name
            )
        })?;

        if maps.remove(name).is_none() {
            return Err(anyhow!(
                "Variable for group '{}' in domain '{}' ({}:____) does not exist",
                group_name,
                domain_name,
                name
            ));
        }

        println!(
            "Removed variable for group '{}' in domain '{}' ({}:____)",
            group_name, domain_name, name
        );
        Ok(())
    }

    // Group-level volumes

    pub fn add_group_volume(
        &mut self,
        domain_name: &str,
        group_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();

        let vols = group.volumes.get_or_insert_with(Vec::new);

        let new_vol = Volume {
            container: container_dir.to_string(),
            host: host_dir.to_string(),
        };

        if vols
            .iter()
            .any(|v| v.container == new_vol.container && v.host == new_vol.host)
        {
            return Err(anyhow!(
                "Volume mapping already exists for group '{}' in domain '{}': {} -> {}",
                group_name,
                domain_name,
                new_vol.host,
                new_vol.container
            ));
        }

        vols.push(new_vol);
        println!(
            "Added volume to group '{}' in domain '{}': {} -> {}",
            group_name, domain_name, host_dir, container_dir
        );
        Ok(())
    }

    pub fn rm_group_volume(
        &mut self,
        domain_name: &str,
        group_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;
        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;

        let vols = group.volumes.as_mut().ok_or_else(|| {
            anyhow!(
                "No volumes configured for group '{}' in domain '{}'",
                group_name,
                domain_name
            )
        })?;

        let before = vols.len();
        vols.retain(|v| !(v.container == container_dir && v.host == host_dir));

        if vols.len() == before {
            return Err(anyhow!(
                "No matching volume found in group '{}' in domain '{}' for host '{}' -> container '{}'",
                group_name,
                domain_name,
                host_dir,
                container_dir
            ));
        }

        println!(
            "Removed volume from group '{}' in domain '{}': {} -> {}",
            group_name, domain_name, host_dir, container_dir
        );
        Ok(())
    }

    // Environment-level serve_command

    pub fn set_serve_command(&mut self, env_name: &str, cmd: &str) -> Result<()> {
        let envs = self.environments.get_or_insert_with(BTreeMap::new);
        let env = envs.entry(env_name.to_string()).or_default();

        env.serve_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_serve_command(&mut self, env_name: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        if env.serve_command.is_none() {
            return Err(anyhow!(
                "Environment '{}' has no custom serve_command.",
                env_name
            ));
        }

        env.serve_command = None;
        Ok(())
    }

    // Environment-level shell_command

    pub fn set_shell_command(&mut self, env_name: &str, cmd: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        env.shell_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_shell_command(&mut self, env_name: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        if env.shell_command.is_none() {
            return Err(anyhow!(
                "Environment '{}' has no custom shell_command.",
                env_name
            ));
        }

        env.shell_command = None;
        Ok(())
    }

    // Environment-level image_repository

    pub fn set_image_repository(&mut self, env_name: &str, repo: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        env.image_repository = Some(repo.to_string());
        Ok(())
    }

    pub fn rm_image_repository(&mut self, env_name: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        if env.image_repository.is_none() {
            return Err(anyhow!(
                "Environment '{}' has no custom image_repository.",
                env_name
            ));
        }

        env.image_repository = None;
        Ok(())
    }

    // Environment-level platform

    pub fn set_platform(&mut self, env_name: &str, platform: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        env.platform = Some(platform.to_string());
        Ok(())
    }

    pub fn rm_platform(&mut self, env_name: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        if env.platform.is_none() {
            return Err(anyhow!(
                "Environment '{}' has no custom platform.",
                env_name
            ));
        }

        env.platform = None;
        Ok(())
    }

    // Environment-level default_container_image

    pub fn set_default_container_image(&mut self, env_name: &str, image: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        env.default_container_image = Some(image.to_string());
        Ok(())
    }

    pub fn rm_default_container_image(&mut self, env_name: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        if env.default_container_image.is_none() {
            return Err(anyhow!(
                "Environment '{}' has no default_container_image.",
                env_name
            ));
        }

        env.default_container_image = None;
        Ok(())
    }

    // Service-level variables

    pub fn add_variable(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        // Look up by domain name (the map key).
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let service = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);
        let host_maps = service.variables.get_or_insert_with(BTreeMap::new);

        if host_maps.contains_key(host_port) {
            return Err(anyhow!(
                "Variable on host side '{}.{}' ({}:____) already exists",
                domain_name,
                service_name,
                host_port
            ));
        }

        host_maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created variable for '{}.{}' ({}:{})",
            domain_name, service_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_variable(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        host_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;

        let service = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        let host_maps = service
            .variables
            .as_mut()
            .ok_or_else(|| anyhow!("No variables configured"))?;

        if host_maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Variable on host side '{}.{}' ({}:____) does not exist",
                domain_name,
                service_name,
                host_port
            ));
        }

        println!(
            "Removed variable for '{}.{}' ({}:____)",
            domain_name, service_name, host_port
        );
        Ok(())
    }

    // Environment-level variables (auto-creates environment)

    pub fn add_env_variable(
        &mut self,
        env_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let envs = self.environments.get_or_insert_with(BTreeMap::new);
        let env = envs.entry(env_name.to_string()).or_default();

        let maps = env.variables.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(host_port) {
            return Err(anyhow!(
                "Variable on host side for environment '{}' ({}:____) already exists",
                env_name,
                host_port
            ));
        }

        maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created variable for environment '{}' ({}:{})",
            env_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_env_variable(&mut self, env_name: &str, host_port: &str) -> Result<()> {
        let envs = self
            .environments
            .as_mut()
            .ok_or_else(|| anyhow!("No environments configured"))?;
        let env = envs
            .get_mut(env_name)
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        let maps = env
            .variables
            .as_mut()
            .ok_or_else(|| anyhow!("No variables configured for environment '{}'", env_name))?;

        if maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Variable on host side for environment '{}' ({}:____) does not exist",
                env_name,
                host_port
            ));
        }

        println!(
            "Removed variable for environment '{}' ({}:____)",
            env_name, host_port
        );
        Ok(())
    }

    // Service-level port mappings

    pub fn add_portmap(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        // Look up by domain name (the map key).
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let service = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);
        let host_maps = service.host_portmappings.get_or_insert_with(BTreeMap::new);

        if host_maps.contains_key(host_port) {
            return Err(anyhow!(
                "Portmapping on host side '{}.{}' ({}:____) already exists",
                domain_name,
                service_name,
                host_port
            ));
        }

        host_maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created portmapping for '{}.{}' ({}:{})",
            domain_name, service_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_portmap(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        host_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;

        let service = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        let host_maps = service
            .host_portmappings
            .as_mut()
            .ok_or_else(|| anyhow!("No host_portmappings configured"))?;

        if host_maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Portmapping on host side '{}.{}' ({}:____) does not exist",
                domain_name,
                service_name,
                host_port
            ));
        }

        println!(
            "Removed portmapping for '{}.{}' ({}:____)",
            domain_name, service_name, host_port
        );
        Ok(())
    }

    // Environment-level port mappings (auto-creates environment)

    pub fn add_env_portmap(
        &mut self,
        env_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let envs = self.environments.get_or_insert_with(BTreeMap::new);
        let env = envs.entry(env_name.to_string()).or_default();

        let maps = env.host_portmappings.get_or_insert_with(BTreeMap::new);

        if maps.contains_key(host_port) {
            return Err(anyhow!(
                "Portmapping on host side for environment '{}' ({}:____) already exists",
                env_name,
                host_port
            ));
        }

        maps.insert(host_port.to_string(), container_port.to_string());
        println!(
            "Created portmapping for environment '{}' ({}:{})",
            env_name, host_port, container_port
        );
        Ok(())
    }

    pub fn rm_env_portmap(&mut self, env_name: &str, host_port: &str) -> Result<()> {
        let envs = self
            .environments
            .as_mut()
            .ok_or_else(|| anyhow!("No environments configured"))?;
        let env = envs
            .get_mut(env_name)
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        let maps = env.host_portmappings.as_mut().ok_or_else(|| {
            anyhow!(
                "No host_portmappings configured for environment '{}'",
                env_name
            )
        })?;

        if maps.remove(host_port).is_none() {
            return Err(anyhow!(
                "Portmapping on host side for environment '{}' ({}:____) does not exist",
                env_name,
                host_port
            ));
        }

        println!(
            "Removed portmapping for environment '{}' ({}:____)",
            env_name, host_port
        );
        Ok(())
    }

    // Environment-level volumes (auto-creates environment)

    pub fn add_volume(
        &mut self,
        env_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let envs = self.environments.get_or_insert_with(BTreeMap::new);
        let env = envs.entry(env_name.to_string()).or_default();

        let vols = env.volumes.get_or_insert_with(Vec::new);
        let new_vol = Volume {
            container: container_dir.to_string(),
            host: host_dir.to_string(),
        };

        if vols
            .iter()
            .any(|v| v.container == new_vol.container && v.host == new_vol.host)
        {
            return Err(anyhow!(
                "Volume mapping already exists for environment '{}': {} -> {}",
                env_name,
                new_vol.host,
                new_vol.container
            ));
        }

        vols.push(new_vol);
        println!(
            "Added volume to environment '{}': {} -> {}",
            env_name, host_dir, container_dir
        );
        Ok(())
    }

    pub fn rm_volume(&mut self, env_name: &str, container_dir: &str, host_dir: &str) -> Result<()> {
        let envs = self
            .environments
            .as_mut()
            .ok_or_else(|| anyhow!("No environments configured"))?;
        let env = envs
            .get_mut(env_name)
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        let vols = env
            .volumes
            .as_mut()
            .ok_or_else(|| anyhow!("No volumes configured for environment '{}'", env_name))?;

        let before = vols.len();
        vols.retain(|v| !(v.container == container_dir && v.host == host_dir));

        if vols.len() == before {
            return Err(anyhow!(
                "No matching volume found in environment '{}' for host '{}' -> container '{}'",
                env_name,
                host_dir,
                container_dir
            ));
        }

        println!(
            "Removed volume from environment '{}': {} -> {}",
            env_name, host_dir, container_dir
        );
        Ok(())
    }

    // Service-level volumes

    pub fn add_service_volume(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        let vols = svc.volumes.get_or_insert_with(Vec::new);

        let new_vol = Volume {
            container: container_dir.to_string(),
            host: host_dir.to_string(),
        };

        if vols
            .iter()
            .any(|v| v.container == new_vol.container && v.host == new_vol.host)
        {
            return Err(anyhow!(
                "Volume mapping already exists for service '{}.{}': {} -> {}",
                domain_name,
                service_name,
                new_vol.host,
                new_vol.container
            ));
        }

        vols.push(new_vol);
        println!(
            "Added volume to service '{}.{}': {} -> {}",
            domain_name, service_name, host_dir, container_dir
        );
        Ok(())
    }

    pub fn rm_service_volume(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        let vols = svc.volumes.as_mut().ok_or_else(|| {
            anyhow!(
                "No volumes configured for service '{}.{}'",
                domain_name,
                service_name
            )
        })?;

        let before = vols.len();
        vols.retain(|v| !(v.container == container_dir && v.host == host_dir));

        if vols.len() == before {
            return Err(anyhow!(
                "No matching volume found in service '{}.{}' for host '{}' -> container '{}'",
                domain_name,
                service_name,
                host_dir,
                container_dir
            ));
        }

        println!(
            "Removed volume from service '{}.{}': {} -> {}",
            domain_name, service_name, host_dir, container_dir
        );
        Ok(())
    }

    // Service-level serve_command

    // Service-level default_environment

    pub fn set_service_default_environment(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        env_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.default_environment = Some(env_name.to_string());
        Ok(())
    }

    pub fn rm_service_default_environment(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.default_environment.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no default_environment.",
                domain_name,
                service_name
            ));
        }

        svc.default_environment = None;
        Ok(())
    }

    // Service-level serve_command

    pub fn set_service_serve_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        cmd: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.serve_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_service_serve_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.serve_command.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no custom serve_command.",
                domain_name,
                service_name
            ));
        }

        svc.serve_command = None;
        Ok(())
    }

    // Service-level shell_command

    pub fn set_service_shell_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        cmd: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.shell_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_service_shell_command(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.shell_command.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no custom shell_command.",
                domain_name,
                service_name
            ));
        }

        svc.shell_command = None;
        Ok(())
    }

    // Service-level image_repository

    pub fn set_service_image_repository(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        repo: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.image_repository = Some(repo.to_string());
        Ok(())
    }

    pub fn rm_service_image_repository(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.image_repository.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no custom image_repository.",
                domain_name,
                service_name
            ));
        }

        svc.image_repository = None;
        Ok(())
    }

    // Service-level platform

    pub fn set_service_platform(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        platform: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.platform = Some(platform.to_string());
        Ok(())
    }

    pub fn rm_service_platform(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.platform.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no custom platform.",
                domain_name,
                service_name
            ));
        }

        svc.platform = None;
        Ok(())
    }

    // Service-level default_container_image

    pub fn set_service_default_container_image(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
        image: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);
        let group = groups.entry(group_name.to_string()).or_default();
        let services = group.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.default_container_image = Some(image.to_string());
        Ok(())
    }

    pub fn rm_service_default_container_image(
        &mut self,
        domain_name: &str,
        group_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain
            .groups
            .as_mut()
            .ok_or_else(|| anyhow!("No groups configured for domain {}", domain_name))?;
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
        let services = group.services.as_mut().ok_or_else(|| {
            anyhow!(
                "No services configured for group '{}' in domain {}",
                group_name,
                domain_name
            )
        })?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        if svc.default_container_image.is_none() {
            return Err(anyhow!(
                "Service '{}.{}' has no default_container_image.",
                domain_name,
                service_name
            ));
        }

        svc.default_container_image = None;
        Ok(())
    }
}

fn maybe_migrate(path: &Path) -> Result<()> {
    let data = fs::read(path)?;
    let mut value: serde_json::Value = serde_json::from_slice(&data).unwrap_or_default();
    let mut changed = false;

    // Migration 1: path-keyed domains → name-keyed domains with location field
    if let Some(domains) = value.get("domains").and_then(|d| d.as_object()) {
        let needs_path_migration = domains
            .iter()
            .any(|(key, val)| key.starts_with('/') && val.get("name").is_some());

        if needs_path_migration {
            let mut new_domains = serde_json::Map::new();
            for (old_key, domain_val) in domains.clone() {
                let name = domain_val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let mut new_val = domain_val.clone();
                if let Some(obj) = new_val.as_object_mut() {
                    obj.remove("name");
                    obj.insert("location".to_string(), serde_json::Value::String(old_key));
                }

                new_domains.insert(name, new_val);
            }

            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "domains".to_string(),
                    serde_json::Value::Object(new_domains),
                );
            }
            changed = true;
        }
    }

    // Migration 2: domains with "services" but no "groups" → wrap services in "." group
    if let Some(domains) = value.get_mut("domains").and_then(|d| d.as_object_mut()) {
        for (_key, domain_val) in domains.iter_mut() {
            if let Some(obj) = domain_val.as_object_mut() {
                if obj.contains_key("services") && !obj.contains_key("groups") {
                    let services = obj.remove("services").unwrap_or(serde_json::Value::Null);
                    let mut dot_group = serde_json::Map::new();
                    dot_group.insert("services".to_string(), services);

                    let mut groups = serde_json::Map::new();
                    groups.insert(".".to_string(), serde_json::Value::Object(dot_group));

                    obj.insert("groups".to_string(), serde_json::Value::Object(groups));
                    changed = true;
                }
            }
        }
    }

    // Migration 3: old string pre_config → array of objects
    if let Some(pre) = value.get("pre_config") {
        if pre.is_string() {
            let location = pre.as_str().unwrap_or("").to_string();
            let mut entry = serde_json::Map::new();
            entry.insert("location".to_string(), serde_json::Value::String(location));
            let arr = serde_json::Value::Array(vec![serde_json::Value::Object(entry)]);
            if let Some(obj) = value.as_object_mut() {
                obj.insert("pre_config".to_string(), arr);
            }
            changed = true;
        }
    }

    if changed {
        let data = serde_json::to_vec_pretty(&value)?;
        fs::write(path, data)?;
        eprintln!("Migrated config at {} to new format.", path.display());
    }

    Ok(())
}

pub fn merge_values(base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;

    match (base, overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            // Track which base keys are overridden by *-prefixed overlay keys
            let mut star_overrides: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            for (key, overlay_val) in overlay_map {
                if let Some(actual_key) = key.strip_prefix('*') {
                    let actual_key = actual_key.to_string();
                    star_overrides.insert(actual_key.clone());
                    if overlay_val.is_null() {
                        // *key: null means force-remove from parent
                        base_map.remove(&actual_key);
                    } else {
                        // *key: value means force-replace (no recursion)
                        base_map.insert(actual_key, overlay_val);
                    }
                } else if overlay_val.is_null() {
                    continue; // Plain null: don't override base values
                } else if let Some(base_val) = base_map.remove(&key) {
                    // Recursive merge
                    base_map.insert(key, merge_values(base_val, overlay_val));
                } else {
                    // New key from overlay
                    base_map.insert(key, overlay_val);
                }
            }

            Value::Object(base_map)
        }
        (Value::Array(mut base_arr), Value::Array(overlay_arr)) => {
            // Concatenate arrays by default
            base_arr.extend(overlay_arr);
            Value::Array(base_arr)
        }
        (base, overlay) => {
            // Scalar or type mismatch: overlay wins, unless overlay is null
            if overlay.is_null() { base } else { overlay }
        }
    }
}

impl Config {
    pub fn load_merged(leaf_path: &Path) -> Result<Self> {
        // 1. Load and migrate the leaf config
        if !leaf_path.exists() {
            return Config::load(leaf_path);
        }

        maybe_migrate(leaf_path)?;

        let leaf_data = fs::read(leaf_path)?;
        let leaf_val: serde_json::Value = serde_json::from_slice(&leaf_data).unwrap_or_default();

        // 2. Extract pre_config array from leaf
        let pre_configs = leaf_val
            .get("pre_config")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // 3. Load each pre_config file, check for domain conflicts between them
        let mut pre_values: Vec<serde_json::Value> = Vec::new();
        let mut seen_domains: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new(); // domain_key -> source file

        for entry in &pre_configs {
            let location = entry
                .get("location")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("pre_config entry missing 'location' field"))?;

            let resolved = resolve_location(location)?;
            if !resolved.exists() {
                eprintln!(
                    "Warning: pre_config '{}' does not exist, skipping.",
                    resolved.display()
                );
                continue;
            }

            maybe_migrate(&resolved)?;

            let data = fs::read(&resolved)?;
            let val: serde_json::Value = serde_json::from_slice(&data).unwrap_or_default();

            // Check for domain conflicts between pre_configs
            if let Some(domains) = val.get("domains").and_then(|d| d.as_object()) {
                for key in domains.keys() {
                    if let Some(prev_source) = seen_domains.get(key) {
                        return Err(anyhow!(
                            "Domain '{}' is defined in both '{}' and '{}'. \
                             Pre-configs cannot have overlapping domains.",
                            key,
                            prev_source,
                            location
                        ));
                    }
                    seen_domains.insert(key.clone(), location.to_string());
                }
            }

            pre_values.push(val);
        }

        // 4. Merge pre_configs together (array order = merge order)
        let mut merged = serde_json::Value::Object(serde_json::Map::new());
        for val in pre_values {
            merged = merge_values(merged, val);
        }

        // 5. Overlay the leaf config on top
        merged = merge_values(merged, leaf_val);

        // 6. Strip pre_config from the merged result
        if let Some(obj) = merged.as_object_mut() {
            obj.remove("pre_config");
        }

        let cfg: Config = serde_json::from_value(merged)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a Config with a single domain.
    fn config_with_domain(domain_name: &str, location: &str) -> Config {
        let mut config = Config::default();
        config.add_domain(domain_name, location).unwrap();
        config
    }

    // ---------------------------------------------------------------
    // Group auto-creation on set
    // ---------------------------------------------------------------

    #[test]
    fn set_group_serve_command_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_group_serve_command("d", "new-grp", "npm start")
            .unwrap();

        let group = config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["new-grp"]
            .clone();
        assert_eq!(group.serve_command.unwrap(), "npm start");
    }

    #[test]
    fn set_group_shell_command_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.set_group_shell_command("d", "g", "bash").unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.shell_command.as_deref(), Some("bash"));
    }

    #[test]
    fn set_group_image_repository_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_group_image_repository("d", "g", "myrepo")
            .unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.image_repository.as_deref(), Some("myrepo"));
    }

    #[test]
    fn set_group_platform_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.set_group_platform("d", "g", "linux/amd64").unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.platform.as_deref(), Some("linux/amd64"));
    }

    #[test]
    fn set_group_default_environment_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_group_default_environment("d", "g", "dev")
            .unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.default_environment.as_deref(), Some("dev"));
    }

    #[test]
    fn set_group_default_container_image_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_group_default_container_image("d", "g", "ubuntu:latest")
            .unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(
            group.default_container_image.as_deref(),
            Some("ubuntu:latest")
        );
    }

    // ---------------------------------------------------------------
    // Group auto-creation on add
    // ---------------------------------------------------------------

    #[test]
    fn add_group_portmap_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.add_group_portmap("d", "g", "8080", "80").unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.host_portmappings.as_ref().unwrap()["8080"], "80");
    }

    #[test]
    fn add_group_variable_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.add_group_variable("d", "g", "FOO", "bar").unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.variables.as_ref().unwrap()["FOO"], "bar");
    }

    #[test]
    fn add_group_volume_creates_group() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .add_group_volume("d", "g", "/app", "/host/app")
            .unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        let vols = group.volumes.as_ref().unwrap();
        assert_eq!(vols.len(), 1);
        assert_eq!(vols[0].container, "/app");
        assert_eq!(vols[0].host, "/host/app");
    }

    // ---------------------------------------------------------------
    // Service add/set auto-creates group
    // ---------------------------------------------------------------

    #[test]
    fn set_service_serve_command_creates_group_and_service() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_service_serve_command("d", "g", "svc", "npm run dev")
            .unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.serve_command.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn add_service_portmap_creates_group_and_service() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.add_portmap("d", "g", "svc", "3000", "3000").unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.host_portmappings.as_ref().unwrap()["3000"], "3000");
    }

    #[test]
    fn add_service_variable_creates_group_and_service() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.add_variable("d", "g", "svc", "KEY", "val").unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.variables.as_ref().unwrap()["KEY"], "val");
    }

    #[test]
    fn add_service_volume_creates_group_and_service() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .add_service_volume("d", "g", "svc", "/app", "/host")
            .unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        let vols = svc.volumes.as_ref().unwrap();
        assert_eq!(vols.len(), 1);
    }

    // ---------------------------------------------------------------
    // Existing group/service is preserved (not overwritten)
    // ---------------------------------------------------------------

    #[test]
    fn set_on_existing_group_preserves_other_fields() {
        let mut config = config_with_domain("d", "/tmp/d");
        config.set_group_serve_command("d", "g", "cmd1").unwrap();
        config.set_group_shell_command("d", "g", "cmd2").unwrap();

        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.serve_command.as_deref(), Some("cmd1"));
        assert_eq!(group.shell_command.as_deref(), Some("cmd2"));
    }

    #[test]
    fn add_to_existing_service_preserves_other_fields() {
        let mut config = config_with_domain("d", "/tmp/d");
        config
            .set_service_serve_command("d", "g", "svc", "cmd")
            .unwrap();
        config.add_portmap("d", "g", "svc", "8080", "80").unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.serve_command.as_deref(), Some("cmd"));
        assert_eq!(svc.host_portmappings.as_ref().unwrap()["8080"], "80");
    }

    // ---------------------------------------------------------------
    // Domain auto-creation via ensure_domain_exists
    // ---------------------------------------------------------------

    #[test]
    fn ensure_domain_exists_creates_domain_with_location() {
        let mut config = Config::default();
        config
            .ensure_domain_exists("newdom", Some("/tmp/newdom"))
            .unwrap();

        let dom = &config.domains.as_ref().unwrap()["newdom"];
        assert_eq!(dom.location, "/tmp/newdom");
    }

    #[test]
    fn ensure_domain_exists_errors_without_location() {
        let mut config = Config::default();
        let err = config.ensure_domain_exists("nope", None).unwrap_err();
        assert!(err.to_string().contains("does not exist"));
        assert!(err.to_string().contains("-l"));
    }

    #[test]
    fn ensure_domain_exists_noop_when_exists() {
        let mut config = config_with_domain("d", "/tmp/d");
        // Should not error even without location since domain already exists
        config.ensure_domain_exists("d", None).unwrap();
        assert_eq!(config.domains.as_ref().unwrap()["d"].location, "/tmp/d");
    }

    #[test]
    fn ensure_domain_exists_finds_domain_in_pre_config() {
        // Write a parent config with a domain
        let dir = std::env::temp_dir().join("darp_test_pre_config");
        let _ = fs::create_dir_all(&dir);
        let parent_path = dir.join("parent_config.json");
        let parent_json = serde_json::json!({
            "domains": {
                "parent-dom": {
                    "location": "/projects/parent-dom"
                }
            }
        });
        fs::write(
            &parent_path,
            serde_json::to_vec_pretty(&parent_json).unwrap(),
        )
        .unwrap();

        // Create a leaf config with a pre_config pointing to the parent
        let mut config = Config::default();
        config.pre_config = Some(vec![PreConfig {
            location: parent_path.to_string_lossy().into_owned(),
            repo_location: None,
        }]);

        // Should find the domain from the pre_config without -l
        config.ensure_domain_exists("parent-dom", None).unwrap();

        let dom = &config.domains.as_ref().unwrap()["parent-dom"];
        assert_eq!(dom.location, "/projects/parent-dom");

        // Cleanup
        let _ = fs::remove_file(&parent_path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn ensure_domain_exists_pre_config_domain_allows_group_operations() {
        // Write a parent config with a domain
        let dir = std::env::temp_dir().join("darp_test_pre_config_grp");
        let _ = fs::create_dir_all(&dir);
        let parent_path = dir.join("parent_config.json");
        let parent_json = serde_json::json!({
            "domains": {
                "parent-dom": {
                    "location": "/projects/parent-dom"
                }
            }
        });
        fs::write(
            &parent_path,
            serde_json::to_vec_pretty(&parent_json).unwrap(),
        )
        .unwrap();

        let mut config = Config::default();
        config.pre_config = Some(vec![PreConfig {
            location: parent_path.to_string_lossy().into_owned(),
            repo_location: None,
        }]);

        // Domain comes from pre_config, group and service auto-created
        config.ensure_domain_exists("parent-dom", None).unwrap();
        config
            .set_group_serve_command("parent-dom", "g", "npm start")
            .unwrap();

        let group = &config.domains.as_ref().unwrap()["parent-dom"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.serve_command.as_deref(), Some("npm start"));

        // Cleanup
        let _ = fs::remove_file(&parent_path);
        let _ = fs::remove_dir(&dir);
    }

    // ---------------------------------------------------------------
    // Group set/add auto-creates domain (with -l) + group
    // ---------------------------------------------------------------

    #[test]
    fn set_group_creates_domain_and_group() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config.set_group_serve_command("d", "g", "cmd").unwrap();

        let dom = &config.domains.as_ref().unwrap()["d"];
        assert_eq!(dom.location, "/tmp/d");
        let group = &dom.groups.as_ref().unwrap()["g"];
        assert_eq!(group.serve_command.as_deref(), Some("cmd"));
    }

    #[test]
    fn add_group_portmap_creates_domain_and_group() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config.add_group_portmap("d", "g", "8080", "80").unwrap();

        assert_eq!(config.domains.as_ref().unwrap()["d"].location, "/tmp/d");
        let group = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"];
        assert_eq!(group.host_portmappings.as_ref().unwrap()["8080"], "80");
    }

    #[test]
    fn set_group_on_missing_domain_errors_without_location() {
        let mut config = Config::default();
        let err = config
            .set_group_serve_command("nope", "g", "cmd")
            .unwrap_err();
        assert!(err.to_string().contains("No domains configured"));
    }

    // ---------------------------------------------------------------
    // Service set/add auto-creates domain (with -l) + group + service
    // ---------------------------------------------------------------

    #[test]
    fn set_service_creates_domain_group_and_service() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config
            .set_service_serve_command("d", "g", "svc", "npm run dev")
            .unwrap();

        let dom = &config.domains.as_ref().unwrap()["d"];
        assert_eq!(dom.location, "/tmp/d");
        let svc = &dom.groups.as_ref().unwrap()["g"].services.as_ref().unwrap()["svc"];
        assert_eq!(svc.serve_command.as_deref(), Some("npm run dev"));
    }

    #[test]
    fn add_service_portmap_creates_domain_group_and_service() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config.add_portmap("d", "g", "svc", "3000", "3000").unwrap();

        assert_eq!(config.domains.as_ref().unwrap()["d"].location, "/tmp/d");
        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.host_portmappings.as_ref().unwrap()["3000"], "3000");
    }

    #[test]
    fn add_service_variable_creates_domain_group_and_service() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config.add_variable("d", "g", "svc", "KEY", "val").unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.variables.as_ref().unwrap()["KEY"], "val");
    }

    #[test]
    fn add_service_volume_creates_domain_group_and_service() {
        let mut config = Config::default();
        config.ensure_domain_exists("d", Some("/tmp/d")).unwrap();
        config
            .add_service_volume("d", "g", "svc", "/app", "/host")
            .unwrap();

        let svc = &config.domains.as_ref().unwrap()["d"]
            .groups
            .as_ref()
            .unwrap()["g"]
            .services
            .as_ref()
            .unwrap()["svc"];
        assert_eq!(svc.volumes.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn set_service_on_missing_domain_errors_without_location() {
        let mut config = Config::default();
        let err = config
            .set_service_serve_command("nope", "g", "svc", "cmd")
            .unwrap_err();
        assert!(err.to_string().contains("No domains configured"));
    }

    // ---------------------------------------------------------------
    // Null-safe serialization
    // ---------------------------------------------------------------

    #[test]
    fn serialized_config_has_no_null_values() {
        let config = config_with_domain("d", "/tmp/d");
        let value = serde_json::to_value(&config).unwrap();
        fn assert_no_nulls(val: &serde_json::Value, path: &str) {
            match val {
                serde_json::Value::Null => panic!("Found null at {}", path),
                serde_json::Value::Object(map) => {
                    for (k, v) in map {
                        assert_no_nulls(v, &format!("{}.{}", path, k));
                    }
                }
                serde_json::Value::Array(arr) => {
                    for (i, v) in arr.iter().enumerate() {
                        assert_no_nulls(v, &format!("{}[{}]", path, i));
                    }
                }
                _ => {}
            }
        }
        assert_no_nulls(&value, "root");
    }

    #[test]
    fn strip_nulls_removes_nested_nulls() {
        let mut val: serde_json::Value = serde_json::json!({
            "a": null,
            "b": "keep",
            "c": { "d": null, "e": "also_keep" }
        });
        strip_nulls(&mut val);
        assert_eq!(
            val,
            serde_json::json!({
                "b": "keep",
                "c": { "e": "also_keep" }
            })
        );
    }

    // ---------------------------------------------------------------
    // merge_values: null handling
    // ---------------------------------------------------------------

    #[test]
    fn merge_values_null_does_not_override_base() {
        let base = serde_json::json!({ "engine": "docker", "name": "test" });
        let overlay = serde_json::json!({ "engine": null, "name": "test" });
        let merged = merge_values(base, overlay);
        assert_eq!(merged["engine"], "docker");
    }

    #[test]
    fn merge_values_star_null_removes_key() {
        let base = serde_json::json!({ "serve_command": "old" });
        let overlay = serde_json::json!({ "*serve_command": null });
        let merged = merge_values(base, overlay);
        assert!(merged.get("serve_command").is_none());
    }

    #[test]
    fn merge_values_star_value_force_replaces() {
        let base = serde_json::json!({ "cmd": "old" });
        let overlay = serde_json::json!({ "*cmd": "new" });
        let merged = merge_values(base, overlay);
        assert_eq!(merged["cmd"], "new");
    }
}
