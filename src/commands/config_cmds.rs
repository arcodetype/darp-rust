use colored::*;

use crate::cli::*;
use crate::config::{self, Config, DarpPaths, ResolvedSettings};
use crate::engine::EngineKind;

fn config_mutate(
    config: &mut Config,
    path: &std::path::Path,
    f: impl FnOnce(&mut Config) -> anyhow::Result<()>,
    msg: Option<String>,
) -> anyhow::Result<()> {
    f(config)?;
    config.save(path)?;
    if let Some(msg) = msg {
        println!("{}", msg);
    }
    Ok(())
}

pub fn cmd_set(
    cmd: SetCommand,
    paths: &DarpPaths,
    config: &mut Config,
    _engine_kind: &EngineKind,
) -> anyhow::Result<()> {
    let p = &paths.config_path;
    match cmd {
        SetCommand::PodmanMachine { new_podman_machine } => {
            config_mutate(
                config,
                p,
                |c| {
                    c.podman_machine = Some(new_podman_machine.clone());
                    Ok(())
                },
                Some(format!(
                    "PODMAN_MACHINE set to '{}' in config ({}).",
                    new_podman_machine,
                    p.display()
                )),
            )?;
        }
        SetCommand::Engine { engine } => {
            let engine_lc = engine.to_lowercase();
            if engine_lc != "podman" && engine_lc != "docker" {
                eprintln!("engine must be 'podman' or 'docker'");
                std::process::exit(1);
            }
            config_mutate(
                config,
                p,
                |c| {
                    c.engine = Some(engine_lc);
                    Ok(())
                },
                Some("Engine set. New Darp invocations will use this container engine.".into()),
            )?;
        }
        SetCommand::Env { cmd } => match cmd {
            SetEnvCommand::ImageRepository {
                environment,
                image_repository,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_image_repository(&environment, &image_repository),
                    Some(format!(
                        "Set image_repository for environment '{}' to:\n  {}",
                        environment, image_repository
                    )),
                )?;
            }
            SetEnvCommand::ServeCommand {
                environment,
                serve_command,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_serve_command(&environment, &serve_command),
                    Some(format!(
                        "Set serve_command for environment '{}' to:\n  {}",
                        environment, serve_command
                    )),
                )?;
            }
            SetEnvCommand::ShellCommand {
                environment,
                shell_command,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_shell_command(&environment, &shell_command),
                    Some(format!(
                        "Set shell_command for environment '{}' to:\n  {}",
                        environment, shell_command
                    )),
                )?;
            }
            SetEnvCommand::Platform {
                environment,
                platform,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_platform(&environment, &platform),
                    Some(format!(
                        "Set platform for environment '{}' to:\n  {}",
                        environment, platform
                    )),
                )?;
            }
            SetEnvCommand::DefaultContainerImage {
                environment,
                default_container_image,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_default_container_image(&environment, &default_container_image),
                    Some(format!(
                        "Set default_container_image for environment '{}' to:\n  {}",
                        environment, default_container_image
                    )),
                )?;
            }
            SetEnvCommand::ConnectionType {
                environment,
                connection_type,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.set_environment_connection_type(&environment, &connection_type),
                    Some(format!(
                        "Set connection_type for environment '{}' to:\n  {}",
                        environment, connection_type
                    )),
                )?;
            }
        },
        SetCommand::Svc { cmd } => match cmd {
            SetSvcCommand::DefaultEnvironment {
                domain_name,
                group_name,
                service_name,
                default_environment,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_default_environment(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &default_environment,
                        )
                    },
                    Some(format!(
                        "Set default_environment for service '{}.{}' to '{}'",
                        domain_name, service_name, default_environment
                    )),
                )?;
            }
            SetSvcCommand::ImageRepository {
                domain_name,
                group_name,
                service_name,
                image_repository,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_image_repository(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &image_repository,
                        )
                    },
                    Some(format!(
                        "Set image_repository for service '{}.{}' to:\n  {}",
                        domain_name, service_name, image_repository
                    )),
                )?;
            }
            SetSvcCommand::ServeCommand {
                domain_name,
                group_name,
                service_name,
                serve_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_serve_command(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &serve_command,
                        )
                    },
                    Some(format!(
                        "Set serve_command for service '{}.{}' to:\n  {}",
                        domain_name, service_name, serve_command
                    )),
                )?;
            }
            SetSvcCommand::ShellCommand {
                domain_name,
                group_name,
                service_name,
                shell_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_shell_command(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &shell_command,
                        )
                    },
                    Some(format!(
                        "Set shell_command for service '{}.{}' to:\n  {}",
                        domain_name, service_name, shell_command
                    )),
                )?;
            }
            SetSvcCommand::Platform {
                domain_name,
                group_name,
                service_name,
                platform,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_platform(&domain_name, &group_name, &service_name, &platform)
                    },
                    Some(format!(
                        "Set platform for service '{}.{}' to:\n  {}",
                        domain_name, service_name, platform
                    )),
                )?;
            }
            SetSvcCommand::DefaultContainerImage {
                domain_name,
                group_name,
                service_name,
                default_container_image,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_default_container_image(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &default_container_image,
                        )
                    },
                    Some(format!(
                        "Set default_container_image for service '{}.{}' to:\n  {}",
                        domain_name, service_name, default_container_image
                    )),
                )?;
            }
            SetSvcCommand::ConnectionType {
                domain_name,
                group_name,
                service_name,
                connection_type,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_service_connection_type(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &connection_type,
                        )
                    },
                    Some(format!(
                        "Set connection_type for service '{}.{}' to:\n  {}",
                        domain_name, service_name, connection_type
                    )),
                )?;
            }
        },
        SetCommand::Dom { cmd } => match cmd {
            SetDomCommand::DefaultEnvironment {
                domain_name,
                default_environment,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_default_environment(&domain_name, &default_environment)
                    },
                    Some(format!(
                        "Set default_environment for domain '{}' to environment '{}'",
                        domain_name, default_environment
                    )),
                )?;
            }
            SetDomCommand::ImageRepository {
                domain_name,
                image_repository,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_image_repository(&domain_name, &image_repository)
                    },
                    Some(format!(
                        "Set image_repository for domain '{}' to:\n  {}",
                        domain_name, image_repository
                    )),
                )?;
            }
            SetDomCommand::ServeCommand {
                domain_name,
                serve_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_serve_command(&domain_name, &serve_command)
                    },
                    Some(format!(
                        "Set serve_command for domain '{}' to:\n  {}",
                        domain_name, serve_command
                    )),
                )?;
            }
            SetDomCommand::ShellCommand {
                domain_name,
                shell_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_shell_command(&domain_name, &shell_command)
                    },
                    Some(format!(
                        "Set shell_command for domain '{}' to:\n  {}",
                        domain_name, shell_command
                    )),
                )?;
            }
            SetDomCommand::Platform {
                domain_name,
                platform,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_platform(&domain_name, &platform)
                    },
                    Some(format!(
                        "Set platform for domain '{}' to:\n  {}",
                        domain_name, platform
                    )),
                )?;
            }
            SetDomCommand::DefaultContainerImage {
                domain_name,
                default_container_image,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_default_container_image(&domain_name, &default_container_image)
                    },
                    Some(format!(
                        "Set default_container_image for domain '{}' to:\n  {}",
                        domain_name, default_container_image
                    )),
                )?;
            }
            SetDomCommand::ConnectionType {
                domain_name,
                connection_type,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_domain_connection_type(&domain_name, &connection_type)
                    },
                    Some(format!(
                        "Set connection_type for domain '{}' to:\n  {}",
                        domain_name, connection_type
                    )),
                )?;
            }
        },
        SetCommand::Grp { cmd } => match cmd {
            SetGrpCommand::DefaultEnvironment {
                domain_name,
                group_name,
                default_environment,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_default_environment(
                            &domain_name,
                            &group_name,
                            &default_environment,
                        )
                    },
                    Some(format!(
                        "Set default_environment for group '{}' in domain '{}' to '{}'",
                        group_name, domain_name, default_environment
                    )),
                )?;
            }
            SetGrpCommand::ImageRepository {
                domain_name,
                group_name,
                image_repository,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_image_repository(&domain_name, &group_name, &image_repository)
                    },
                    Some(format!(
                        "Set image_repository for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, image_repository
                    )),
                )?;
            }
            SetGrpCommand::ServeCommand {
                domain_name,
                group_name,
                serve_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_serve_command(&domain_name, &group_name, &serve_command)
                    },
                    Some(format!(
                        "Set serve_command for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, serve_command
                    )),
                )?;
            }
            SetGrpCommand::ShellCommand {
                domain_name,
                group_name,
                shell_command,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_shell_command(&domain_name, &group_name, &shell_command)
                    },
                    Some(format!(
                        "Set shell_command for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, shell_command
                    )),
                )?;
            }
            SetGrpCommand::Platform {
                domain_name,
                group_name,
                platform,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_platform(&domain_name, &group_name, &platform)
                    },
                    Some(format!(
                        "Set platform for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, platform
                    )),
                )?;
            }
            SetGrpCommand::DefaultContainerImage {
                domain_name,
                group_name,
                default_container_image,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_default_container_image(
                            &domain_name,
                            &group_name,
                            &default_container_image,
                        )
                    },
                    Some(format!(
                        "Set default_container_image for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, default_container_image
                    )),
                )?;
            }
            SetGrpCommand::ConnectionType {
                domain_name,
                group_name,
                connection_type,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.set_group_connection_type(&domain_name, &group_name, &connection_type)
                    },
                    Some(format!(
                        "Set connection_type for group '{}' in domain '{}' to:\n  {}",
                        group_name, domain_name, connection_type
                    )),
                )?;
            }
        },
        SetCommand::UrlsInHosts { value } => {
            let v = config.parse_bool(&value)?;
            config_mutate(
                config,
                p,
                |c| {
                    c.urls_in_hosts = Some(v);
                    Ok(())
                },
                Some(format!(
                    "urls_in_hosts has been {} (stored in {}). Next 'darp deploy' will sync /etc/hosts accordingly.",
                    if v { "enabled" } else { "disabled" },
                    p.display()
                )),
            )?;
        }
        SetCommand::Wsl { value } => {
            let v = config.parse_bool(&value)?;
            config_mutate(
                config,
                p,
                |c| {
                    c.wsl = Some(v);
                    Ok(())
                },
                Some(format!(
                    "WSL mode has been {} (stored in {}). When enabled alongside urls_in_hosts, 'darp deploy' will also sync /mnt/c/Windows/System32/drivers/etc/hosts.",
                    if v { "enabled" } else { "disabled" },
                    p.display()
                )),
            )?;
        }
    }

    Ok(())
}

pub fn cmd_add(cmd: AddCommand, paths: &DarpPaths, config: &mut Config) -> anyhow::Result<()> {
    let p = &paths.config_path;
    match cmd {
        AddCommand::PreConfig {
            location,
            repo_location,
        } => {
            config_mutate(
                config,
                p,
                |c| c.add_pre_config(&location, repo_location.as_deref()),
                Some(format!("Added pre_config '{}'", location)),
            )?;
        }
        AddCommand::Dom { cmd } => match cmd {
            AddDomCommand::Portmap {
                domain_name,
                host_port,
                container_port,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_domain_portmap(&domain_name, &host_port, &container_port)
                    },
                    None,
                )?;
            }
            AddDomCommand::Variable {
                domain_name,
                name,
                value,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_domain_variable(&domain_name, &name, &value)
                    },
                    None,
                )?;
            }
            AddDomCommand::Volume {
                domain_name,
                container_dir,
                host_dir,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_domain_volume(&domain_name, &container_dir, &host_dir)
                    },
                    None,
                )?;
            }
        },
        AddCommand::Grp { cmd } => match cmd {
            AddGrpCommand::Portmap {
                domain_name,
                group_name,
                host_port,
                container_port,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_group_portmap(&domain_name, &group_name, &host_port, &container_port)
                    },
                    None,
                )?;
            }
            AddGrpCommand::Variable {
                domain_name,
                group_name,
                name,
                value,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_group_variable(&domain_name, &group_name, &name, &value)
                    },
                    None,
                )?;
            }
            AddGrpCommand::Volume {
                domain_name,
                group_name,
                container_dir,
                host_dir,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_group_volume(&domain_name, &group_name, &container_dir, &host_dir)
                    },
                    None,
                )?;
            }
        },
        AddCommand::Env { cmd } => match cmd {
            AddEnvCommand::Portmap {
                environment,
                host_port,
                container_port,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.add_env_portmap(&environment, &host_port, &container_port),
                    None,
                )?;
            }
            AddEnvCommand::Variable {
                environment,
                name,
                value,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.add_env_variable(&environment, &name, &value),
                    None,
                )?;
            }
            AddEnvCommand::Volume {
                environment,
                container_dir,
                host_dir,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.add_volume(&environment, &container_dir, &host_dir),
                    None,
                )?;
            }
        },
        AddCommand::Svc { cmd } => match cmd {
            AddSvcCommand::Portmap {
                domain_name,
                group_name,
                service_name,
                host_port,
                container_port,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_portmap(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &host_port,
                            &container_port,
                        )
                    },
                    None,
                )?;
            }
            AddSvcCommand::Variable {
                domain_name,
                group_name,
                service_name,
                name,
                value,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_variable(&domain_name, &group_name, &service_name, &name, &value)
                    },
                    None,
                )?;
            }
            AddSvcCommand::Volume {
                domain_name,
                group_name,
                service_name,
                container_dir,
                host_dir,
                location,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.ensure_domain_exists(&domain_name, location.as_deref())?;
                        c.add_service_volume(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &container_dir,
                            &host_dir,
                        )
                    },
                    None,
                )?;
            }
        },
    }

    Ok(())
}

pub fn cmd_rm(cmd: RmCommand, paths: &DarpPaths, config: &mut Config) -> anyhow::Result<()> {
    let p = &paths.config_path;
    match cmd {
        RmCommand::PodmanMachine {} => {
            config_mutate(
                config,
                p,
                |c| {
                    c.podman_machine = None;
                    Ok(())
                },
                None,
            )?;
        }
        RmCommand::PreConfig { location } => {
            config_mutate(
                config,
                p,
                |c| c.rm_pre_config(&location),
                Some(format!("Removed pre_config '{}'", location)),
            )?;
        }
        RmCommand::Domain { name } => {
            config_mutate(config, p, |c| c.rm_domain(&name), None)?;
        }
        RmCommand::Group {
            domain_name,
            group_name,
        } => {
            config_mutate(config, p, |c| c.rm_group(&domain_name, &group_name), None)?;
        }
        RmCommand::Service {
            domain_name,
            group_name,
            service_name,
        } => {
            config_mutate(
                config,
                p,
                |c| c.rm_service(&domain_name, &group_name, &service_name),
                None,
            )?;
        }
        RmCommand::Dom { cmd } => match cmd {
            RmDomCommand::DefaultEnvironment { domain_name } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_default_environment(&domain_name),
                    Some(format!(
                        "Removed default_environment for domain '{}'",
                        domain_name
                    )),
                )?;
            }
            RmDomCommand::Portmap {
                domain_name,
                host_port,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_portmap(&domain_name, &host_port),
                    None,
                )?;
            }
            RmDomCommand::Variable { domain_name, name } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_variable(&domain_name, &name),
                    None,
                )?;
            }
            RmDomCommand::Volume {
                domain_name,
                container_dir,
                host_dir,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_volume(&domain_name, &container_dir, &host_dir),
                    None,
                )?;
            }
            RmDomCommand::ServeCommand { domain_name } => {
                config_mutate(config, p, |c| c.rm_domain_serve_command(&domain_name), None)?;
            }
            RmDomCommand::ShellCommand { domain_name } => {
                config_mutate(config, p, |c| c.rm_domain_shell_command(&domain_name), None)?;
            }
            RmDomCommand::ImageRepository { domain_name } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_image_repository(&domain_name),
                    None,
                )?;
            }
            RmDomCommand::Platform { domain_name } => {
                config_mutate(config, p, |c| c.rm_domain_platform(&domain_name), None)?;
            }
            RmDomCommand::DefaultContainerImage { domain_name } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_default_container_image(&domain_name),
                    None,
                )?;
            }
            RmDomCommand::ConnectionType { domain_name } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_domain_connection_type(&domain_name),
                    None,
                )?;
            }
        },
        RmCommand::Grp { cmd } => match cmd {
            RmGrpCommand::DefaultEnvironment {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_default_environment(&domain_name, &group_name),
                    Some(format!(
                        "Removed default_environment for group '{}' in domain '{}'",
                        group_name, domain_name
                    )),
                )?;
            }
            RmGrpCommand::Portmap {
                domain_name,
                group_name,
                host_port,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_portmap(&domain_name, &group_name, &host_port),
                    None,
                )?;
            }
            RmGrpCommand::Variable {
                domain_name,
                group_name,
                name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_variable(&domain_name, &group_name, &name),
                    None,
                )?;
            }
            RmGrpCommand::Volume {
                domain_name,
                group_name,
                container_dir,
                host_dir,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_volume(&domain_name, &group_name, &container_dir, &host_dir),
                    None,
                )?;
            }
            RmGrpCommand::ServeCommand {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_serve_command(&domain_name, &group_name),
                    None,
                )?;
            }
            RmGrpCommand::ShellCommand {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_shell_command(&domain_name, &group_name),
                    None,
                )?;
            }
            RmGrpCommand::ImageRepository {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_image_repository(&domain_name, &group_name),
                    None,
                )?;
            }
            RmGrpCommand::Platform {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_platform(&domain_name, &group_name),
                    None,
                )?;
            }
            RmGrpCommand::DefaultContainerImage {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_default_container_image(&domain_name, &group_name),
                    None,
                )?;
            }
            RmGrpCommand::ConnectionType {
                domain_name,
                group_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_group_connection_type(&domain_name, &group_name),
                    None,
                )?;
            }
        },
        RmCommand::Env { cmd } => match cmd {
            RmEnvCommand::Portmap {
                environment,
                host_port,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_env_portmap(&environment, &host_port),
                    None,
                )?;
            }
            RmEnvCommand::Variable { environment, name } => {
                config_mutate(config, p, |c| c.rm_env_variable(&environment, &name), None)?;
            }
            RmEnvCommand::Volume {
                environment,
                container_dir,
                host_dir,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_volume(&environment, &container_dir, &host_dir),
                    None,
                )?;
            }
            RmEnvCommand::ServeCommand { environment } => {
                config_mutate(config, p, |c| c.rm_serve_command(&environment), None)?;
            }
            RmEnvCommand::ShellCommand { environment } => {
                config_mutate(config, p, |c| c.rm_shell_command(&environment), None)?;
            }
            RmEnvCommand::ImageRepository { environment } => {
                config_mutate(config, p, |c| c.rm_image_repository(&environment), None)?;
            }
            RmEnvCommand::Platform { environment } => {
                config_mutate(config, p, |c| c.rm_platform(&environment), None)?;
            }
            RmEnvCommand::DefaultContainerImage { environment } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_default_container_image(&environment),
                    None,
                )?;
            }
            RmEnvCommand::ConnectionType { environment } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_environment_connection_type(&environment),
                    None,
                )?;
            }
        },
        RmCommand::Svc { cmd } => match cmd {
            RmSvcCommand::DefaultEnvironment {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_default_environment(&domain_name, &group_name, &service_name),
                    Some(format!(
                        "Removed default_environment for service '{}.{}'",
                        domain_name, service_name
                    )),
                )?;
            }
            RmSvcCommand::Portmap {
                domain_name,
                group_name,
                service_name,
                host_port,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_portmap(&domain_name, &group_name, &service_name, &host_port),
                    None,
                )?;
            }
            RmSvcCommand::Variable {
                domain_name,
                group_name,
                service_name,
                name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_variable(&domain_name, &group_name, &service_name, &name),
                    None,
                )?;
            }
            RmSvcCommand::Volume {
                domain_name,
                group_name,
                service_name,
                container_dir,
                host_dir,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.rm_service_volume(
                            &domain_name,
                            &group_name,
                            &service_name,
                            &container_dir,
                            &host_dir,
                        )
                    },
                    None,
                )?;
            }
            RmSvcCommand::ServeCommand {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_serve_command(&domain_name, &group_name, &service_name),
                    None,
                )?;
            }
            RmSvcCommand::ShellCommand {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_shell_command(&domain_name, &group_name, &service_name),
                    None,
                )?;
            }
            RmSvcCommand::ImageRepository {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_image_repository(&domain_name, &group_name, &service_name),
                    None,
                )?;
            }
            RmSvcCommand::Platform {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_platform(&domain_name, &group_name, &service_name),
                    None,
                )?;
            }
            RmSvcCommand::DefaultContainerImage {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| {
                        c.rm_service_default_container_image(
                            &domain_name,
                            &group_name,
                            &service_name,
                        )
                    },
                    None,
                )?;
            }
            RmSvcCommand::ConnectionType {
                domain_name,
                group_name,
                service_name,
            } => {
                config_mutate(
                    config,
                    p,
                    |c| c.rm_service_connection_type(&domain_name, &group_name, &service_name),
                    None,
                )?;
            }
        },
    }

    Ok(())
}

pub fn cmd_show(environment_cli: Option<String>, config: &Config) -> anyhow::Result<()> {
    let ctx = config
        .service_context_from_cwd(environment_cli)
        .unwrap_or_else(|| {
            eprintln!("Current directory does not exist in any darp domain configuration.");
            std::process::exit(1);
        });

    if let Some(ref env_name) = ctx.environment_name {
        if ctx.environment.is_none() {
            eprintln!("Environment '{}' does not exist.", env_name);
            std::process::exit(1);
        }
    }

    let resolved = ResolvedSettings::resolve(
        ctx.domain_name.clone(),
        ctx.group_name.clone(),
        ctx.current_directory_name,
        ctx.environment_name,
        ctx.service,
        ctx.group,
        ctx.domain,
        ctx.environment,
    );

    println!("{}", serde_json::to_string_pretty(&resolved)?);
    Ok(())
}

pub fn cmd_pull(config: &Config) -> anyhow::Result<()> {
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

pub fn cmd_urls(paths: &DarpPaths, _config: &Config) -> anyhow::Result<()> {
    let portmap: serde_json::Value = config::read_json(&paths.portmap_path)?;
    println!();
    if let Some(obj) = portmap.as_object() {
        for (domain_name, domain) in obj.iter() {
            println!("{}", domain_name.green());
            if let Some(groups) = domain.as_object() {
                let mut group_entries: Vec<_> = groups.iter().collect();
                group_entries.sort_by(|(a, _), (b, _)| match (a.as_str(), b.as_str()) {
                    (".", _) => std::cmp::Ordering::Less,
                    (_, ".") => std::cmp::Ordering::Greater,
                    _ => a.cmp(b),
                });

                for (group_name, group) in group_entries {
                    if let Some(services) = group.as_object() {
                        let indent = if group_name == "." {
                            "  "
                        } else {
                            println!("  {}", group_name.cyan());
                            "    "
                        };

                        let mut entries: Vec<_> = services.iter().collect();
                        entries.sort_by_key(|(k, _)| *k);
                        for (service_name, entry) in entries {
                            // Portmap entries are either a bare number (legacy) or
                            // an object {"port": N, "type": "..."}.
                            let port = entry
                                .get("port")
                                .and_then(|p| p.as_u64())
                                .or_else(|| entry.as_u64())
                                .unwrap_or(0);
                            let conn_type =
                                entry.get("type").and_then(|t| t.as_str()).unwrap_or("http");

                            match conn_type {
                                "tcp" => {
                                    println!(
                                        "{}tcp://{}.{}.test:{}",
                                        indent,
                                        service_name.blue(),
                                        domain_name.green(),
                                        port
                                    );
                                }
                                "websocket" => {
                                    println!(
                                        "{}ws://{}.{}.test ({})",
                                        indent,
                                        service_name.blue(),
                                        domain_name.green(),
                                        port
                                    );
                                }
                                _ => {
                                    println!(
                                        "{}http://{}.{}.test ({})",
                                        indent,
                                        service_name.blue(),
                                        domain_name.green(),
                                        port
                                    );
                                }
                            }
                        }
                    }
                }
            }
            println!();
        }
    }
    Ok(())
}
