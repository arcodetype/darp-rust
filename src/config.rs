// config.rs
use anyhow::{anyhow, Result};
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
        let darp_root_env = std::env::var("DARP_ROOT").unwrap_or_else(|_| {
            home.join(".darp")
                .to_string_lossy()
                .into_owned()
        });
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub engine: Option<String>,
    pub podman_machine: Option<String>,
    pub domains: Option<std::collections::BTreeMap<String, Domain>>,
    pub environments: Option<std::collections::BTreeMap<String, Environment>>,
    pub urls_in_hosts: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    #[serde(default)]
    pub services: Option<BTreeMap<String, Service>>,
    #[serde(default)]
    pub default_environment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Service {
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub volumes: Option<Vec<Volume>>,
    #[serde(default)]
    pub serve_command: Option<String>,
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
    pub image_repository: Option<String>,
    #[serde(default)]
    pub host_portmappings: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub default_container_image: Option<String>,
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
        service: Option<&Service>,
        cli_image: &str,
    ) -> String {
        if let Some(svc) = service {
            if let Some(repo) = &svc.image_repository {
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

    pub fn add_domain(&mut self, location: &str) -> Result<()> {
        let loc_path = PathBuf::from(location);
        let domain_label = loc_path
            .file_name()
            .ok_or_else(|| anyhow!("Could not determine domain name from {location}"))?
            .to_string_lossy()
            .to_string();

        let domain_name = slugify_name(&domain_label);

        let loc_abs = fs::canonicalize(&loc_path).map_err(|e| {
            anyhow!(
                "Failed to canonicalize domain location '{}': {}",
                location,
                e
            )
        })?;
        let loc_key = loc_abs.to_string_lossy().to_string();

        let domains = self.domains.get_or_insert_with(BTreeMap::new);

        if domains.contains_key(&loc_key) {
            return Err(anyhow!(
                "Domain with location '{}' already exists.",
                loc_key
            ));
        }

        if domains.values().any(|d| d.name == domain_name) {
            return Err(anyhow!(
                "Domain name '{}' already exists. Domain names must be unique.",
                domain_name
            ));
        }

        domains.insert(
            loc_key.clone(),
            Domain {
                name: domain_name.clone(),
                services: None,
                default_environment: None,
            },
        );

        println!("created '{}' at {}", domain_name, loc_key);
        Ok(())
    }

    pub fn rm_domain(&mut self, name: &str) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("no domains configured"))?;

        // Support removing either by name or by exact location key.
        let key_to_remove = domains
            .iter()
            .find(|(location, domain)| domain.name == name || location.as_str() == name)
            .map(|(location, _)| location.clone());

        if let Some(key) = key_to_remove {
            domains.remove(&key);
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
            .values_mut()
            .find(|d| d.name == domain_name)
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
            .values_mut()
            .find(|d| d.name == domain_name)
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

    // Environment-level serve_command

    pub fn set_serve_command(&mut self, env_name: &str, cmd: &str) -> Result<()> {
        let env = self
            .environments
            .as_mut()
            .and_then(|e| e.get_mut(env_name))
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

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

    pub fn set_default_container_image(
        &mut self,
        env_name: &str,
        image: &str,
    ) -> Result<()> {
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

    // Service-level port mappings

    pub fn add_portmap(
        &mut self,
        domain_name: &str,
        service_name: &str,
        host_port: &str,
        container_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        // Look up by logical domain name (Domain.name), *not* by location key.
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
        let service = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);
        let host_maps = service
            .host_portmappings
            .get_or_insert_with(BTreeMap::new);

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
        service_name: &str,
        host_port: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;

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
        let env = envs
            .entry(env_name.to_string())
            .or_insert_with(Environment::default);

        let maps = env
            .host_portmappings
            .get_or_insert_with(BTreeMap::new);

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

    pub fn rm_env_portmap(
        &mut self,
        env_name: &str,
        host_port: &str,
    ) -> Result<()> {
        let envs = self
            .environments
            .as_mut()
            .ok_or_else(|| anyhow!("No environments configured"))?;
        let env = envs
            .get_mut(env_name)
            .ok_or_else(|| anyhow!("Environment '{}' does not exist.", env_name))?;

        let maps = env
            .host_portmappings
            .as_mut()
            .ok_or_else(|| anyhow!("No host_portmappings configured for environment '{}'", env_name))?;

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
        let env = envs
            .entry(env_name.to_string())
            .or_insert_with(Environment::default);

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

    pub fn rm_volume(
        &mut self,
        env_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
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
        service_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
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
        service_name: &str,
        container_dir: &str,
        host_dir: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;

        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;
        let svc = services
            .get_mut(service_name)
            .ok_or_else(|| anyhow!("service, {}, does not exist", service_name))?;

        let vols = svc
            .volumes
            .as_mut()
            .ok_or_else(|| anyhow!("No volumes configured for service '{}.{}'", domain_name, service_name))?;

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
        service_name: &str,
        cmd: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.serve_command = Some(cmd.to_string());
        Ok(())
    }

    pub fn rm_service_serve_command(
        &mut self,
        domain_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;
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

    // Service-level image_repository

    pub fn set_service_image_repository(
        &mut self,
        domain_name: &str,
        service_name: &str,
        repo: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.image_repository = Some(repo.to_string());
        Ok(())
    }

    pub fn rm_service_image_repository(
        &mut self,
        domain_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;
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
        service_name: &str,
        platform: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.platform = Some(platform.to_string());
        Ok(())
    }

    pub fn rm_service_platform(
        &mut self,
        domain_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;
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
        service_name: &str,
        image: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain.services.get_or_insert_with(BTreeMap::new);
        let svc = services
            .entry(service_name.to_string())
            .or_insert_with(Service::default);

        svc.default_container_image = Some(image.to_string());
        Ok(())
    }

    pub fn rm_service_default_container_image(
        &mut self,
        domain_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let domains = self
            .domains
            .as_mut()
            .ok_or_else(|| anyhow!("No domains configured"))?;
        let domain = domains
            .values_mut()
            .find(|d| d.name == domain_name)
            .ok_or_else(|| anyhow!("domain, {}, does not exist", domain_name))?;

        let services = domain
            .services
            .as_mut()
            .ok_or_else(|| anyhow!("No services configured for domain {}", domain_name))?;
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

//// Simple slugifier for domain names:
/// - lower-cases
/// - turns spaces/underscores/dashes into single '-'
/// - strips leading/trailing '-'
fn slugify_name(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if ch.is_whitespace() || ch == '_' || ch == '-' {
            if !last_dash && !out.is_empty() {
                out.push('-');
                last_dash = true;
            }
        } else {
            // skip other punctuation
        }
    }

    if out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "domain".to_string()
    } else {
        out
    }
}
