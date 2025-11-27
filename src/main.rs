mod config;
mod engine;
mod os;

use clap::{Parser, Subcommand, CommandFactory};
use clap_complete::{generate, shells};
use colored::*;
use std::io;
use std::io::Write;

use crate::config::{Config, DarpPaths};
use crate::engine::{Engine, EngineKind};
use crate::os::OsIntegration;

/// Your directories auto-reverse proxied.
#[derive(Parser, Debug)]
#[command(
    name = "darp",
    about = "Your directories auto-reverse proxied.",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate shell completion scripts
    Completions {
        /// Shell name: bash, zsh, fish, powershell, elvish
        shell: String,
    },
    /// Configuration commands that modify config.json
    Config {
        #[command(subcommand)]
        cmd: ConfigCommand,
    },
    /// Deploys the environment
    Deploy,
    /// Runs the environment serve_command
    Serve {
        /// Environment name (required)
        #[arg(short, long)]
        environment: String,
        /// Container image to use
        container_image: String,
    },
    /// Starts a shell instance
    Shell {
        /// Environment name (optional)
        #[arg(short, long)]
        environment: Option<String>,
        /// Container image to use
        container_image: String,
    },
    /// List Darp URLs
    Urls,
    /// One-time sudo installation
    Install,
    /// Uninstall darp system integration
    Uninstall,
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Set config values / environment options
    Set {
        #[command(subcommand)]
        cmd: SetCommand,
    },
    /// Add things to config
    Add {
        #[command(subcommand)]
        cmd: AddCommand,
    },
    /// Remove things from config
    Rm {
        #[command(subcommand)]
        cmd: RmCommand,
    },
}

#[derive(Subcommand, Debug)]
enum SetCommand {
    /// Set container engine (podman|docker)
    Engine {
        engine: String,
    },
    /// Set image_repository on an environment
    ImageRepository {
        environment: String,
        image_repository: String,
    },
    /// Set Podman machine name
    PodmanMachine {
        /// Name of the Podman machine to use (e.g. 'podman-machine-default')
        new_podman_machine: String,
    },
    /// Set serve_command on an environment
    ServeCommand {
        environment: String,
        serve_command: String,
    },
    /// Enable/disable mirroring URLs into /etc/hosts
    UrlsInHosts {
        value: String,
    },
}

#[derive(Subcommand, Debug)]
enum AddCommand {
    /// Add a domain
    Domain {
        /// Location of the domain folder
        location: String,
    },
    /// Add an environment
    Environment {
        name: String,
    },
    /// Add port mapping to a service
    Portmap {
        domain_name: String,
        service_name: String,
        host_port: String,
        container_port: String,
    },
    /// Add volume to an environment
    Volume {
        environment: String,
        container_dir: String,
        host_dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum RmCommand {
    /// Remove a domain
    Domain {
        name: String,
        location: Option<String>,
    },
    /// Remove an environment
    Environment {
        name: String,
    },
    /// Remove image_repository from an environment
    ImageRepository {
        environment: String,
    },
    /// Remove PODMAN_MACHINE from config
    PodmanMachine {
    },
    /// Remove port mapping from a service
    Portmap {
        domain_name: String,
        service_name: String,
        host_port: String,
        container_port: Option<String>,
    },
    /// Remove serve_command from an environment
    ServeCommand {
        environment: String,
    },
    /// Remove volume from an environment
    Volume {
        environment: String,
        container_dir: String,
        host_dir: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Paths & config
    let paths = DarpPaths::from_env()?;
    let mut config = Config::load(&paths.config_path)?;

    // Determine engine from config
    let engine_kind = EngineKind::from_config(&config);
    let engine = Engine::new(engine_kind.clone(), &config)?;

    // OS integration abstraction
    let os = OsIntegration::new(&paths, &config, &engine_kind);

    if let Some(cmd) = cli.command {
        match cmd {
            Command::Install => cmd_install(&paths, &mut config, &os, &engine)?,
            Command::Uninstall => cmd_uninstall(&paths, &mut config, &os, &engine)?,
            Command::Deploy => cmd_deploy(&paths, &mut config, &os, &engine)?,
            Command::Shell {
                environment,
                container_image,
            } => cmd_shell(environment, container_image, &paths, &config, &engine)?,
            Command::Serve {
                environment,
                container_image,
            } => cmd_serve(environment, container_image, &paths, &config, &engine)?,
            Command::Config { cmd } => match cmd {
                ConfigCommand::Set { cmd } => {
                    cmd_set(cmd, &paths, &mut config, &engine_kind)?
                }
                ConfigCommand::Add { cmd } => cmd_add(cmd, &paths, &mut config)?,
                ConfigCommand::Rm { cmd } => cmd_rm(cmd, &paths, &mut config)?,
            },
            Command::Urls => cmd_urls(&paths, &config)?,
            Command::Completions { shell } => cmd_completions(shell),
        }
    } else {
        // No subcommand: print help
        let mut cmd = Cli::command();
        cmd.print_help()?;
        println!();
    }

    Ok(())
}

fn cmd_install(
    paths: &DarpPaths,
    config: &mut Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    println!("Running installation");

    // OS-specific resolver + nginx.conf copy
    os.init_resolver()?;
    os.ensure_dnsmasq_dir()?;
    os.copy_nginx_conf()?;
    os.write_test_conf()?;

    // Podman-specific unprivileged_port_start logic lives in engine module
    engine.configure_unprivileged_ports_if_needed()?;

    // Persist any config changes (if needed)
    config.save(&paths.config_path)?;
    Ok(())
}

fn cmd_uninstall(
    _paths: &DarpPaths,
    _config: &mut Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    println!("Running uninstallation");

    // Best-effort: stop darp containers and helper containers.
    engine.stop_running_darps()?;
    engine.stop_named_container("darp-reverse-proxy")?;
    engine.stop_named_container("darp-masq")?;

    // OS-level cleanup (resolver, etc.)
    os.uninstall()?;

    println!("Uninstall complete. Darp config.json has been left on disk.");
    Ok(())
}

fn cmd_deploy(
    paths: &DarpPaths,
    config: &mut Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    println!("Deploying Container Development\n");

    let host_gateway = engine.host_gateway();

    let domains = match &mut config.domains {
        Some(d) if !d.is_empty() => d,
        _ => {
            eprintln!("Please configure a domain.");
            std::process::exit(1);
        }
    };

    let mut hosts_container_lines = Vec::<String>::new();
    let mut portmap = serde_json::Map::new();

    let mut port_number = 50100u16;

    let host_proxy_template = r#"server {
    listen 80;
    server_name {url};
    location / {{
        proxy_pass http://{host_gateway}:{port}/;
        proxy_set_header Host $host;
    }}
}
"#;

    for (domain_name, domain) in domains.iter() {
        let location = &domain.location;
        let mut domain_map = serde_json::Map::new();

        let entries = std::fs::read_dir(location)?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let folder_name = entry.file_name().to_string_lossy().to_string();

                domain_map.insert(
                    folder_name.clone(),
                    serde_json::Value::Number(port_number.into()),
                );

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

                port_number += 1;
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
        os.sync_system_hosts(&hosts_container_lines)?;
    }

    Ok(())
}

fn cmd_shell(
    environment_name: Option<String>,
    container_image: String,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let env = environment_name
        .as_deref()
        .and_then(|name| config.environments.as_ref()?.get(name).cloned());

    let current_dir = std::env::current_dir()?;
    let current_directory_name = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let parent_directory = current_dir.parent().unwrap_or(&current_dir);
    let parent_directory_name = parent_directory
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let domain = config
        .domains
        .as_ref()
        .and_then(|d| d.get(&parent_directory_name))
        .unwrap_or_else(|| {
            eprintln!(
                "domain, {} does not exist in darp's domain configuration.",
                parent_directory_name
            );
            std::process::exit(1);
        });

    let container_name = format!("darp_{}_{}", parent_directory_name, current_directory_name);

    let mut cmd = engine.base_run_interactive(&container_name);
    cmd.arg("-v")
        .arg(format!("{}:/app", current_dir.display()))
        .arg("-v")
        .arg(format!("{}:/etc/hosts", paths.hosts_container_path.display()))
        .arg("-v")
        .arg(format!("{}:/etc/nginx/nginx.conf", paths.nginx_conf_path.display()))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/http.d/vhost_docker.conf",
            paths.vhost_container_conf.display()
        ));

    if let Some(env) = env.as_ref() {
        for v in env.volumes.iter().flatten() {
            let host = config.resolve_host_path(&v.host, &current_dir)?;
            if !host.exists() {
                eprintln!("Volume {} does not appear to exist.", v.host);
                std::process::exit(1);
            }
            cmd.arg("-v")
                .arg(format!("{}:{}", host.display(), v.container));
        }
    }

    // Host port mappings
    if let Some(pm) = domain
        .services
        .as_ref()
        .and_then(|s| s.get(&current_directory_name))
        .and_then(|service| service.host_portmappings.as_ref())
    {
        for (host_port, container_port) in pm {
            cmd.arg("-p")
                .arg(format!("{host}:{container}", host = host_port, container = container_port));
        }
    }

    // Reverse proxy port
    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    let rev_proxy_port = portmap
        .get(&parent_directory_name)
        .and_then(|d| d.get(&current_directory_name))
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            eprintln!(
                "port not yet assigned to {}, run 'darp deploy'",
                current_directory_name
            );
            std::process::exit(1);
        });

    cmd.arg("-p")
        .arg(format!("{}:8000", rev_proxy_port));

    let image_name = config.resolve_image_name(env.as_ref(), &container_image);

    let inner_cmd = r#"if command -v nginx >/dev/null 2>&1; then
    echo "Starting nginx..."; nginx;
else
    echo "nginx not found, skipping";
fi;
echo "";
echo "To leave this shell and stop the container, type: \033[33mexit\033[0m";
echo "";
cd /app; exec sh"#;

    cmd.arg(&image_name).arg("sh").arg("-c").arg(inner_cmd);

    engine.run_container_interactive(cmd, &container_name, &[137])?;
    Ok(())
}

fn cmd_serve(
    environment_name: String,
    container_image: String,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let env = config
        .environments
        .as_ref()
        .and_then(|e| e.get(&environment_name))
        .unwrap_or_else(|| {
            eprintln!("Environment '{}' does not exist.", environment_name);
            std::process::exit(1);
        });

    let serve_command = env.serve_command.as_deref().unwrap_or_else(|| {
        eprintln!(
            "Environment '{}' has no serve_command. Use 'darp set serve_command' first.",
            environment_name
        );
        std::process::exit(1);
    });

    let current_dir = std::env::current_dir()?;
    let current_directory_name = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let parent_directory = current_dir.parent().unwrap_or(&current_dir);
    let parent_directory_name = parent_directory
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let domain = config
        .domains
        .as_ref()
        .and_then(|d| d.get(&parent_directory_name))
        .unwrap_or_else(|| {
            eprintln!(
                "domain, {} does not exist in darp's domain configuration.",
                parent_directory_name
            );
            std::process::exit(1);
        });

    let container_name = format!("darp_{}_{}", parent_directory_name, current_directory_name);

    let mut cmd = engine.base_run_noninteractive(&container_name);
    cmd.arg("-v")
        .arg(format!("{}:/app", current_dir.display()))
        .arg("-v")
        .arg(format!("{}:/etc/hosts", paths.hosts_container_path.display()))
        .arg("-v")
        .arg(format!("{}:/etc/nginx/nginx.conf", paths.nginx_conf_path.display()))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/http.d/vhost_docker.conf",
            paths.vhost_container_conf.display()
        ));

    for v in env.volumes.iter().flatten() {
        let host = config.resolve_host_path(&v.host, &current_dir)?;
        if !host.exists() {
            eprintln!("Volume {} does not appear to exist.", v.host);
            std::process::exit(1);
        }
        cmd.arg("-v")
            .arg(format!("{}:{}", host.display(), v.container));
    }

    // Host port mappings
    if let Some(pm) = domain
        .services
        .as_ref()
        .and_then(|s| s.get(&current_directory_name))
        .and_then(|service| service.host_portmappings.as_ref())
    {
        for (host_port, container_port) in pm {
            cmd.arg("-p")
                .arg(format!("{host}:{container}", host = host_port, container = container_port));
        }
    }

    // Reverse proxy port
    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    let rev_proxy_port = portmap
        .get(&parent_directory_name)
        .and_then(|d| d.get(&current_directory_name))
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            eprintln!(
                "port not yet assigned to {}, run 'darp deploy'",
                current_directory_name
            );
            std::process::exit(1);
        });

    cmd.arg("-p")
        .arg(format!("{}:8000", rev_proxy_port));

    let image_name = config.resolve_image_name(Some(env), &container_image);

    let inner_cmd = format!(
        r#"if command -v nginx >/dev/null 2>&1; then
    echo "Starting nginx..."; nginx;
else
    echo "nginx not found, skipping";
fi;
cd /app; {serve}"#,
        serve = serve_command
    );

    cmd.arg(&image_name).arg("sh").arg("-c").arg(inner_cmd);

    engine.run_container_interactive(cmd, &container_name, &[0, 2])?;
    Ok(())
}

fn cmd_set(
    cmd: SetCommand,
    paths: &DarpPaths,
    config: &mut Config,
    _engine_kind: &EngineKind,
) -> anyhow::Result<()> {
    match cmd {
        SetCommand::PodmanMachine {
            new_podman_machine,
        } => {
            // Persist in config.json; env var is optional and legacy now.
            config.podman_machine = Some(new_podman_machine.clone());
            config.save(&paths.config_path)?;
            println!(
                "PODMAN_MACHINE set to '{}' in config ({}).",
                new_podman_machine,
                paths.config_path.display()
            );
        }
        SetCommand::Engine { engine } => {
            let engine_lc = engine.to_lowercase();
            if engine_lc != "podman" && engine_lc != "docker" {
                eprintln!("engine must be 'podman' or 'docker'");
                std::process::exit(1);
            }
            config.engine = Some(engine_lc);
            config.save(&paths.config_path)?;
            println!("Engine set. New Darp invocations will use this container engine.");
        }
        SetCommand::ImageRepository {
            environment,
            image_repository,
        } => {
            config.set_image_repository(&environment, &image_repository)?;
            config.save(&paths.config_path)?;
            println!(
                "Set image_repository for environment '{}' to:\n  {}",
                environment, image_repository
            );
        }
        SetCommand::ServeCommand {
            environment,
            serve_command,
        } => {
            config.set_serve_command(&environment, &serve_command)?;
            config.save(&paths.config_path)?;
            println!(
                "Set serve_command for environment '{}' to:\n  {}",
                environment, serve_command
            );
        }
        SetCommand::UrlsInHosts { value } => {
            let v = config.parse_bool(&value)?;
            config.urls_in_hosts = Some(v);
            config.save(&paths.config_path)?;
            let state = if v { "enabled" } else { "disabled" };
            println!(
                "urls_in_hosts has been {} (stored in {}). Next 'darp deploy' will sync /etc/hosts accordingly.",
                state,
                paths.config_path.display()
            );
        }
    }

    Ok(())
}

fn cmd_add(cmd: AddCommand, paths: &DarpPaths, config: &mut Config) -> anyhow::Result<()> {
    match cmd {
        AddCommand::Domain { location } => {
            config.add_domain(&location)?;
            config.save(&paths.config_path)?;
        }
        AddCommand::Environment { name } => {
            config.add_environment(&name)?;
            config.save(&paths.config_path)?;
        }
        AddCommand::Portmap {
            domain_name,
            service_name,
            host_port,
            container_port,
        } => {
            config.add_portmap(&domain_name, &service_name, &host_port, &container_port)?;
            config.save(&paths.config_path)?;
        }
        AddCommand::Volume {
            environment,
            container_dir,
            host_dir,
        } => {
            config.add_volume(&environment, &container_dir, &host_dir)?;
            config.save(&paths.config_path)?;
        }
    }

    Ok(())
}

fn cmd_rm(
    cmd: RmCommand,
    paths: &DarpPaths,
    config: &mut Config,
) -> anyhow::Result<()> {
    match cmd {
        RmCommand::PodmanMachine { } => {
            // Clear from config.json
            config.podman_machine = None;
            config.save(&paths.config_path)?;
        }
        RmCommand::Domain { name, .. } => {
            config.rm_domain(&name)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::Environment { name } => {
            config.rm_environment(&name)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::ImageRepository { environment } => {
            config.rm_image_repository(&environment)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::Portmap {
            domain_name,
            service_name,
            host_port,
            ..
        } => {
            config.rm_portmap(&domain_name, &service_name, &host_port)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::Volume {
            environment,
            container_dir,
            host_dir,
        } => {
            config.rm_volume(&environment, &container_dir, &host_dir)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::ServeCommand { environment } => {
            config.rm_serve_command(&environment)?;
            config.save(&paths.config_path)?;
        }
    }

    Ok(())
}

fn cmd_urls(paths: &DarpPaths, _config: &Config) -> anyhow::Result<()> {
    let portmap: serde_json::Value = config::read_json(&paths.portmap_path)?;
    println!();
    if let Some(obj) = portmap.as_object() {
        for (domain_name, domain) in obj.iter() {
            println!("{}", domain_name.green());
            if let Some(d) = domain.as_object() {
                let mut entries: Vec<_> = d.iter().collect();
                entries.sort_by_key(|(k, _)| *k);
                for (folder_name, port) in entries {
                    let port = port.as_u64().unwrap_or(0);
                    println!(
                        "  http://{}.{}.test ({})",
                        folder_name.blue(),
                        domain_name,
                        port
                    );
                }
            }
            println!();
        }
    }
    Ok(())
}

fn cmd_completions(shell: String) {
    use clap::CommandFactory;

    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();

    match shell.as_str() {
        "bash" => generate(shells::Bash, &mut cmd, name, &mut io::stdout()),
        "zsh" => generate(shells::Zsh, &mut cmd, name, &mut io::stdout()),
        "fish" => generate(shells::Fish, &mut cmd, name, &mut io::stdout()),
        "powershell" => generate(shells::PowerShell, &mut cmd, name, &mut io::stdout()),
        "elvish" => generate(shells::Elvish, &mut cmd, name, &mut io::stdout()),
        other => {
            eprintln!("Unsupported shell: {}", other);
            std::process::exit(1);
        }
    }
}
