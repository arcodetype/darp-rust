use colored::*;

use crate::config::{self, Config, DarpPaths, ResolvedSettings, ServiceContext};
use crate::engine::{Engine, EngineKind};

fn add_platform_args(cmd: &mut std::process::Command, engine: &Engine, platform: &str) {
    match engine.kind {
        EngineKind::Docker => {
            cmd.arg("--platform").arg(platform);
        }
        EngineKind::Podman => {
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
        EngineKind::None => {}
    }
}

/// Build the common container run command used by both cmd_shell and cmd_serve.
fn build_container_command(
    resolved: &ResolvedSettings,
    ctx: &ServiceContext<'_>,
    image_name: &str,
    interactive: bool,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<std::process::Command> {
    let container_name = format!("darp_{}_{}", resolved.domain_name, resolved.service_name);

    let mut cmd = if interactive {
        engine.base_run_interactive(&container_name)
    } else {
        engine.base_run_noninteractive(&container_name)
    };

    if engine.is_docker() {
        cmd.arg("--add-host")
            .arg("host.docker.internal:host-gateway");
    }

    cmd.arg("-v")
        .arg(format!("{}:/app", ctx.current_dir.display()))
        .arg("-v")
        .arg(format!(
            "{}:/etc/hosts",
            paths.hosts_container_path.display()
        ))
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

    if let Some(vols) = &resolved.volumes {
        for v in vols {
            let host = config.resolve_host_path(&v.host, &ctx.current_dir)?;
            if !host.exists() {
                eprintln!("Volume {} does not appear to exist.", v.host);
                std::process::exit(1);
            }
            cmd.arg("-v")
                .arg(format!("{}:{}", host.display(), v.container));
        }
    }

    if let Some(pm) = &resolved.host_portmappings {
        for (host_port, container_port) in pm {
            cmd.arg("-p").arg(format!(
                "{host}:{container}",
                host = host_port,
                container = container_port
            ));
        }
    }

    if let Some(vars) = &resolved.variables {
        for (name, value) in vars {
            cmd.arg("-e")
                .arg(format!("{name}={value}", name = name, value = value));
        }
    }

    if let Some(ref platform) = resolved.platform {
        add_platform_args(&mut cmd, engine, platform);
    }

    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    // Portmap entries are either a bare number (legacy) or {"port": N, "type": "..."}.
    let rev_proxy_port = portmap
        .get(&resolved.domain_name)
        .and_then(|d| d.get(&resolved.group_name))
        .and_then(|g| g.get(&resolved.service_name))
        .and_then(|v| {
            v.get("port")
                .and_then(|p| p.as_u64())
                .or_else(|| v.as_u64())
        })
        .unwrap_or_else(|| {
            eprintln!(
                "port not yet assigned to {}, run 'darp deploy'",
                resolved.service_name
            );
            std::process::exit(1);
        });

    // Container-internal port convention keyed off connection_type:
    //   http      -> 8000 (default)
    //   websocket -> 8001
    //   tcp       -> 8002
    let container_port: u16 = match resolved.connection_type.as_deref() {
        Some("websocket") => 8001,
        Some("tcp") => 8002,
        _ => 8000,
    };
    cmd.arg("-p")
        .arg(format!("{}:{}", rev_proxy_port, container_port));
    cmd.arg(image_name);

    Ok(cmd)
}

pub fn cmd_shell(
    environment_cli: Option<String>,
    dry_run: bool,
    container_image: Option<String>,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

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
        ctx.current_directory_name.clone(),
        ctx.environment_name.clone(),
        ctx.service,
        ctx.group,
        ctx.domain,
        ctx.environment,
    );

    let container_name = format!("darp_{}_{}", ctx.domain_name, ctx.current_directory_name);
    let shell_command = resolved.shell_command.as_deref().unwrap_or("sh");

    if engine.is_container_running(&container_name) {
        if dry_run {
            println!(
                "{} exec -it {} sh -c 'cd /app; exec {}'",
                engine.bin.unwrap_or("docker"),
                container_name,
                shell_command
            );
            return Ok(());
        }

        println!(
            "Attaching to running container {}...",
            ctx.current_directory_name.cyan()
        );
        let bin = engine.bin.expect("engine bin not set");
        let exec_inner = format!("cd /app; exec {}", shell_command);
        let status = std::process::Command::new(bin)
            .arg("exec")
            .arg("-it")
            .arg(&container_name)
            .arg("sh")
            .arg("-c")
            .arg(&exec_inner)
            .status()?;

        if let Some(code) = status.code() {
            if code != 0 {
                println!("exiting with status code {}", code);
            }
        }
        return Ok(());
    }

    let image_name = resolved
        .resolve_full_image_name(container_image.as_deref())
        .unwrap_or_else(|| {
            eprintln!(
                "No container image provided for '{}.{}'.\n\
                 Either pass an explicit image to 'darp shell' or configure a default_container_image:\n\
                   darp config set svc default-container-image {} {} <image>\n\
                 or\n\
                   darp config set env default-container-image <env> <image>",
                ctx.domain_name,
                ctx.current_directory_name,
                ctx.domain_name,
                ctx.current_directory_name,
            );
            std::process::exit(1);
        });

    let mut cmd =
        build_container_command(&resolved, &ctx, &image_name, true, paths, config, engine)?;

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

    cmd.arg("sh").arg("-c").arg(inner_cmd);

    if dry_run {
        println!("{}", engine.command_to_string(&cmd));
        return Ok(());
    }

    engine.run_container_interactive(cmd, &container_name, &[])?;
    Ok(())
}

pub fn cmd_serve(
    environment_cli: Option<String>,
    dry_run: bool,
    container_image: Option<String>,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let ctx = config
        .service_context_from_cwd(environment_cli)
        .unwrap_or_else(|| {
            eprintln!("Current directory does not exist in any darp domain configuration.");
            std::process::exit(1);
        });

    let environment_name = match &ctx.environment_name {
        Some(name) => name.clone(),
        None => {
            eprintln!(
                "Environment is required for 'darp serve' in domain '{}'.\n\
Either pass an explicit environment:\n  darp serve --environment <env>\n\
or configure a default_environment for this domain:\n  darp config set dom default-environment {} <env>",
                ctx.domain_name, ctx.domain_name
            );
            std::process::exit(1);
        }
    };

    if ctx.environment.is_none() {
        eprintln!("Environment '{}' does not exist.", environment_name);
        std::process::exit(1);
    }

    let resolved = ResolvedSettings::resolve(
        ctx.domain_name.clone(),
        ctx.group_name.clone(),
        ctx.current_directory_name.clone(),
        ctx.environment_name.clone(),
        ctx.service,
        ctx.group,
        ctx.domain,
        ctx.environment,
    );

    let serve_command = resolved.serve_command.as_deref().unwrap_or_else(|| {
        eprintln!(
            "Neither service '{}.{}', domain '{}', nor environment '{}' has a serve_command configured.\n\
Use 'darp config set svc serve-command {} {} <cmd>' or \
'darp config set dom serve-command {} <cmd>' or \
'darp config set env serve-command {} <cmd>' first.",
            ctx.domain_name,
            ctx.current_directory_name,
            ctx.domain_name,
            environment_name,
            ctx.domain_name,
            ctx.current_directory_name,
            ctx.domain_name,
            environment_name,
        );
        std::process::exit(1);
    });

    let container_name = format!("darp_{}_{}", ctx.domain_name, ctx.current_directory_name);

    if engine.is_container_running(&container_name) {
        let serve_binary = serve_command
            .split_whitespace()
            .next()
            .unwrap_or(serve_command);
        if engine.is_process_running_in_container(&container_name, serve_binary) {
            println!(
                "darp is already serving {}",
                ctx.current_directory_name.cyan()
            );
            return Ok(());
        }

        if dry_run {
            println!(
                "{} exec {} sh -c 'cd /app; {}'",
                engine.bin.unwrap_or("docker"),
                container_name,
                serve_command
            );
            return Ok(());
        }

        println!(
            "Starting serve in running container {}...",
            ctx.current_directory_name.cyan()
        );
        let bin = engine.bin.expect("engine bin not set");
        let exec_inner = format!("cd /app; {}", serve_command);
        let status = std::process::Command::new(bin)
            .arg("exec")
            .arg(&container_name)
            .arg("sh")
            .arg("-c")
            .arg(&exec_inner)
            .status()?;

        if let Some(code) = status.code() {
            if code != 0 {
                println!("exiting with status code {}", code);
            }
        }
        return Ok(());
    }

    let image_name = resolved
        .resolve_full_image_name(container_image.as_deref())
        .unwrap_or_else(|| {
            eprintln!(
                "No container image provided for '{}.{}' in environment '{}'.\n\
                 Either pass an explicit image to 'darp serve' or configure a default_container_image:\n\
                   darp config set svc default-container-image {} {} <image>\n\
                 or\n\
                   darp config set env default-container-image {} <image>",
                ctx.domain_name,
                ctx.current_directory_name,
                environment_name,
                ctx.domain_name,
                ctx.current_directory_name,
                environment_name,
            );
            std::process::exit(1);
        });

    let mut cmd =
        build_container_command(&resolved, &ctx, &image_name, false, paths, config, engine)?;

    let inner_cmd = format!(
        r#"if command -v nginx >/dev/null 2>&1; then
    echo "Starting nginx..."; nginx;
else
    echo "nginx not found, skipping";
fi;
cd /app; {serve}"#,
        serve = serve_command
    );

    cmd.arg("sh").arg("-c").arg(inner_cmd);

    if dry_run {
        println!("{}", engine.command_to_string(&cmd));
        return Ok(());
    }

    engine.run_container_interactive(cmd, &container_name, &[])?;
    Ok(())
}
