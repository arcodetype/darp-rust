use clap::{CommandFactory, Parser};

use darp::cli::*;
use darp::commands::*;
use darp::config::{Config, DarpPaths};
use darp::engine::{self, Engine, EngineKind};
use darp::os::OsIntegration;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let paths = DarpPaths::from_env()?;

    if let Some(cmd) = cli.command {
        match cmd {
            Command::Config { cmd } => match cmd {
                ConfigCommand::Show { environment } => {
                    let config = Config::load_merged(&paths.config_path)?;
                    cmd_show(environment, &config)?;
                }
                ConfigCommand::Pull => {
                    let config = Config::load(&paths.config_path)?;
                    cmd_pull(&config)?;
                }
                _ => {
                    let mut config = Config::load(&paths.config_path)?;
                    let engine_kind = EngineKind::from_config(&config);
                    match cmd {
                        ConfigCommand::Set { cmd } => {
                            cmd_set(cmd, &paths, &mut config, &engine_kind)?
                        }
                        ConfigCommand::Add { cmd } => cmd_add(cmd, &paths, &mut config)?,
                        ConfigCommand::Rm { cmd } => cmd_rm(cmd, &paths, &mut config)?,
                        ConfigCommand::Show { .. } | ConfigCommand::Pull => unreachable!(),
                    }
                }
            },
            _ => {
                let config = Config::load_merged(&paths.config_path)?;
                let engine_kind = EngineKind::from_config(&config);
                let engine = Engine::new(engine_kind.clone(), &config)?;
                let os = OsIntegration::new(&paths, &config, &engine_kind);
                match cmd {
                    Command::Install => cmd_install(&paths, &config, &os, &engine)?,
                    Command::Uninstall => cmd_uninstall(&paths, &mut config.clone(), &os, &engine)?,
                    Command::Deploy => cmd_deploy(&paths, &config, &os, &engine)?,
                    Command::Shell {
                        environment,
                        dry_run,
                        container_image,
                    } => cmd_shell(
                        environment,
                        dry_run,
                        container_image,
                        &paths,
                        &config,
                        &engine,
                    )?,
                    Command::Serve {
                        environment,
                        dry_run,
                        container_image,
                    } => cmd_serve(
                        environment,
                        dry_run,
                        container_image,
                        &paths,
                        &config,
                        &engine,
                    )?,
                    Command::Urls => cmd_urls(&paths, &config)?,
                    Command::Doctor => cmd_doctor(&paths, &config, &engine)?,
                    Command::CheckImage { image, environment } => {
                        cmd_check_image(image, environment, &paths, &config, &engine)?
                    }
                    Command::Config { .. } => unreachable!(),
                }
            }
        }
    } else {
        let mut cmd = Cli::command();
        cmd.print_help()?;
        println!();
    }

    Ok(())
}

fn cmd_install(
    paths: &DarpPaths,
    _config: &Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    println!("Running installation");

    os.init_resolver()?;
    os.ensure_dnsmasq_dir()?;
    os.copy_nginx_conf()?;
    os.write_test_conf()?;

    engine.configure_unprivileged_ports_if_needed()?;

    install_shell_completions()?;

    // Probe the container engine for its host-gateway IP and cache it for deploy.
    // Skipped if the engine isn't configured or isn't currently running — deploy
    // will re-probe on demand.
    if engine.require_ready().is_ok() {
        match engine.probe_host_gateway_ip() {
            Ok(ip) => {
                engine::write_container_host_ip(&paths.container_host_ip_path, &engine.kind, &ip)?;
                println!("cached container host gateway: {}", ip);
            }
            Err(e) => {
                eprintln!(
                    "warning: could not probe container host gateway ({}); deploy will retry",
                    e
                );
            }
        }
    }

    Ok(())
}

fn cmd_uninstall(
    _paths: &DarpPaths,
    _config: &mut Config,
    os: &OsIntegration,
    engine: &Engine,
) -> anyhow::Result<()> {
    println!("Running uninstallation");

    engine.stop_running_darps()?;
    engine.stop_named_container("darp-reverse-proxy")?;
    engine.stop_named_container("darp-masq")?;

    os.uninstall()?;

    uninstall_shell_completions()?;

    println!("Uninstall complete. Darp config.json has been left on disk.");
    Ok(())
}
