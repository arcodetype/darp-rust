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

/// How a container's stdio is attached to the terminal.
#[derive(Clone, Copy)]
struct Attach {
    interactive: bool,
    tty: bool,
}

impl Attach {
    /// `darp shell` — always a pty.
    fn shell() -> Self {
        Self {
            interactive: true,
            tty: true,
        }
    }

    /// `darp serve` — detached stdio.
    fn serve() -> Self {
        Self {
            interactive: false,
            tty: false,
        }
    }

    /// `darp exec` — stdin is always forwarded; a pty only if the host has one.
    fn exec() -> Self {
        Self {
            interactive: true,
            tty: host_is_tty(),
        }
    }
}

/// A pty is only safe when *both* ends are a terminal.
///
/// stdout alone is not enough: `echo input | darp exec cat` has a tty stdout but a
/// piped stdin, and `-t` there makes the engine allocate a pty that echoes the piped
/// bytes back and rewrites `\n` to `\r\n`, corrupting the stream.
fn host_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Build the common container run command used by cmd_shell, cmd_serve and cmd_exec.
fn build_container_command(
    resolved: &ResolvedSettings,
    ctx: &ServiceContext<'_>,
    image_name: &str,
    attach: Attach,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<std::process::Command> {
    let container_name = format!("darp_{}_{}", resolved.domain_name, resolved.service_name);

    let portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));

    // Reverse-proxy port must have been assigned by `darp deploy`.
    let rev_proxy_port = config::portmap_proxy_port(
        &portmap,
        &resolved.domain_name,
        &resolved.group_name,
        &resolved.service_name,
    )
    .unwrap_or_else(|| {
        eprintln!(
            "port not yet assigned to {}, run 'darp deploy'",
            resolved.service_name
        );
        std::process::exit(1);
    });

    // Debug port is assigned by `darp deploy`; fall back to the base for stale portmaps
    // written before this feature (so pre-upgrade deploys keep working).
    let debug_port = config::portmap_debug_port(
        &portmap,
        &resolved.domain_name,
        &resolved.group_name,
        &resolved.service_name,
    )
    .unwrap_or(config::DEBUG_PORT_BASE);

    let tokens = config::TokenCtx {
        domain: &resolved.domain_name,
        group: &resolved.group_name,
        service: &resolved.service_name,
        debug_port,
        proxy_port: Some(rev_proxy_port),
    };

    let mut cmd = engine.base_run(&container_name, attach.interactive, attach.tty);

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
        let domain_loc = config::resolve_location(&ctx.domain.location)?;
        for v in vols {
            let host = config.resolve_host_path(&v.host, &ctx.current_dir, &domain_loc)?;
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
                host = config::substitute_tokens(host_port, &tokens),
                container = config::substitute_tokens(container_port, &tokens)
            ));
        }
    }

    if let Some(vars) = &resolved.variables {
        for (name, value) in vars {
            cmd.arg("-e").arg(format!(
                "{name}={value}",
                name = name,
                value = config::substitute_tokens(value, &tokens)
            ));
        }
    }

    if let Some(ref platform) = resolved.platform {
        add_platform_args(&mut cmd, engine, platform);
    }

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

    let mut cmd = build_container_command(
        &resolved,
        &ctx,
        &image_name,
        Attach::shell(),
        paths,
        config,
        engine,
    )?;

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

/// `$0` handed to the in-container `sh`, so its diagnostics identify themselves
/// (e.g. `darp-exec: exec: php: not found`).
pub const EXEC_ARGV0: &str = "darp-exec";

/// Inner script for `darp exec`: enter the bind-mounted project directory, then
/// replace the shell with the user's argv.
///
/// The argv arrives through `"$@"` rather than being interpolated into the script,
/// so nothing is re-split, re-quoted or re-globbed on the way in. `&&` rather than
/// `;` so a missing `/app` fails loudly instead of silently running the command in
/// `/`. `exec` makes the user's process the container's PID 1 on the cold-start
/// path, so signals and the exit code pass straight through.
pub const EXEC_SCRIPT: &str = r#"cd /app && exec "$@""#;

/// The trailing `sh -c <script> darp-exec <args...>` shared by both the exec path
/// and the cold-start run path.
pub fn exec_wrapper_argv(args: &[String]) -> Vec<String> {
    let mut out = vec![
        "sh".to_string(),
        "-c".to_string(),
        EXEC_SCRIPT.to_string(),
        EXEC_ARGV0.to_string(),
    ];
    out.extend(args.iter().cloned());
    out
}

/// Full argv for exec'ing into an already-running container.
///
/// Pure so it can be asserted on without a container engine, and so the real command
/// and the `--dry-run` string are built from the same source.
pub fn container_exec_argv(
    bin: &str,
    container_name: &str,
    tty: bool,
    args: &[String],
) -> Vec<String> {
    let mut out = vec![bin.to_string(), "exec".to_string(), "-i".to_string()];
    if tty {
        out.push("-t".to_string());
    }
    out.push(container_name.to_string());
    out.extend(exec_wrapper_argv(args));
    out
}

/// Map a child's exit status onto darp's own exit code.
///
/// A signal-terminated child follows the shell convention of 128 + signal, so
/// Ctrl+C surfaces as 130. `None` here means the engine CLI itself was killed;
/// if the *container's* process dies on a signal the engine already reports
/// 128 + signal as its own ordinary exit code.
pub fn exit_code_for(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return 128 + sig;
        }
    }
    1
}

fn command_from_argv(argv: &[String]) -> std::process::Command {
    let mut cmd = std::process::Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    cmd
}

pub fn cmd_exec(
    environment_cli: Option<String>,
    dry_run: bool,
    image_cli: Option<String>,
    args: Vec<String>,
    paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<i32> {
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

    // The service container is already up: run the command inside it. Nothing is
    // printed around the command — stdout belongs to it, so `darp exec` stays
    // safe to pipe.
    if engine.is_container_running(&container_name) {
        let bin = engine.bin.expect("engine bin not set");
        let mut cmd = command_from_argv(&container_exec_argv(
            bin,
            &container_name,
            host_is_tty(),
            &args,
        ));

        if dry_run {
            println!("{}", engine.command_to_string(&cmd));
            return Ok(0);
        }

        return Ok(exit_code_for(cmd.status()?));
    }

    // Nothing running: cold-start a throwaway container with the same volumes,
    // variables and ports `darp shell` would give you, minus nginx (a one-off
    // command isn't serving, and the banner would pollute its stdout).
    let image_name = resolved
        .resolve_full_image_name(image_cli.as_deref())
        .unwrap_or_else(|| {
            eprintln!(
                "No container image provided for '{}.{}'.\n\
                 Either pass an explicit image to 'darp exec --image <image>' or configure a default_container_image:\n\
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

    let mut cmd = build_container_command(
        &resolved,
        &ctx,
        &image_name,
        Attach::exec(),
        paths,
        config,
        engine,
    )?;
    cmd.args(exec_wrapper_argv(&args));

    if dry_run {
        println!("{}", engine.command_to_string(&cmd));
        return Ok(0);
    }

    // stderr, so stdout stays clean for piping.
    eprintln!(
        "{} is not running, starting a one-off container from {}...",
        ctx.current_directory_name.cyan(),
        image_name
    );

    Ok(exit_code_for(cmd.status()?))
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

    // Interpolate {debug_port}/{proxy_port}/… in the serve command so per-service
    // debugger flags (e.g. `dlv --listen=:{debug_port}`) resolve. Ports come from the
    // portmap written by `darp deploy`.
    let serve_portmap: serde_json::Value =
        config::read_json(&paths.portmap_path).unwrap_or_else(|_| serde_json::json!({}));
    let serve_tokens = config::TokenCtx {
        domain: &resolved.domain_name,
        group: &resolved.group_name,
        service: &resolved.service_name,
        debug_port: config::portmap_debug_port(
            &serve_portmap,
            &resolved.domain_name,
            &resolved.group_name,
            &resolved.service_name,
        )
        .unwrap_or(config::DEBUG_PORT_BASE),
        proxy_port: config::portmap_proxy_port(
            &serve_portmap,
            &resolved.domain_name,
            &resolved.group_name,
            &resolved.service_name,
        ),
    };
    let serve_command = config::substitute_tokens(serve_command, &serve_tokens);
    let serve_command = serve_command.as_str();

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

    let mut cmd = build_container_command(
        &resolved,
        &ctx,
        &image_name,
        Attach::serve(),
        paths,
        config,
        engine,
    )?;

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
