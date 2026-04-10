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
    pub engine: Option<String>,
    pub podman_machine: Option<String>,
    pub domains: Option<std::collections::BTreeMap<String, Domain>>,
    pub environments: Option<std::collections::BTreeMap<String, Environment>>,
    pub urls_in_hosts: Option<bool>,
}

pub fn resolve_location(location: &str) -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow!("Could not determine home directory"))?;
    let resolved = location.replace("{home}", &home.to_string_lossy());
    Ok(PathBuf::from(resolved))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub location: String,
    #[serde(default)]
    pub groups: Option<BTreeMap<String, Group>>,
    #[serde(default)]
    pub default_environment: Option<String>,
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default)]
    pub serve_command: Option<String>,
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default)]
    pub image_repository: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Group {
    #[serde(default)]
    pub services: Option<BTreeMap<String, Service>>,
    #[serde(default)]
    pub default_environment: Option<String>,
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default)]
    pub serve_command: Option<String>,
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default)]
    pub image_repository: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Service {
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default)]
    pub serve_command: Option<String>,
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default)]
    pub image_repository: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub default_container_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Environment {
    #[serde(default)]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default)]
    pub serve_command: Option<String>,
    #[serde(default)]
    pub shell_command: Option<String>,
    #[serde(default)]
    pub image_repository: Option<String>,
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub variables: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub default_container_image: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Volume {
    pub container: String,
    pub host: String,
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
        let data = serde_json::to_vec_pretty(self)?;
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
            "false" | "0" | "no" | "n" | "off" => Err(anyhow!(
                "Invalid boolean value: {} (expected TRUE/FALSE/yes/no/1/0)",
                s
            )),
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

    pub fn resolve_image_name(
        &self,
        environment: Option<&Environment>,
        group: Option<&Group>,
        domain: Option<&Domain>,
        service: Option<&Service>,
        cli_image: &str,
    ) -> String {
        if let Some(svc) = service {
            if let Some(repo) = &svc.image_repository {
                return format!("{repo}:{image}", repo = repo, image = cli_image);
            }
        }
        if let Some(grp) = group {
            if let Some(repo) = &grp.image_repository {
                return format!("{repo}:{image}", repo = repo, image = cli_image);
            }
        }
        if let Some(dom) = domain {
            if let Some(repo) = &dom.image_repository {
                return format!("{repo}:{image}", repo = repo, image = cli_image);
            }
        }
        if let Some(env) = environment {
            if let Some(repo) = &env.image_repository {
                return format!("{repo}:{image}", repo = repo, image = cli_image);
            }
        }
        cli_image.to_string()
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

    pub fn add_group(&mut self, domain_name: &str, group_name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .get_mut(domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let groups = domain.groups.get_or_insert_with(BTreeMap::new);

        if groups.contains_key(group_name) {
            return Err(anyhow!(
                "Group '{}' already exists in domain '{}'.",
                group_name,
                domain_name
            ));
        }

        groups.insert(group_name.to_string(), Group::default());
        println!("Created group '{}' in domain '{}'", group_name, domain_name);
        Ok(())
    }

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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
        let group = groups.get_mut(group_name).ok_or_else(|| {
            anyhow!(
                "group, {}, does not exist in domain {}",
                group_name,
                domain_name
            )
        })?;
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
                    // Force replace: no recursion
                    let actual_key = actual_key.to_string();
                    star_overrides.insert(actual_key.clone());
                    base_map.insert(actual_key, overlay_val);
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
        (_, overlay) => {
            // Scalar or type mismatch: overlay wins
            overlay
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
