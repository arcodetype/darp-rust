use clap::{Parser, Subcommand};

/// Your directories auto-reverse proxied.
#[derive(Parser, Debug)]
#[command(
    name = "darp",
    about = "Your directories auto-reverse proxied.",
    version,
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
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
    /// Check system health and configuration
    Doctor,
    /// Validate a container image works with darp
    CheckImage {
        /// Container image to check (if omitted, resolves from current directory context)
        image: Option<String>,
        /// Environment name (used to resolve serve_command and shell_command)
        #[arg(short, long)]
        environment: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommand {
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
pub enum SetCommand {
    /// Set container engine (podman|docker)
    Engine { engine: String },
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
    UrlsInHosts { value: String },
    /// Enable/disable WSL mode (syncs Windows hosts file and adds doctor checks)
    Wsl { value: String },
}

#[derive(Subcommand, Debug)]
pub enum SetDomCommand {
    /// Set default_environment on a domain
    DefaultEnvironment {
        /// Logical domain name (e.g. 'my-domain')
        domain_name: String,
        /// Environment name to use by default for this domain
        default_environment: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set image_repository on a domain
    ImageRepository {
        domain_name: String,
        image_repository: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set serve_command on a domain
    ServeCommand {
        domain_name: String,
        serve_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set shell_command on a domain (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        shell_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set platform architecture (e.g., linux/amd64) on a domain
    Platform {
        domain_name: String,
        platform: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set default_container_image on a domain (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        default_container_image: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SetGrpCommand {
    /// Set default_environment on a group
    DefaultEnvironment {
        domain_name: String,
        group_name: String,
        default_environment: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set image_repository on a group
    ImageRepository {
        domain_name: String,
        group_name: String,
        image_repository: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set serve_command on a group
    ServeCommand {
        domain_name: String,
        group_name: String,
        serve_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set shell_command on a group (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        group_name: String,
        shell_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set platform architecture (e.g., linux/amd64) on a group
    Platform {
        domain_name: String,
        group_name: String,
        platform: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set default_container_image on a group (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
        default_container_image: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum SetEnvCommand {
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
pub enum SetSvcCommand {
    /// Set image_repository on a service
    ImageRepository {
        domain_name: String,
        group_name: String,
        service_name: String,
        image_repository: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set serve_command on a service
    ServeCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
        serve_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set shell_command on a service (used by `darp shell`)
    ShellCommand {
        domain_name: String,
        group_name: String,
        service_name: String,
        shell_command: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set platform architecture (e.g., linux/amd64) on a service
    Platform {
        domain_name: String,
        group_name: String,
        service_name: String,
        platform: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Set default_container_image on a service (used when no image is passed on the CLI)
    DefaultContainerImage {
        domain_name: String,
        group_name: String,
        service_name: String,
        default_container_image: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AddCommand {
    /// Add a pre_config entry (parent config for chaining)
    PreConfig {
        /// Path to the config file (supports {home} token)
        location: String,
        /// Path to git repo for `darp config pull` (supports {home} token)
        #[arg(short, long)]
        repo_location: Option<String>,
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
pub enum AddDomCommand {
    /// Add port mapping to a domain
    Portmap {
        domain_name: String,
        host_port: String,
        container_port: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add variable to a domain
    Variable {
        domain_name: String,
        name: String,
        value: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add volume to a domain
    Volume {
        domain_name: String,
        container_dir: String,
        host_dir: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AddGrpCommand {
    /// Add port mapping to a group
    Portmap {
        domain_name: String,
        group_name: String,
        host_port: String,
        container_port: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add variable to a group
    Variable {
        domain_name: String,
        group_name: String,
        name: String,
        value: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add volume to a group
    Volume {
        domain_name: String,
        group_name: String,
        container_dir: String,
        host_dir: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum AddEnvCommand {
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
pub enum AddSvcCommand {
    /// Add port mapping to a service
    Portmap {
        domain_name: String,
        group_name: String,
        service_name: String,
        host_port: String,
        container_port: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add variable to a service
    Variable {
        domain_name: String,
        group_name: String,
        service_name: String,
        name: String,
        value: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
    /// Add volume to a service
    Volume {
        domain_name: String,
        group_name: String,
        service_name: String,
        container_dir: String,
        host_dir: String,
        /// Create the domain at this path if it doesn't exist
        #[arg(short = 'l', long)]
        location: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum RmCommand {
    /// Remove a domain
    Domain { name: String },
    /// Remove a group from a domain
    Group {
        domain_name: String,
        group_name: String,
    },
    /// Remove a service from a group
    Service {
        domain_name: String,
        group_name: String,
        service_name: String,
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
pub enum RmDomCommand {
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
    Variable { domain_name: String, name: String },
    /// Remove volume from a domain
    Volume {
        domain_name: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from a domain
    ServeCommand { domain_name: String },
    /// Remove shell_command from a domain
    ShellCommand { domain_name: String },
    /// Remove image_repository from a domain
    ImageRepository { domain_name: String },
    /// Remove platform architecture from a domain
    Platform { domain_name: String },
    /// Remove default_container_image from a domain
    DefaultContainerImage { domain_name: String },
}

#[derive(Subcommand, Debug)]
pub enum RmGrpCommand {
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
pub enum RmEnvCommand {
    /// Remove port mapping from an environment
    Portmap {
        environment: String,
        host_port: String,
    },
    /// Remove variable from an environment
    Variable { environment: String, name: String },
    /// Remove volume from an environment
    Volume {
        environment: String,
        container_dir: String,
        host_dir: String,
    },
    /// Remove serve_command from an environment
    ServeCommand { environment: String },
    /// Remove shell_command from an environment
    ShellCommand { environment: String },
    /// Remove image_repository from an environment
    ImageRepository { environment: String },
    /// Remove platform architecture from an environment
    Platform { environment: String },
    /// Remove default_container_image from an environment
    DefaultContainerImage { environment: String },
}

#[derive(Subcommand, Debug)]
pub enum RmSvcCommand {
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
