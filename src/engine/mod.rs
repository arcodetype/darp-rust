// engine/mod.rs

use crate::config::Config;
use crate::config::DarpPaths;
use anyhow::{Result, anyhow};
use colored::*;
use std::ffi::OsStr;
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub enum EngineKind {
    Podman,
    Docker,
    None,
}

impl EngineKind {
    pub fn from_config(config: &Config) -> Self {
        match config.engine.as_deref().map(|s| s.to_lowercase()) {
            Some(ref e) if e == "docker" => EngineKind::Docker,
            Some(ref e) if e == "podman" => EngineKind::Podman,
            _ => EngineKind::None,
        }
    }

    pub fn bin(&self) -> Option<&'static str> {
        match self {
            EngineKind::Podman => Some("podman"),
            EngineKind::Docker => Some("docker"),
            EngineKind::None => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EngineKind::Podman => "podman",
            EngineKind::Docker => "docker",
            EngineKind::None => "none",
        }
    }
}

/// Read the cached host-gateway IP. Returns `None` if the file is missing, malformed,
/// or was written for a different engine (so the caller re-probes after an engine switch).
pub fn read_container_host_ip(path: &std::path::Path, kind: &EngineKind) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    let cached_kind = lines.next()?.trim();
    let cached_ip = lines.next()?.trim();
    if cached_kind == kind.as_str() && !cached_ip.is_empty() {
        Some(cached_ip.to_string())
    } else {
        None
    }
}

pub fn write_container_host_ip(path: &std::path::Path, kind: &EngineKind, ip: &str) -> Result<()> {
    std::fs::write(path, format!("{}\n{}\n", kind.as_str(), ip))
        .map_err(|e| anyhow!("failed to write {}: {}", path.display(), e))
}

pub struct Engine {
    pub kind: EngineKind,
    pub bin: Option<&'static str>,
    pub podman_machine: Option<String>,
}

impl Engine {
    pub fn new(kind: EngineKind, config: &Config) -> Result<Self> {
        let podman_machine = config.podman_machine.clone();

        Ok(Self {
            bin: kind.bin(),
            kind,
            podman_machine,
        })
    }

    pub fn host_gateway(&self) -> &'static str {
        match self.kind {
            EngineKind::Podman => "host.containers.internal",
            EngineKind::Docker => "host.docker.internal",
            EngineKind::None => "localhost",
        }
    }

    pub fn is_docker(&self) -> bool {
        matches!(self.kind, EngineKind::Docker)
    }

    pub fn require_ready(&self) -> Result<()> {
        match self.kind {
            EngineKind::Docker => Command::new("docker")
                .arg("info")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|e| anyhow!("failed to run docker info: {}", e))
                .and_then(|s| {
                    if s.success() {
                        Ok(())
                    } else {
                        Err(anyhow!(
                            "Docker does not appear to be running ({})",
                            "docker info".red()
                        ))
                    }
                }),
            EngineKind::Podman => {
                if cfg!(target_os = "linux") {
                    // On Linux, Podman runs natively without a VM/machine.
                    Command::new("podman")
                        .arg("info")
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status()
                        .map_err(|e| anyhow!("failed to run podman info: {}", e))
                        .and_then(|s| {
                            if s.success() {
                                Ok(())
                            } else {
                                Err(anyhow!(
                                    "Podman does not appear to be running ({})",
                                    "podman info".red()
                                ))
                            }
                        })
                } else {
                    // On macOS/Windows, Podman requires a running machine/VM.
                    let output = Command::new("podman")
                        .arg("machine")
                        .arg("list")
                        .arg("--format")
                        .arg("{{.Name}} {{.Running}}")
                        .output()?;

                    if !output.status.success() {
                        return Err(anyhow!(
                            "Failed to run 'podman machine list': exit {}",
                            output.status
                        ));
                    }

                    let text = String::from_utf8_lossy(&output.stdout);
                    let machine_env = self
                        .podman_machine
                        .as_deref()
                        .unwrap_or("podman-machine-default");

                    for line in text.lines() {
                        let parts: Vec<_> = line.split_whitespace().collect();
                        if parts.len() != 2 {
                            continue;
                        }
                        let name = parts[0].trim_end_matches('*');
                        let running = parts[1];
                        if name == machine_env && running.eq_ignore_ascii_case("true") {
                            return Ok(());
                        }
                    }

                    Err(anyhow!(
                        "Podman machine '{}' appears to be down ({})",
                        machine_env,
                        format!("podman machine start {}", machine_env).red()
                    ))
                }
            }
            EngineKind::None => Err(anyhow!(
                "No container engine is configured.\nUse 'darp set engine podman' or 'darp set engine docker'."
            )),
        }
    }

    pub fn base_run_interactive(&self, container_name: &str) -> Command {
        let bin = self.bin.expect("engine bin not set");
        let mut cmd = Command::new(bin);
        cmd.arg("run")
            .arg("--rm")
            .arg("-it")
            .arg("--name")
            .arg(container_name);
        cmd
    }

    pub fn base_run_noninteractive(&self, container_name: &str) -> Command {
        let bin = self.bin.expect("engine bin not set");
        let mut cmd = Command::new(bin);
        cmd.arg("run").arg("--rm").arg("--name").arg(container_name);
        cmd
    }

    pub fn command_to_string(&self, cmd: &Command) -> String {
        fn shell_escape(s: &OsStr) -> String {
            let s = s.to_string_lossy();

            let safe = s.chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || matches!(c, '_' | '-' | '.' | '/' | ':' | '=' | ',' | '@' | '+')
            });

            if safe {
                s.into_owned()
            } else {
                // Single-quote wrap; escape internal single quotes: ' -> '\''
                let mut out = String::from("'");
                for ch in s.chars() {
                    if ch == '\'' {
                        out.push_str("'\\''");
                    } else {
                        out.push(ch);
                    }
                }
                out.push('\'');
                out
            }
        }

        let mut parts = Vec::new();
        parts.push(shell_escape(cmd.get_program()));
        for arg in cmd.get_args() {
            parts.push(shell_escape(arg));
        }
        parts.join(" ")
    }

    pub fn is_container_running(&self, name: &str) -> bool {
        let Some(bin) = self.bin else { return false };
        let output = Command::new(bin)
            .arg("ps")
            .arg("--format")
            .arg("{{.Names}}")
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                return text.lines().any(|l| l.trim() == name);
            }
        }
        false
    }

    pub fn is_process_running_in_container(&self, container_name: &str, process: &str) -> bool {
        let Some(bin) = self.bin else { return false };
        let output = Command::new(bin).arg("top").arg(container_name).output();
        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                return text.lines().skip(1).any(|line| line.contains(process));
            }
        }
        false
    }

    pub fn is_engine_installed(&self) -> bool {
        let Some(bin) = self.bin else { return false };
        Command::new("which")
            .arg(bin)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Ask the engine to expand `host-gateway` in a throwaway container and return the IP.
    ///
    /// Docker Desktop and Docker Engine / Podman each map `host-gateway` to a different
    /// IP (VM-internal bridge vs. host-routable magic address), so this is the only way
    /// to learn the platform-correct value without guessing.
    pub fn probe_host_gateway_ip(&self) -> Result<String> {
        let bin = self
            .bin
            .ok_or_else(|| anyhow!("no container engine configured"))?;

        const PROBE_HOST: &str = "_darp_probe_";

        let output = Command::new(bin)
            .arg("run")
            .arg("--rm")
            .arg("--add-host")
            .arg(format!("{PROBE_HOST}:host-gateway"))
            .arg("nginx")
            .arg("cat")
            .arg("/etc/hosts")
            .output()
            .map_err(|e| anyhow!("failed to run probe container: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "probe container exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let mut parts = line.split_whitespace();
            let ip = parts.next();
            let name = parts.next();
            if name == Some(PROBE_HOST) {
                if let Some(ip) = ip {
                    return Ok(ip.to_string());
                }
            }
        }

        Err(anyhow!(
            "probe container did not return a {} entry in /etc/hosts",
            PROBE_HOST
        ))
    }

    pub fn start_reverse_proxy(&self, paths: &DarpPaths) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        const REVERSE_PROXY: &str = "darp-reverse-proxy";

        if self.is_container_running(REVERSE_PROXY) {
            return Ok(());
        }

        println!("starting {}", REVERSE_PROXY.green());

        let mut cmd = Command::new(bin);
        cmd.arg("run")
            .arg("-d")
            .arg("--rm")
            .arg("--name")
            .arg(REVERSE_PROXY)
            .arg("-p")
            .arg("80:80")
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

        if self.is_docker() {
            cmd.arg("--add-host")
                .arg("host.docker.internal:host-gateway");
        }

        cmd.arg("nginx:alpine")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(())
    }

    pub fn restart_reverse_proxy(&self, paths: &DarpPaths) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        const REVERSE_PROXY: &str = "darp-reverse-proxy";

        if !self.is_container_running(REVERSE_PROXY) {
            return self.start_reverse_proxy(paths);
        }

        println!("restarting {}", REVERSE_PROXY.green());

        Command::new(bin)
            .arg("restart")
            .arg(REVERSE_PROXY)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(())
    }

    pub fn start_darp_masq(&self, paths: &DarpPaths) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        const DNSMASQ: &str = "darp-masq";

        if self.is_container_running(DNSMASQ) {
            return Ok(());
        }

        println!("starting {}", DNSMASQ.green());

        let mut cmd = Command::new(bin);
        cmd.arg("run")
            .arg("-d")
            .arg("--rm")
            .arg("--name")
            .arg(DNSMASQ)
            .arg("-p")
            .arg("53:53/udp")
            .arg("-p")
            .arg("53:53/tcp")
            .arg("-v")
            .arg(format!("{}:/etc/dnsmasq.d", paths.dnsmasq_dir.display()))
            .arg("--cap-add=NET_ADMIN");

        if self.is_docker() {
            cmd.arg("--add-host")
                .arg("host.docker.internal:host-gateway");
        }

        cmd.arg("dockurr/dnsmasq")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    pub fn stop_running_darps(&self) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        let output = Command::new(bin)
            .arg("ps")
            .arg("--format")
            .arg("{{.Names}}")
            .output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        for name in text.lines() {
            let name = name.trim();
            if name.starts_with("darp_") {
                println!("stopping {}", name.cyan());
                Command::new(bin)
                    .arg("stop")
                    .arg(name)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
            }
        }
        Ok(())
    }

    pub fn stop_named_container(&self, name: &str) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        if !self.is_container_running(name) {
            return Ok(());
        }
        println!("stopping {}", name.cyan());
        Command::new(bin)
            .arg("stop")
            .arg(name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    pub fn run_container_interactive(
        &self,
        mut cmd: Command,
        container_name: &str,
        restart_on: &[i32],
    ) -> Result<()> {
        let restart_on: Vec<i32> = restart_on.to_vec();
        let bin = self.bin.expect("engine bin not set").to_string();

        loop {
            let mut child = cmd.spawn()?;

            let container_name_for_handler = container_name.to_string();
            let bin_clone = bin.clone();

            ctrlc::set_handler(move || {
                eprintln!("\nStopping {} (Ctrl+C)", container_name_for_handler.cyan());
                // Best-effort stop
                let _ = Command::new(&bin_clone)
                    .arg("stop")
                    .arg(&container_name_for_handler)
                    .status();
            })?;

            let status = child.wait()?;

            if let Some(code) = status.code() {
                if restart_on.contains(&code) {
                    println!("restarting {} with code {}", container_name.cyan(), &code);
                    continue;
                }
                println!("exiting with status code {}", &code);
            }

            // Normal exit or non-restartable error
            break;
        }

        Ok(())
    }

    pub fn configure_unprivileged_ports_if_needed(&self) -> Result<()> {
        // Keep behavior only for podman + mac/linux; for Docker we skip.
        if let EngineKind::Podman = self.kind {
            // You can mirror your Python sysctl/Podman logic here if you want.
            // For now we leave it as a no-op stub.
        }
        Ok(())
    }
}
