// main.rs
mod config;
mod engine;
mod os;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, shells};
use colored::*;
use dirs::home_dir;
use std::fs;
use std::io::Write;
use std::path::Path;

use crate::config::{Config, DarpPaths, Domain, Environment, Group, ResolvedSettings, Service};
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
    /// Configuration commands that modify config.json
    Config {
        #[command(subcommand)]
        cmd: ConfigCommand,
    },
    /// Generates domains and starts reverse proxy
    Deploy,
    /// Runs the environment serve_command (uses domain default_environment if set)
    Serve {
        /// Environment name (optional; falls back to domain default_environment if configured)
        #[arg(short, long)]
        environment: Option<String>,
        /// Print the generated container command and exit without running it
        #[arg(long)]
        dry_run: bool,
        /// Container image to use (optional if default_container_image is configured)
        container_image: Option<String>,
    },
    /// Starts a shell instance (uses service/environment shell_command if set, otherwise 'sh')
    Shell {
        /// Environment name (optional)
        #[arg(short, long)]
        environment: Option<String>,
        /// Print the generated container command and exit without running it
        #[arg(long)]
        dry_run: bool,
        /// Container image to use (optional if default_container_image is configured)
        container_image: Option<String>,
    },
    /// List Darp URLs
    Urls,
    /// Install darp system installation
    Install,
    /// Uninstall darp system integration
    Uninstall,
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Set values in config
    Set {
        #[command(subcommand)]
        cmd: SetCommand,
    },
    /// Add to config
    Add {
        #[command(subcommand)]
        cmd: AddCommand,
    },
    /// Remove from config
    Rm {
        #[command(subcommand)]
        cmd: RmCommand,
    },
    /// Show the effective resolved configuration for the current directory
    Show {
        /// Environment name (optional; falls back to domain's default_environment)
        #[arg(short, long)]
        environment: Option<String>,
    },
    /// Pull latest changes for all pre_config repos
    Pull,
}

#[derive(Subcommand, Debug)]
enum SetCommand {
    /// Set container engine (podman|docker)
    Engine {
        engine: String,
    },
    /// Set image_repository / serve_command / shell_command / platform / default_container_image on an environment
    Env {
        #[command(subcommand)]
        cmd: SetEnvCommand,
    },
    /// Set image_repository / serve_command / shell_command / platform / default_container_image on a service
    Svc {
        #[command(subcommand)]
        cmd: SetSvcCommand,
    },
    /// Set domain-level properties
    Dom {
        #[command(subcommand)]
        cmd: SetDomCommand,
    },
    /// Set group-level properties
    Grp {
        #[command(subcommand)]
        cmd: SetGrpCommand,
    },
    /// Set Podman machine name
    PodmanMachine {
        /// Name of the Podman machine to use (e.g. 'podman-machine-default')
        new_podman_machine: String,
    },
    /// Enable/disable mirroring URLs into /etc/hosts
    UrlsInHosts {
        value: String,
    },
}

#[derive(Subcommand, Debug)]
enum SetDomCommand {
    /// Set default_environment on a domain
    DefaultEnvironment {
        /// Logical domain name (e.g. 'my-domain')
        domain_name: String,
        /// Environment name to use by default for this domain
        default_environment: String,
    },
    /// Set image_repository on a domain
    ImageRepository {
        domain_name: String,
        image_repository: String,
    },
    /// Set serve_command on a domain
    ServeCommand {
        domain_name: String,
        serve_command: String,
    },
    /// Set shell_command on a domain (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        shell_command: String,
    },
    /// Set platform architecture (e.g., linux/amd64) on a domain
    Platform {
        domain_name: String,
        platform: String,
    },
    /// Set default_container_image on a domain (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        default_container_image: String,
    },
}

#[derive(Subcommand, Debug)]
enum SetGrpCommand {
    /// Set default_environment on a group
    DefaultEnvironment {
        domain_name: String,
        group_name: String,
        default_environment: String,
    },
    /// Set image_repository on a group
    ImageRepository {
        domain_name: String,
        group_name: String,
        image_repository: String,
    },
    /// Set serve_command on a group
    ServeCommand {
        domain_name: String,
        group_name: String,
        serve_command: String,
    },
    /// Set shell_command on a group (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        group_name: String,
        shell_command: String,
    },
    /// Set platform architecture (e.g., linux/amd64) on a group
    Platform {
        domain_name: String,
        group_name: String,
        platform: String,
    },
    /// Set default_container_image on a group (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
        default_container_image: String,
    },
}

#[derive(Subcommand, Debug)]
enum SetEnvCommand {
    /// Set image_repository on an environment
    ImageRepository {
        environment: String,
        image_repository: String,
    },
    /// Set serve_command on an environment
    ServeCommand {
        environment: String,
        serve_command: String,
    },
    /// Set shell_command on an environment (used by `darp shell`)
    ShellCommand {
        environment: String,
        shell_command: String,
    },
    /// Set platform architecture (e.g., linux/amd64) on an environment
    Platform {
        environment: String,
        platform: String,
    },
    /// Set default_container_image on an environment (used when no image is passed on the CLI)
    DefaultContainerImage {
        environment: String,
        default_container_image: String,
    },
}

#[derive(Subcommand, Debug)]
enum SetSvcCommand {
    /// Set image_repository on a service
    ImageRepository {
        domain_name: String,
        group_name: String,
        service_name: String,
        image_repository: String,
    },
    /// Set serve_command on a service
    ServeCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
        serve_command: String,
    },
    /// Set shell_command on a service (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
        shell_command: String,
    },
    /// Set platform architecture (e.g., linux/amd64) on a service
    Platform {
        domain_name: String,
        group_name: String,
        service_name: String,
        platform: String,
    },
    /// Set default_container_image on a service (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
        service_name: String,
        default_container_image: String,
    },
}

#[derive(Subcommand, Debug)]
enum AddCommand {
    /// Add a pre_config entry (parent config for chaining)
    PreConfig {
        /// Path to the config file (supports {home} token)
        location: String,
        /// Path to git repo for `darp config pull` (supports {home} token)
        #[arg(short, long)]
        repo_location: Option<String>,
    },
    /// Add a domain
    Domain {
        /// Logical domain name (e.g. 'my-project')
        name: String,
        /// Location of the domain folder (supports {home} token)
        location: String,
    },
    /// Add domain-scoped configuration (volumes, port mappings, variables)
    Dom {
        #[command(subcommand)]
        cmd: AddDomCommand,
    },
    /// Add group-scoped configuration
    Grp {
        #[command(subcommand)]
        cmd: AddGrpCommand,
    },
    /// Add environment-scoped configuration (volumes, port mappings, variables). Environments
    /// are created automatically as needed.
    Env {
        #[command(subcommand)]
        cmd: AddEnvCommand,
    },
    /// Add service-scoped configuration (volumes, port mappings, variables)
    Svc {
        #[command(subcommand)]
        cmd: AddSvcCommand,
    },
}

#[derive(Subcommand, Debug)]
enum AddDomCommand {
    /// Add port mapping to a domain
    Portmap {
        domain_name: String,
        host_port: String,
        container_port: String,
    },
    /// Add variable to a domain
    Variable {
        domain_name: String,
        name: String,
        value: String,
    },
    /// Add volume to a domain
    Volume {
        domain_name: String,
        container_dir: String,
        host_dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum AddGrpCommand {
    /// Create a new group in a domain
    Group {
        domain_name: String,
        group_name: String,
    },
    /// Add port mapping to a group
    Portmap {
        domain_name: String,
        group_name: String,
        host_port: String,
        container_port: String,
    },
    /// Add variable to a group
    Variable {
        domain_name: String,
        group_name: String,
        name: String,
        value: String,
    },
    /// Add volume to a group
    Volume {
        domain_name: String,
        group_name: String,
        container_dir: String,
        host_dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum AddEnvCommand {
    /// Add port mapping to an environment (auto-creates environment if needed)
    Portmap {
        environment: String,
        host_port: String,
        container_port: String,
    },
    /// Add variable to an environment (auto-creates environment if needed)
    Variable {
        environment: String,
        name: String,
        value: String,
    },
    /// Add volume to an environment (auto-creates environment if needed)
    Volume {
        environment: String,
        container_dir: String,
        host_dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum AddSvcCommand {
    /// Add port mapping to a service
    Portmap {
        domain_name: String,
        group_name: String,
        service_name: String,
        host_port: String,
        container_port: String,
    },
    /// Add variable to a service
    Variable {
        domain_name: String,
        group_name: String,
        service_name: String,
        name: String,
        value: String,
    },
    /// Add volume to a service
    Volume {
        domain_name: String,
        group_name: String,
        service_name: String,
        container_dir: String,
        host_dir: String,
    },
}

#[derive(Subcommand, Debug)]
enum RmCommand {
    /// Remove a domain
    Domain {
        name: String,
    },
    /// Remove a pre_config entry by its location
    PreConfig {
        /// Path to the config file to remove
        location: String,
    },
    /// Remove PODMAN_MACHINE from config
    PodmanMachine {},
    /// Remove domain-level configuration
    Dom {
        #[command(subcommand)]
        cmd: RmDomCommand,
    },
    /// Remove group-level configuration
    Grp {
        #[command(subcommand)]
        cmd: RmGrpCommand,
    },
    /// Remove environment-scoped configuration
    Env {
        #[command(subcommand)]
        cmd: RmEnvCommand,
    },
    /// Remove service-scoped configuration
    Svc {
        #[command(subcommand)]
        cmd: RmSvcCommand,
    },
}

#[derive(Subcommand, Debug)]
enum RmDomCommand {
    /// Remove default_environment from a domain
    DefaultEnvironment {
        /// Logical domain name (e.g. 'my-domain')
        domain_name: String,
    },
    /// Remove port mapping from a domain
    Portmap {
        domain_name: String,
        host_port: String,
    },
    /// Remove variable from a domain
    Variable {
        domain_name: String,
        name: String,
    },
    /// Remove volume from a domain
    Volume {
        domain_name: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from a domain
    ServeCommand {
        domain_name: String,
    },
    /// Remove shell_command from a domain
    ShellCommand {
        domain_name: String,
    },
    /// Remove image_repository from a domain
    ImageRepository {
        domain_name: String,
    },
    /// Remove platform architecture from a domain
    Platform {
        domain_name: String,
    },
    /// Remove default_container_image from a domain
    DefaultContainerImage {
        domain_name: String,
    },
}

#[derive(Subcommand, Debug)]
enum RmGrpCommand {
    /// Remove a group from a domain
    Group {
        domain_name: String,
        group_name: String,
    },
    /// Remove default_environment from a group
    DefaultEnvironment {
        domain_name: String,
        group_name: String,
    },
    /// Remove port mapping from a group
    Portmap {
        domain_name: String,
        group_name: String,
        host_port: String,
    },
    /// Remove variable from a group
    Variable {
        domain_name: String,
        group_name: String,
        name: String,
    },
    /// Remove volume from a group
    Volume {
        domain_name: String,
        group_name: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from a group
    ServeCommand {
        domain_name: String,
        group_name: String,
    },
    /// Remove shell_command from a group
    ShellCommand {
        domain_name: String,
        group_name: String,
    },
    /// Remove image_repository from a group
    ImageRepository {
        domain_name: String,
        group_name: String,
    },
    /// Remove platform architecture from a group
    Platform {
        domain_name: String,
        group_name: String,
    },
    /// Remove default_container_image from a group
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
    },
}

#[derive(Subcommand, Debug)]
enum RmEnvCommand {
    /// Remove port mapping from an environment
    Portmap {
        environment: String,
        host_port: String,
    },
    /// Remove variable from an environment
    Variable {
        environment: String,
        name: String,
    },
    /// Remove volume from an environment
    Volume {
        environment: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from an environment
    ServeCommand {
        environment: String,
    },
    /// Remove shell_command from an environment
    ShellCommand {
        environment: String,
    },
    /// Remove image_repository from an environment
    ImageRepository {
        environment: String,
    },
    /// Remove platform architecture from an environment
    Platform {
        environment: String,
    },
    /// Remove default_container_image from an environment
    DefaultContainerImage {
        environment: String,
    },
}

#[derive(Subcommand, Debug)]
enum RmSvcCommand {
    /// Remove port mapping from a service
    Portmap {
        domain_name: String,
        group_name: String,
        service_name: String,
        host_port: String,
    },
    /// Remove variable from a service
    Variable {
        domain_name: String,
        group_name: String,
        service_name: String,
        name: String,
    },
    /// Remove volume from a service
    Volume {
        domain_name: String,
        group_name: String,
        service_name: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from a service
    ServeCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
    },
    /// Remove shell_command from a service
    ShellCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
    },
    /// Remove image_repository from a service
    ImageRepository {
        domain_name: String,
        group_name: String,
        service_name: String,
    },
    /// Remove platform architecture from a service
    Platform {
        domain_name: String,
        group_name: String,
        service_name: String,
    },
    /// Remove default_container_image from a service
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
        service_name: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Paths
    let paths = DarpPaths::from_env()?;

    if let Some(cmd) = cli.command {
        match cmd {
            Command::Config { cmd } => {
                match cmd {
                    ConfigCommand::Show { environment } => {
                        // Show uses merged config
                        let config = Config::load_merged(&paths.config_path)?;
                        cmd_show(environment, &config)?;
                    }
                    ConfigCommand::Pull => {
                        let config = Config::load(&paths.config_path)?;
                        cmd_pull(&config)?;
                    }
                    _ => {
                        // Config mutations use leaf config only
                        let mut config = Config::load(&paths.config_path)?;
                        let engine_kind = EngineKind::from_config(&config);
                        match cmd {
                            ConfigCommand::Set { cmd } => cmd_set(cmd, &paths, &mut config, &engine_kind)?,
                            ConfigCommand::Add { cmd } => cmd_add(cmd, &paths, &mut config)?,
                            ConfigCommand::Rm { cmd } => cmd_rm(cmd, &paths, &mut config)?,
                            ConfigCommand::Show { .. } | ConfigCommand::Pull => unreachable!(),
                        }
                    }
                }
            }
            _ => {
                // Runtime commands use merged config
                let config = Config::load_merged(&paths.config_path)?;
                let engine_kind = EngineKind::from_config(&config);
                let engine = Engine::new(engine_kind.clone(), &config)?;
                let os = OsIntegration::new(&paths, &config, &engine_kind);
                match cmd {
                    Command::Install => cmd_install(&paths, &mut config.clone(), &os, &engine)?,
                    Command::Uninstall => cmd_uninstall(&paths, &mut config.clone(), &os, &engine)?,
                    Command::Deploy => cmd_deploy(&paths, &config, &os, &engine)?,
                    Command::Shell {
                        environment,
                        dry_run,
                        container_image,
                    } => cmd_shell(environment, dry_run, container_image, &paths, &config, &engine)?,
                    Command::Serve {
                        environment,
                        dry_run,
                        container_image,
                    } => cmd_serve(environment, dry_run, container_image, &paths, &config, &engine)?,
                    Command::Urls => cmd_urls(&paths, &config)?,
                    Command::Config { .. } => unreachable!(),
                }
            }
        }
    } else {
        // No subcommand: print help
        let mut cmd = Cli::command();
        cmd.print_help()?;
        println!();
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Shell detection & rc helpers
// ---------------------------------------------------------------------------

const RC_START_MARKER: &str = "# >>> darp completion start >>>";
const RC_END_MARKER: &str = "# <<< darp completion end <<<";

fn detect_shell() -> Option<&'static str> {
    if let Ok(shell_path) = std::env::var("SHELL") {
        if shell_path.ends_with("zsh") {
            Some("zsh")
        } else if shell_path.ends_with("bash") {
            Some("bash")
        } else if shell_path.ends_with("fish") {
            Some("fish")
        } else if shell_path.ends_with("pwsh") || shell_path.ends_with("powershell") {
            Some("powershell")
        } else if shell_path.ends_with("elvish") {
            Some("elvish")
        } else {
            None
        }
    } else {
        None
    }
}

fn ensure_rc_block(rc_path: &Path, body: &str) -> anyhow::Result<()> {
    let contents = fs::read_to_string(rc_path).unwrap_or_default();

    if contents.contains(RC_START_MARKER) {
        return Ok(());
    }

    let mut new_contents = contents;
    if !new_contents.is_empty() && !new_contents.ends_with('\n') {
        new_contents.push('\n');
    }

    new_contents.push_str(RC_START_MARKER);
    new_contents.push('\n');
    new_contents.push_str(body);
    if !body.ends_with('\n') {
        new_contents.push('\n');
    }
    new_contents.push_str(RC_END_MARKER);
    new_contents.push('\n');

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(rc_path, new_contents)?;
    Ok(())
}

fn remove_rc_block(rc_path: &Path) -> anyhow::Result<()> {
    let contents = match fs::read_to_string(rc_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    let start = if let Some(s) = contents.find(RC_START_MARKER) {
        s
    } else {
        return Ok(());
    };

    // Find end after start
    let after_start = &contents[start..];
    let end_rel = if let Some(e) = after_start.find(RC_END_MARKER) {
        e + RC_END_MARKER.len()
    } else {
        contents.len() - start
    };
    let end = start + end_rel;

    let mut new_contents = String::new();
    new_contents.push_str(contents[..start].trim_end_matches('\n'));
    if !new_contents.is_empty() {
        new_contents.push('\n');
    }
    let tail = contents[end..].trim_start_matches('\n');
    if !tail.is_empty() {
        if !new_contents.is_empty() {
            new_contents.push('\n');
        }
        new_contents.push_str(tail);
        new_contents.push('\n');
    }

    fs::write(rc_path, new_contents)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Shell completion install/uninstall
// ---------------------------------------------------------------------------

fn install_shell_completions() -> anyhow::Result<()> {
    let Some(shell) = detect_shell() else {
        println!("Could not detect shell from $SHELL; skipping shell completion install.");
        return Ok(());
    };

    let Some(home) = home_dir() else {
        println!("Could not determine home directory; skipping shell completion install.");
        return Ok(());
    };

    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();

    match shell {
        "bash" => {
            let dir = home.join(".local/share/bash-completion/completions");
            fs::create_dir_all(&dir)?;
            let path = dir.join("darp");
            let mut file = fs::File::create(&path)?;
            generate(shells::Bash, &mut cmd, name, &mut file);
            println!("Installed bash completions to {}", path.display());

            // Wire into ~/.bashrc
            let rc_path = home.join(".bashrc");
            let body = r#"if command -v darp >/dev/null 2>&1; then
  source "${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/darp"
fi"#;
            ensure_rc_block(&rc_path, body)?;
            println!("Updated {} with darp completion block", rc_path.display());
        }
        "zsh" => {
            let dir = home.join(".zfunc");
            fs::create_dir_all(&dir)?;
            let path = dir.join("_darp");
            let mut file = fs::File::create(&path)?;
            generate(shells::Zsh, &mut cmd, name, &mut file);
            println!("Installed zsh completions to {}", path.display());

            // Wire into ~/.zshrc
            let rc_path = home.join(".zshrc");
            let body = r#"if command -v darp >/dev/null 2>&1; then
  fpath+=("$HOME/.zfunc")
  autoload -Uz compinit
  compinit
fi"#;
            ensure_rc_block(&rc_path, body)?;
            println!("Updated {} with darp completion block", rc_path.display());
        }
        "fish" => {
            let dir = home.join(".config/fish/completions");
            fs::create_dir_all(&dir)?;
            let path = dir.join("darp.fish");
            let mut file = fs::File::create(&path)?;
            generate(shells::Fish, &mut cmd, name, &mut file);
            println!("Installed fish completions to {}", path.display());
            println!("Fish automatically loads completions from ~/.config/fish/completions.");
        }
        "powershell" => {
            println!("PowerShell completion installation is not yet automated; skipping.");
        }
        "elvish" => {
            println!("Elvish completion installation is not yet automated; skipping.");
        }
        other => {
            println!(
                "Shell '{}' not supported for automatic completions; skipping.",
                other
            );
        }
    }

    Ok(())
}

fn uninstall_shell_completions() -> anyhow::Result<()> {
    let Some(shell) = detect_shell() else {
        println!("Could not detect shell from $SHELL; skipping shell completion removal.");
        return Ok(());
    };

    let Some(home) = home_dir() else {
        println!("Could not determine home directory; skipping shell completion removal.");
        return Ok(());
    };

    match shell {
        "bash" => {
            let path = home.join(".local/share/bash-completion/completions/darp");
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(e.into());
                    }
                } else {
                    println!("Removed bash completions at {}", path.display());
                }
            }
            let rc_path = home.join(".bashrc");
            remove_rc_block(&rc_path)?;
        }
        "zsh" => {
            let path = home.join(".zfunc/_darp");
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(e.into());
                    }
                } else {
                    println!("Removed zsh completions at {}", path.display());
                }
            }
            let rc_path = home.join(".zshrc");
            remove_rc_block(&rc_path)?;
        }
        "fish" => {
            let path = home.join(".config/fish/completions/darp.fish");
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        return Err(e.into());
                    }
                } else {
                    println!("Removed fish completions at {}", path.display());
                }
            }
            // No rc modification needed for fish.
        }
        _ => {
            println!(
                "Shell '{}' not supported for automatic completion removal; skipping.",
                shell
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Platform helper
// ---------------------------------------------------------------------------

fn add_platform_args(
    cmd: &mut std::process::Command,
    engine: &Engine,
    platform: &str,
) {
    match engine.kind {
        EngineKind::Docker => {
            // Docker expects full platform string, e.g. linux/amd64
            cmd.arg("--platform").arg(platform);
        }
        EngineKind::Podman => {
            // For Podman, map "os/arch" to --os / --arch; if no slash, treat as arch only.
            let parts: Vec<&str> = platform.split('/').collect();
            if parts.len() >= 2 {
                let os = parts[0];
                let arch = parts[1];
                cmd.arg("--os").arg(os);
                cmd.arg("--arch").arg(arch);
            } else {
                cmd.arg("--arch").arg(platform);
            }
        }
        EngineKind::None => {} // No engine configured; nothing to do here. require_ready will error earlier.
    }
}

/// Resolve the "base" image name to use, applying the precedence:
/// 1) CLI-provided image
/// 2) service.default_container_image
/// 3) environment.default_container_image
/// If none are set, print a helpful message and exit(1).
fn resolve_base_image(
    cli_image: Option<&str>,
    env: Option<&Environment>,
    group: Option<&Group>,
    domain: &Domain,
    service: Option<&Service>,
    env_name: Option<&str>,
    domain_name: &str,
    service_folder_name: &str,
    command_name: &str,
) -> String {
    if let Some(img) = cli_image {
        return img.to_string();
    }

    let from_service = service.and_then(|s| s.default_container_image.as_deref());
    let from_group = group.and_then(|g| g.default_container_image.as_deref());
    let from_domain = domain.default_container_image.as_deref();
    let from_env = env.and_then(|e| e.default_container_image.as_deref());

    if let Some(img) = from_service.or(from_group).or(from_domain).or(from_env) {
        return img.to_string();
    }

    match env_name {
        Some(env_name) => {
            eprintln!(
                "No container image provided for '{}.{}' in environment '{}'.\n\
                 Either pass an explicit image to 'darp {cmd}' or configure a default_container_image:\n\
                   darp config set svc default-container-image {domain} {service} <image>\n\
                 or\n\
                   darp config set env default-container-image {env} <image>",
                domain_name,
                service_folder_name,
                env_name,
                cmd = command_name,
                domain = domain_name,
                service = service_folder_name,
                env = env_name,
            );
        }
        None => {
            eprintln!(
                "No container image provided for '{}.{}'.\n\
                 Either pass an explicit image to 'darp {cmd}' or configure a default_container_image:\n\
                   darp config set svc default-container-image {domain} {service} <image>\n\
                 or\n\
                   darp config set env default-container-image <env> <image>",
                domain_name,
                service_folder_name,
                cmd = command_name,
                domain = domain_name,
                service = service_folder_name,
            );
        }
    }

    std::process::exit(1);
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

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

    // Install shell completions for detected shell and update rc files
    install_shell_completions()?;

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

    // Remove shell completions & rc entries
    uninstall_shell_completions()?;

    println!("Uninstall complete. Darp config.json has been left on disk.");
    Ok(())
}

fn cmd_deploy(
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
        let register_service =
            |folder_name: &str,
             port_number: &mut u16,
             domain_map: &mut serde_json::Map<String, serde_json::Value>,
             hosts_container_lines: &mut Vec<String>|
             -> anyhow::Result<()> {
                domain_map.insert(
                    folder_name.to_string(),
                    serde_json::Value::Number((*port_number).into()),
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

                *port_number += 1;
                Ok(())
            };

        // Scan "." group: direct children of domain location, excluding group subdirs
        if groups.map_or(true, |g| g.contains_key(".")) {
            if let Ok(entries) = std::fs::read_dir(&location) {
                for entry in entries {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        if !group_names.contains(&folder_name) {
                            register_service(
                                &folder_name,
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
        // Note: we sync directly to the system hosts file using OsIntegration.
        // We only map to 127.0.0.1 here; the container sees 0.0.0.0 above.
        let os = OsIntegration::new(paths, config, &engine.kind);
        os.sync_system_hosts(&hosts_container_lines)?;
    }

    Ok(())
}

fn cmd_shell(
    environment_cli: Option<String>,
    dry_run: bool,
    container_image: Option<String>,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let current_dir = std::env::current_dir()?;
    let current_directory_name = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (domain_name, domain, _group_name, group_opt) = config
        .find_context_by_cwd(&current_dir)
        .unwrap_or_else(|| {
            eprintln!(
                "Current directory does not exist in any darp domain configuration."
            );
            std::process::exit(1);
        });
    let domain_name = domain_name.to_string();

    let effective_env_name: Option<String> = environment_cli
        .or_else(|| group_opt.and_then(|g| g.default_environment.clone()))
        .or_else(|| domain.default_environment.clone());

    let env = if let Some(ref env_name) = effective_env_name {
        let env_opt = config
            .environments
            .as_ref()
            .and_then(|e| e.get(env_name).cloned());
        if env_opt.is_none() {
            eprintln!("Environment '{}' does not exist.", env_name);
            std::process::exit(1);
        }
        env_opt
    } else {
        None
    };

    let service_opt = group_opt
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(&current_directory_name));

    let container_name = format!("darp_{}_{}", domain_name, current_directory_name);

    let mut cmd = engine.base_run_interactive(&container_name);

    if engine.is_docker() {
        cmd.arg("--add-host")
            .arg("host.docker.internal:host-gateway");
    }

    cmd.arg("-v")
        .arg(format!("{}:/app", current_dir.display()))
        .arg("-v")
        .arg(format!("{}:/etc/hosts", paths.hosts_container_path.display()))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/nginx.conf",
            paths.nginx_conf_path.display()
        ))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/http.d/vhost_container.conf",
            paths.vhost_container_conf.display()
        ));

    // Volumes: service > group > domain > environment
    let effective_volumes = service_opt
        .and_then(|s| s.volumes.as_ref())
        .or_else(|| group_opt.and_then(|g| g.volumes.as_ref()))
        .or_else(|| domain.volumes.as_ref())
        .or_else(|| env.as_ref().and_then(|e| e.volumes.as_ref()));

    if let Some(vols) = effective_volumes {
        for v in vols {
            let host = config.resolve_host_path(&v.host, &current_dir)?;
            if !host.exists() {
                eprintln!("Volume {} does not appear to exist.", v.host);
                std::process::exit(1);
            }
            cmd.arg("-v")
                .arg(format!("{}:{}", host.display(), v.container));
        }
    }

    // Host port mappings: service > group > domain > environment
    let host_portmaps = service_opt
        .and_then(|s| s.host_portmappings.as_ref())
        .or_else(|| group_opt.and_then(|g| g.host_portmappings.as_ref()))
        .or_else(|| domain.host_portmappings.as_ref())
        .or_else(|| env.as_ref().and_then(|e| e.host_portmappings.as_ref()));

    if let Some(pm) = host_portmaps {
        for (host_port, container_port) in pm {
            cmd.arg("-p")
                .arg(format!("{host}:{container}", host = host_port, container = container_port));
        }
    }

    // Variables: service > group > domain > environment
    let variables = service_opt
        .and_then(|s| s.variables.as_ref())
        .or_else(|| group_opt.and_then(|g| g.variables.as_ref()))
        .or_else(|| domain.variables.as_ref())
        .or_else(|| env.as_ref().and_then(|e| e.variables.as_ref()));

    if let Some(v) = variables {
        for (name, value) in v {
            cmd.arg("-e")
                .arg(format!("{name}={value}", name = name, value = value));
        }
    }

    // Platform: service > group > domain > environment
    let platform = service_opt
        .and_then(|s| s.platform.as_deref())
        .or_else(|| group_opt.and_then(|g| g.platform.as_deref()))
        .or_else(|| domain.platform.as_deref())
        .or_else(|| env.as_ref().and_then(|e| e.platform.as_deref()));

    if let Some(platform) = platform {
        add_platform_args(&mut cmd, engine, platform);
    }

    // --- Reverse proxy port (unchanged) ---
    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    let rev_proxy_port = portmap
        .get(&domain_name)
        .and_then(|d| d.get(&current_directory_name))
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            eprintln!(
                "port not yet assigned to {}, run 'darp deploy'",
                current_directory_name
            );
            std::process::exit(1);
        });

    cmd.arg("-p").arg(format!("{}:8000", rev_proxy_port));

    let base_image = resolve_base_image(
        container_image.as_deref(),
        env.as_ref(),
        group_opt,
        domain,
        service_opt,
        effective_env_name.as_deref(),
        &domain_name,
        &current_directory_name,
        "shell",
    );

    let image_name = config.resolve_image_name(env.as_ref(), group_opt, Some(domain), service_opt, &base_image);

    let shell_command = service_opt
        .and_then(|s| s.shell_command.as_deref())
        .or_else(|| group_opt.and_then(|g| g.shell_command.as_deref()))
        .or_else(|| domain.shell_command.as_deref())
        .or_else(|| env.as_ref().and_then(|e| e.shell_command.as_deref()))
        .unwrap_or("sh");

    let inner_cmd = format!(
        r#"if command -v nginx >/dev/null 2>&1; then
    echo "Starting nginx..."; nginx;
else
    echo "nginx not found, skipping";
fi;
echo "";
echo "To leave this shell and stop the container, type: $(printf '\033[33m')exit$(printf '\033[0m')"
echo "";
cd /app; exec {shell}"#,
        shell = shell_command
    );

    cmd.arg(&image_name).arg("sh").arg("-c").arg(inner_cmd);

    if dry_run {
        println!("{}", engine.command_to_string(&cmd));
        return Ok(());
    }

    engine.run_container_interactive(cmd, &container_name, &[])?;
    Ok(())
}

fn cmd_serve(
    environment_cli: Option<String>,
    dry_run: bool,
    container_image: Option<String>,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let current_dir = std::env::current_dir()?;
    let current_directory_name = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (domain_name, domain, _group_name, group_opt) = config
        .find_context_by_cwd(&current_dir)
        .unwrap_or_else(|| {
            eprintln!(
                "Current directory does not exist in any darp domain configuration."
            );
            std::process::exit(1);
        });
    let domain_name = domain_name.to_string();

    let service_opt = group_opt
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(&current_directory_name));

    let effective_env_name = environment_cli
        .or_else(|| group_opt.and_then(|g| g.default_environment.clone()))
        .or_else(|| domain.default_environment.clone());

    let environment_name = match effective_env_name {
        Some(name) => name,
        None => {
            eprintln!(
                "Environment is required for 'darp serve' in domain '{}'.\n\
Either pass an explicit environment:\n  darp serve --environment <env>\n\
or configure a default_environment for this domain:\n  darp config set dom default-environment {} <env>",
                domain_name,
                domain_name
            );
            std::process::exit(1);
        }
    };

    let env = config
        .environments
        .as_ref()
        .and_then(|e| e.get(&environment_name))
        .unwrap_or_else(|| {
            eprintln!("Environment '{}' does not exist.", environment_name);
            std::process::exit(1);
        });

    let serve_command = service_opt
        .and_then(|svc| svc.serve_command.as_deref())
        .or_else(|| group_opt.and_then(|g| g.serve_command.as_deref()))
        .or_else(|| domain.serve_command.as_deref())
        .or_else(|| env.serve_command.as_deref())
        .unwrap_or_else(|| {
            eprintln!(
                "Neither service '{}.{}', domain '{}', nor environment '{}' has a serve_command configured.\n\
Use 'darp config set svc serve-command {} {} <cmd>' or \
'darp config set dom serve-command {} <cmd>' or \
'darp config set env serve-command {} <cmd>' first.",
                domain_name,
                current_directory_name,
                domain_name,
                environment_name,
                domain_name,
                current_directory_name,
                domain_name,
                environment_name,
            );
            std::process::exit(1);
        });

    let container_name = format!("darp_{}_{}", domain_name, current_directory_name);
    let mut cmd = engine.base_run_noninteractive(&container_name);

    if engine.is_docker() {
        cmd.arg("--add-host")
            .arg("host.docker.internal:host-gateway");
    }

    cmd.arg("-v")
        .arg(format!("{}:/app", current_dir.display()))
        .arg("-v")
        .arg(format!("{}:/etc/hosts", paths.hosts_container_path.display()))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/nginx.conf",
            paths.nginx_conf_path.display()
        ))
        .arg("-v")
        .arg(format!(
            "{}:/etc/nginx/http.d/vhost_container.conf",
            paths.vhost_container_conf.display()
        ));

    // Volumes: service > domain > environment
    let effective_volumes = service_opt
        .and_then(|s| s.volumes.as_ref())
        .or_else(|| group_opt.and_then(|g| g.volumes.as_ref()))
        .or_else(|| domain.volumes.as_ref())
        .or_else(|| env.volumes.as_ref());

    if let Some(vols) = effective_volumes {
        for v in vols {
            let host = config.resolve_host_path(&v.host, &current_dir)?;
            if !host.exists() {
                eprintln!("Volume {} does not appear to exist.", v.host);
                std::process::exit(1);
            }
            cmd.arg("-v")
                .arg(format!("{}:{}", host.display(), v.container));
        }
    }

    // Host port mappings: service > group > domain > environment
    let host_portmaps = service_opt
        .and_then(|s| s.host_portmappings.as_ref())
        .or_else(|| group_opt.and_then(|g| g.host_portmappings.as_ref()))
        .or_else(|| domain.host_portmappings.as_ref())
        .or_else(|| env.host_portmappings.as_ref());

    if let Some(pm) = host_portmaps {
        for (host_port, container_port) in pm {
            cmd.arg("-p")
                .arg(format!("{host}:{container}", host = host_port, container = container_port));
        }
    }

    // Variables: service > group > domain > environment
    let variables = service_opt
        .and_then(|s| s.variables.as_ref())
        .or_else(|| group_opt.and_then(|g| g.variables.as_ref()))
        .or_else(|| domain.variables.as_ref())
        .or_else(|| env.variables.as_ref());

    if let Some(v) = variables {
        for (name, value) in v {
            cmd.arg("-e")
                .arg(format!("{name}={value}", name = name, value = value));
        }
    }

    // Platform: service > group > domain > environment
    let platform = service_opt
        .and_then(|svc| svc.platform.as_deref())
        .or_else(|| group_opt.and_then(|g| g.platform.as_deref()))
        .or_else(|| domain.platform.as_deref())
        .or_else(|| env.platform.as_deref());

    if let Some(platform) = platform {
        add_platform_args(&mut cmd, engine, platform);
    }

    // Reverse proxy port
    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    let rev_proxy_port = portmap
        .get(&domain_name)
        .and_then(|d| d.get(&current_directory_name))
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| {
            eprintln!(
                "port not yet assigned to {}, run 'darp deploy'",
                current_directory_name
            );
            std::process::exit(1);
        });

    cmd.arg("-p").arg(format!("{}:8000", rev_proxy_port));

    let base_image = resolve_base_image(
        container_image.as_deref(),
        Some(env),
        group_opt,
        domain,
        service_opt,
        Some(&environment_name),
        &domain_name,
        &current_directory_name,
        "serve",
    );

    let image_name = config.resolve_image_name(Some(env), group_opt, Some(domain), service_opt, &base_image);

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

    if dry_run {
        println!("{}", engine.command_to_string(&cmd));
        return Ok(());
    }

    engine.run_container_interactive(cmd, &container_name, &[])?;
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
        SetCommand::Env { cmd } => match cmd {
            SetEnvCommand::ImageRepository {
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
            SetEnvCommand::ServeCommand {
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
            SetEnvCommand::ShellCommand {
                environment,
                shell_command,
            } => {
                config.set_shell_command(&environment, &shell_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set shell_command for environment '{}' to:\n  {}",
                    environment, shell_command
                );
            }
            SetEnvCommand::Platform {
                environment,
                platform,
            } => {
                config.set_platform(&environment, &platform)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set platform for environment '{}' to:\n  {}",
                    environment, platform
                );
            }
            SetEnvCommand::DefaultContainerImage {
                environment,
                default_container_image,
            } => {
                config.set_default_container_image(&environment, &default_container_image)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_container_image for environment '{}' to:\n  {}",
                    environment, default_container_image
                );
            }
        },
        SetCommand::Svc { cmd } => match cmd {
            SetSvcCommand::ImageRepository {
                domain_name,
                group_name,
                service_name,
                image_repository,
            } => {
                config.set_service_image_repository(
                    &domain_name,
                    &group_name,
                    &service_name,
                    &image_repository,
                )?;
                config.save(&paths.config_path)?;
                println!(
                    "Set image_repository for service '{}.{}' to:\n  {}",
                    domain_name, service_name, image_repository
                );
            }
            SetSvcCommand::ServeCommand {
                domain_name,
                group_name,
                service_name,
                serve_command,
            } => {
                config.set_service_serve_command(&domain_name, &group_name, &service_name, &serve_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set serve_command for service '{}.{}' to:\n  {}",
                    domain_name, service_name, serve_command
                );
            }
            SetSvcCommand::ShellCommand {
                domain_name,
                group_name,
                service_name,
                shell_command,
            } => {
                config.set_service_shell_command(&domain_name, &group_name, &service_name, &shell_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set shell_command for service '{}.{}' to:\n  {}",
                    domain_name, service_name, shell_command
                );
            }
            SetSvcCommand::Platform {
                domain_name,
                group_name,
                service_name,
                platform,
            } => {
                config.set_service_platform(&domain_name, &group_name, &service_name, &platform)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set platform for service '{}.{}' to:\n  {}",
                    domain_name, service_name, platform
                );
            }
            SetSvcCommand::DefaultContainerImage {
                domain_name,
                group_name,
                service_name,
                default_container_image,
            } => {
                config.set_service_default_container_image(
                    &domain_name,
                    &group_name,
                    &service_name,
                    &default_container_image,
                )?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_container_image for service '{}.{}' to:\n  {}",
                    domain_name, service_name, default_container_image
                );
            }
        },
        SetCommand::Dom { cmd } => match cmd {
            SetDomCommand::DefaultEnvironment {
                domain_name,
                default_environment,
            } => {
                config.set_domain_default_environment(&domain_name, &default_environment)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_environment for domain '{}' to environment '{}'",
                    domain_name, default_environment
                );
            }
            SetDomCommand::ImageRepository {
                domain_name,
                image_repository,
            } => {
                config.set_domain_image_repository(&domain_name, &image_repository)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set image_repository for domain '{}' to:\n  {}",
                    domain_name, image_repository
                );
            }
            SetDomCommand::ServeCommand {
                domain_name,
                serve_command,
            } => {
                config.set_domain_serve_command(&domain_name, &serve_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set serve_command for domain '{}' to:\n  {}",
                    domain_name, serve_command
                );
            }
            SetDomCommand::ShellCommand {
                domain_name,
                shell_command,
            } => {
                config.set_domain_shell_command(&domain_name, &shell_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set shell_command for domain '{}' to:\n  {}",
                    domain_name, shell_command
                );
            }
            SetDomCommand::Platform {
                domain_name,
                platform,
            } => {
                config.set_domain_platform(&domain_name, &platform)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set platform for domain '{}' to:\n  {}",
                    domain_name, platform
                );
            }
            SetDomCommand::DefaultContainerImage {
                domain_name,
                default_container_image,
            } => {
                config.set_domain_default_container_image(&domain_name, &default_container_image)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_container_image for domain '{}' to:\n  {}",
                    domain_name, default_container_image
                );
            }
        },
        SetCommand::Grp { cmd } => match cmd {
            SetGrpCommand::DefaultEnvironment {
                domain_name,
                group_name,
                default_environment,
            } => {
                config.set_group_default_environment(&domain_name, &group_name, &default_environment)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_environment for group '{}' in domain '{}' to '{}'",
                    group_name, domain_name, default_environment
                );
            }
            SetGrpCommand::ImageRepository {
                domain_name,
                group_name,
                image_repository,
            } => {
                config.set_group_image_repository(&domain_name, &group_name, &image_repository)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set image_repository for group '{}' in domain '{}' to:\n  {}",
                    group_name, domain_name, image_repository
                );
            }
            SetGrpCommand::ServeCommand {
                domain_name,
                group_name,
                serve_command,
            } => {
                config.set_group_serve_command(&domain_name, &group_name, &serve_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set serve_command for group '{}' in domain '{}' to:\n  {}",
                    group_name, domain_name, serve_command
                );
            }
            SetGrpCommand::ShellCommand {
                domain_name,
                group_name,
                shell_command,
            } => {
                config.set_group_shell_command(&domain_name, &group_name, &shell_command)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set shell_command for group '{}' in domain '{}' to:\n  {}",
                    group_name, domain_name, shell_command
                );
            }
            SetGrpCommand::Platform {
                domain_name,
                group_name,
                platform,
            } => {
                config.set_group_platform(&domain_name, &group_name, &platform)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set platform for group '{}' in domain '{}' to:\n  {}",
                    group_name, domain_name, platform
                );
            }
            SetGrpCommand::DefaultContainerImage {
                domain_name,
                group_name,
                default_container_image,
            } => {
                config.set_group_default_container_image(&domain_name, &group_name, &default_container_image)?;
                config.save(&paths.config_path)?;
                println!(
                    "Set default_container_image for group '{}' in domain '{}' to:\n  {}",
                    group_name, domain_name, default_container_image
                );
            }
        },
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
        AddCommand::PreConfig { location, repo_location } => {
            config.add_pre_config(&location, repo_location.as_deref())?;
            config.save(&paths.config_path)?;
            println!("Added pre_config '{}'", location);
        }
        AddCommand::Domain { name, location } => {
            config.add_domain(&name, &location)?;
            config.save(&paths.config_path)?;
        }
        AddCommand::Dom { cmd } => match cmd {
            AddDomCommand::Portmap {
                domain_name,
                host_port,
                container_port,
            } => {
                config.add_domain_portmap(&domain_name, &host_port, &container_port)?;
                config.save(&paths.config_path)?;
            }
            AddDomCommand::Variable {
                domain_name,
                name,
                value,
            } => {
                config.add_domain_variable(&domain_name, &name, &value)?;
                config.save(&paths.config_path)?;
            }
            AddDomCommand::Volume {
                domain_name,
                container_dir,
                host_dir,
            } => {
                config.add_domain_volume(&domain_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
        },
        AddCommand::Grp { cmd } => match cmd {
            AddGrpCommand::Group {
                domain_name,
                group_name,
            } => {
                config.add_group(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            AddGrpCommand::Portmap {
                domain_name,
                group_name,
                host_port,
                container_port,
            } => {
                config.add_group_portmap(&domain_name, &group_name, &host_port, &container_port)?;
                config.save(&paths.config_path)?;
            }
            AddGrpCommand::Variable {
                domain_name,
                group_name,
                name,
                value,
            } => {
                config.add_group_variable(&domain_name, &group_name, &name, &value)?;
                config.save(&paths.config_path)?;
            }
            AddGrpCommand::Volume {
                domain_name,
                group_name,
                container_dir,
                host_dir,
            } => {
                config.add_group_volume(&domain_name, &group_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
        },
        AddCommand::Env { cmd } => match cmd {
            AddEnvCommand::Portmap {
                environment,
                host_port,
                container_port,
            } => {
                config.add_env_portmap(&environment, &host_port, &container_port)?;
                config.save(&paths.config_path)?;
            }
            AddEnvCommand::Variable {
                environment,
                name,
                value,
            } => {
                config.add_env_variable(&environment, &name, &value)?;
                config.save(&paths.config_path)?;
            }
            AddEnvCommand::Volume {
                environment,
                container_dir,
                host_dir,
            } => {
                config.add_volume(&environment, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
        },
        AddCommand::Svc { cmd } => match cmd {
            AddSvcCommand::Portmap {
                domain_name,
                group_name,
                service_name,
                host_port,
                container_port,
            } => {
                config.add_portmap(&domain_name, &group_name, &service_name, &host_port, &container_port)?;
                config.save(&paths.config_path)?;
            }
            AddSvcCommand::Variable {
                domain_name,
                group_name,
                service_name,
                name,
                value,
            } => {
                config.add_variable(&domain_name, &group_name, &service_name, &name, &value)?;
                config.save(&paths.config_path)?;
            }
            AddSvcCommand::Volume {
                domain_name,
                group_name,
                service_name,
                container_dir,
                host_dir,
            } => {
                config.add_service_volume(&domain_name, &group_name, &service_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
        },
    }

    Ok(())
}

fn cmd_rm(
    cmd: RmCommand,
    paths: &DarpPaths,
    config: &mut Config,
) -> anyhow::Result<()> {
    match cmd {
        RmCommand::PodmanMachine {} => {
            // Clear from config.json
            config.podman_machine = None;
            config.save(&paths.config_path)?;
        }
        RmCommand::PreConfig { location } => {
            config.rm_pre_config(&location)?;
            config.save(&paths.config_path)?;
            println!("Removed pre_config '{}'", location);
        }
        RmCommand::Domain { name } => {
            config.rm_domain(&name)?;
            config.save(&paths.config_path)?;
        }
        RmCommand::Dom { cmd } => match cmd {
            RmDomCommand::DefaultEnvironment { domain_name } => {
                config.rm_domain_default_environment(&domain_name)?;
                config.save(&paths.config_path)?;
                println!("Removed default_environment for domain '{}'", domain_name);
            }
            RmDomCommand::Portmap {
                domain_name,
                host_port,
            } => {
                config.rm_domain_portmap(&domain_name, &host_port)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::Variable {
                domain_name,
                name,
            } => {
                config.rm_domain_variable(&domain_name, &name)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::Volume {
                domain_name,
                container_dir,
                host_dir,
            } => {
                config.rm_domain_volume(&domain_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::ServeCommand { domain_name } => {
                config.rm_domain_serve_command(&domain_name)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::ShellCommand { domain_name } => {
                config.rm_domain_shell_command(&domain_name)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::ImageRepository { domain_name } => {
                config.rm_domain_image_repository(&domain_name)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::Platform { domain_name } => {
                config.rm_domain_platform(&domain_name)?;
                config.save(&paths.config_path)?;
            }
            RmDomCommand::DefaultContainerImage { domain_name } => {
                config.rm_domain_default_container_image(&domain_name)?;
                config.save(&paths.config_path)?;
            }
        },
        RmCommand::Grp { cmd } => match cmd {
            RmGrpCommand::Group {
                domain_name,
                group_name,
            } => {
                config.rm_group(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::DefaultEnvironment {
                domain_name,
                group_name,
            } => {
                config.rm_group_default_environment(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
                println!(
                    "Removed default_environment for group '{}' in domain '{}'",
                    group_name, domain_name
                );
            }
            RmGrpCommand::Portmap {
                domain_name,
                group_name,
                host_port,
            } => {
                config.rm_group_portmap(&domain_name, &group_name, &host_port)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::Variable {
                domain_name,
                group_name,
                name,
            } => {
                config.rm_group_variable(&domain_name, &group_name, &name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::Volume {
                domain_name,
                group_name,
                container_dir,
                host_dir,
            } => {
                config.rm_group_volume(&domain_name, &group_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::ServeCommand {
                domain_name,
                group_name,
            } => {
                config.rm_group_serve_command(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::ShellCommand {
                domain_name,
                group_name,
            } => {
                config.rm_group_shell_command(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::ImageRepository {
                domain_name,
                group_name,
            } => {
                config.rm_group_image_repository(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::Platform {
                domain_name,
                group_name,
            } => {
                config.rm_group_platform(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
            RmGrpCommand::DefaultContainerImage {
                domain_name,
                group_name,
            } => {
                config.rm_group_default_container_image(&domain_name, &group_name)?;
                config.save(&paths.config_path)?;
            }
        },
        RmCommand::Env { cmd } => match cmd {
            RmEnvCommand::Portmap {
                environment,
                host_port,
            } => {
                config.rm_env_portmap(&environment, &host_port)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::Variable {
                environment,
                name,
            } => {
                config.rm_env_variable(&environment, &name)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::Volume {
                environment,
                container_dir,
                host_dir,
            } => {
                config.rm_volume(&environment, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::ServeCommand { environment } => {
                config.rm_serve_command(&environment)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::ShellCommand { environment } => {
                config.rm_shell_command(&environment)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::ImageRepository { environment } => {
                config.rm_image_repository(&environment)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::Platform { environment } => {
                config.rm_platform(&environment)?;
                config.save(&paths.config_path)?;
            }
            RmEnvCommand::DefaultContainerImage { environment } => {
                config.rm_default_container_image(&environment)?;
                config.save(&paths.config_path)?;
            }
        },
        RmCommand::Svc { cmd } => match cmd {
            RmSvcCommand::Portmap {
                domain_name,
                group_name,
                service_name,
                host_port,
            } => {
                config.rm_portmap(&domain_name, &group_name, &service_name, &host_port)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::Variable {
                domain_name,
                group_name,
                service_name,
                name,
            } => {
                config.rm_variable(&domain_name, &group_name, &service_name, &name)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::Volume {
                domain_name,
                group_name,
                service_name,
                container_dir,
                host_dir,
            } => {
                config.rm_service_volume(&domain_name, &group_name, &service_name, &container_dir, &host_dir)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::ServeCommand {
                domain_name,
                group_name,
                service_name,
            } => {
                config.rm_service_serve_command(&domain_name, &group_name, &service_name)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::ShellCommand {
                domain_name,
                group_name,
                service_name,
            } => {
                config.rm_service_shell_command(&domain_name, &group_name, &service_name)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::ImageRepository {
                domain_name,
                group_name,
                service_name,
            } => {
                config.rm_service_image_repository(&domain_name, &group_name, &service_name)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::Platform {
                domain_name,
                group_name,
                service_name,
            } => {
                config.rm_service_platform(&domain_name, &group_name, &service_name)?;
                config.save(&paths.config_path)?;
            }
            RmSvcCommand::DefaultContainerImage {
                domain_name,
                group_name,
                service_name,
            } => {
                config.rm_service_default_container_image(&domain_name, &group_name, &service_name)?;
                config.save(&paths.config_path)?;
            }
        },
    }

    Ok(())
}

fn cmd_show(
    environment_cli: Option<String>,
    config: &Config,
) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let current_directory_name = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (domain_name, domain, group_name, group_opt) = config
        .find_context_by_cwd(&current_dir)
        .unwrap_or_else(|| {
            eprintln!(
                "Current directory does not exist in any darp domain configuration."
            );
            std::process::exit(1);
        });
    let domain_name = domain_name.to_string();

    let service_opt = group_opt
        .and_then(|g| g.services.as_ref())
        .and_then(|s| s.get(&current_directory_name));

    let effective_env_name: Option<String> = environment_cli
        .or_else(|| group_opt.and_then(|g| g.default_environment.clone()))
        .or_else(|| domain.default_environment.clone());

    let env = if let Some(ref env_name) = effective_env_name {
        let env_opt = config
            .environments
            .as_ref()
            .and_then(|e| e.get(env_name).cloned());
        if env_opt.is_none() {
            eprintln!("Environment '{}' does not exist.", env_name);
            std::process::exit(1);
        }
        env_opt
    } else {
        None
    };

    let resolved = ResolvedSettings {
        domain_name,
        group_name,
        service_name: current_directory_name,
        environment_name: effective_env_name,
        serve_command: service_opt
            .and_then(|s| s.serve_command.clone())
            .or_else(|| group_opt.and_then(|g| g.serve_command.clone()))
            .or_else(|| domain.serve_command.clone())
            .or_else(|| env.as_ref().and_then(|e| e.serve_command.clone())),
        shell_command: service_opt
            .and_then(|s| s.shell_command.clone())
            .or_else(|| group_opt.and_then(|g| g.shell_command.clone()))
            .or_else(|| domain.shell_command.clone())
            .or_else(|| env.as_ref().and_then(|e| e.shell_command.clone())),
        image_repository: service_opt
            .and_then(|s| s.image_repository.clone())
            .or_else(|| group_opt.and_then(|g| g.image_repository.clone()))
            .or_else(|| domain.image_repository.clone())
            .or_else(|| env.as_ref().and_then(|e| e.image_repository.clone())),
        platform: service_opt
            .and_then(|s| s.platform.clone())
            .or_else(|| group_opt.and_then(|g| g.platform.clone()))
            .or_else(|| domain.platform.clone())
            .or_else(|| env.as_ref().and_then(|e| e.platform.clone())),
        default_container_image: service_opt
            .and_then(|s| s.default_container_image.clone())
            .or_else(|| group_opt.and_then(|g| g.default_container_image.clone()))
            .or_else(|| domain.default_container_image.clone())
            .or_else(|| env.as_ref().and_then(|e| e.default_container_image.clone())),
        host_portmappings: service_opt
            .and_then(|s| s.host_portmappings.clone())
            .or_else(|| group_opt.and_then(|g| g.host_portmappings.clone()))
            .or_else(|| domain.host_portmappings.clone())
            .or_else(|| env.as_ref().and_then(|e| e.host_portmappings.clone())),
        variables: service_opt
            .and_then(|s| s.variables.clone())
            .or_else(|| group_opt.and_then(|g| g.variables.clone()))
            .or_else(|| domain.variables.clone())
            .or_else(|| env.as_ref().and_then(|e| e.variables.clone())),
        volumes: service_opt
            .and_then(|s| s.volumes.clone())
            .or_else(|| group_opt.and_then(|g| g.volumes.clone()))
            .or_else(|| domain.volumes.clone())
            .or_else(|| env.as_ref().and_then(|e| e.volumes.clone())),
    };

    println!("{}", serde_json::to_string_pretty(&resolved)?);
    Ok(())
}

fn cmd_pull(config: &Config) -> anyhow::Result<()> {
    let entries = match &config.pre_config {
        Some(entries) if !entries.is_empty() => entries,
        _ => {
            println!("No pre_config entries configured.");
            return Ok(());
        }
    };

    for entry in entries {
        let repo_location = match &entry.repo_location {
            Some(loc) => loc,
            None => {
                println!("Skipping '{}' (no repo_location)", entry.location);
                continue;
            }
        };

        let resolved = config::resolve_location(repo_location)?;
        println!("Pulling '{}' ...", resolved.display());

        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&resolved)
            .arg("pull")
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if !stdout.is_empty() {
                    print!("  {}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("  {}", stderr);
                }
                if !out.status.success() {
                    eprintln!("  git pull failed with exit code {}", out.status);
                }
            }
            Err(e) => {
                eprintln!("  Failed to run git: {}", e);
            }
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
