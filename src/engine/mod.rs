// engine/mod.rs

use crate::config::Config;
use crate::config::DarpPaths;
use anyhow::{Result, anyhow};
use colored::*;
use std::ffi::OsStr;
use std::path::Path;
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

    /// `run --rm [-it|-i|-t] --name <name>`.
    ///
    /// The stdin and tty flags are separate so `darp exec` can forward stdin without
    /// demanding a pty the host may not have (piped input, CI).
    pub fn base_run(&self, container_name: &str, interactive: bool, tty: bool) -> Command {
        let bin = self.bin.expect("engine bin not set");
        let mut cmd = Command::new(bin);
        cmd.arg("run").arg("--rm");
        match (interactive, tty) {
            (true, true) => {
                cmd.arg("-it");
            }
            (true, false) => {
                cmd.arg("-i");
            }
            (false, true) => {
                cmd.arg("-t");
            }
            (false, false) => {}
        }
        cmd.arg("--name").arg(container_name);
        cmd
    }

    pub fn base_run_interactive(&self, container_name: &str) -> Command {
        self.base_run(container_name, true, true)
    }

    pub fn base_run_noninteractive(&self, container_name: &str) -> Command {
        self.base_run(container_name, false, false)
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
            .arg("80:80");
        self.add_reverse_proxy_args(
            &mut cmd,
            &paths.nginx_conf_path,
            &paths.vhost_container_conf,
        );

        cmd.arg("nginx:alpine")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        Ok(())
    }

    /// Mount the nginx configs the reverse proxy runs with, and let it resolve the
    /// host gateway name its vhosts proxy to. Shared by the real proxy and the
    /// `nginx -t` preflight so the check sees exactly what the proxy will.
    fn add_reverse_proxy_args(&self, cmd: &mut Command, nginx_conf: &Path, vhost_conf: &Path) {
        cmd.arg("-v")
            .arg(format!("{}:/etc/nginx/nginx.conf", nginx_conf.display()))
            .arg("-v")
            .arg(format!(
                "{}:/etc/nginx/http.d/vhost_container.conf",
                vhost_conf.display()
            ));

        if self.is_docker() {
            cmd.arg("--add-host")
                .arg("host.docker.internal:host-gateway");
        }
    }

    /// Run `nginx -t` against candidate configs in a throwaway `nginx:alpine`
    /// container. On failure the error carries nginx's own output.
    pub fn test_nginx_config(&self, nginx_conf: &Path, vhost_conf: &Path) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };

        let mut cmd = Command::new(bin);
        cmd.arg("run").arg("--rm");
        self.add_reverse_proxy_args(&mut cmd, nginx_conf, vhost_conf);
        let output = cmd
            .arg("nginx:alpine")
            .arg("nginx")
            .arg("-t")
            .output()
            .map_err(|e| anyhow!("failed to run nginx config check: {}", e))?;

        if output.status.success() {
            return Ok(());
        }

        // nginx prints each error twice: once to its error log (on stderr in the
        // container, prefixed "2026/09/16 18:25:54 [emerg] 1#1: ...") and once as
        // "nginx: [emerg] ...". Keep the second form.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<&str> = stderr
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !is_nginx_log_line(l))
            .collect();
        Err(anyhow!("{}", lines.join("\n")))
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

    /// `{engine} port <name>` output, or an empty string when it can't be read.
    pub fn container_port_bindings(&self, name: &str) -> String {
        let Some(bin) = self.bin else {
            return String::new();
        };
        match Command::new(bin).arg("port").arg(name).output() {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
            _ => String::new(),
        }
    }

    /// Force-remove a container, so its name can be reused. A container that doesn't
    /// exist is not an error; any other failure carries the engine's own message.
    fn remove_container(&self, name: &str) -> Result<()> {
        let Some(bin) = self.bin else { return Ok(()) };
        let output = Command::new(bin).arg("rm").arg("-f").arg(name).output()?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success() || is_missing_container_error(&stderr) {
            return Ok(());
        }
        Err(anyhow!("could not remove {name}: {}", stderr.trim()))
    }

    /// Put darp-masq where `dns_port` says ([`darp_masq_publish_args`]), returning
    /// whether it is running there. `false` means a warning was printed and deploy
    /// should leave the resolver file alone, so it keeps matching what is running.
    ///
    /// A running darp-masq on the right port is left alone. One published elsewhere is
    /// only removed once its replacement is up, so a port that turns out to be taken
    /// costs nothing: the old one keeps serving. The exception is a darp-masq already
    /// holding the target port (say on another interface), which has to go first.
    pub fn start_darp_masq(&self, paths: &DarpPaths, dns_port: Option<u16>) -> Result<bool> {
        if self.bin.is_none() {
            return Ok(false);
        }
        const DNSMASQ: &str = "darp-masq";
        const REPLACEMENT: &str = "darp-masq-next";

        let running = self.is_container_running(DNSMASQ);
        let bindings = if running {
            self.container_port_bindings(DNSMASQ)
        } else {
            String::new()
        };
        if running && darp_masq_ports_match(&bindings, dns_port) {
            return Ok(true);
        }

        // A replacement left by an interrupted deploy would hold its name and the port.
        if let Err(e) = self.remove_container(REPLACEMENT) {
            eprintln!("\nWarning: {e}");
            return Ok(false);
        }

        if running && !darp_masq_holds_target_port(&bindings, dns_port) {
            println!(
                "moving {} to {}",
                DNSMASQ.green(),
                darp_masq_bind_label(dns_port)
            );
            return self.replace_darp_masq(paths, DNSMASQ, REPLACEMENT, dns_port);
        }

        // Nothing running to preserve: this clears a darp-masq holding the target port,
        // or a stopped one created by hand without --rm that would hold the name.
        if let Err(e) = self.remove_container(DNSMASQ) {
            eprintln!("\nWarning: {e}");
            return Ok(false);
        }

        println!("starting {}", DNSMASQ.green());
        let output = self.run_darp_masq(paths, DNSMASQ, dns_port)?;
        if !output.status.success() {
            eprintln!(
                "\n{}",
                darp_masq_start_warning(&String::from_utf8_lossy(&output.stderr), dns_port)
            );
            return Ok(false);
        }
        Ok(true)
    }

    /// Start the replacement beside the running darp-masq, then swap it in. Any failure
    /// before the old one is gone leaves the old one serving.
    fn replace_darp_masq(
        &self,
        paths: &DarpPaths,
        current: &str,
        replacement: &str,
        dns_port: Option<u16>,
    ) -> Result<bool> {
        let output = self.run_darp_masq(paths, replacement, dns_port)?;
        if !output.status.success() {
            // A failed start can leave the created container behind.
            let _ = self.remove_container(replacement);
            eprintln!(
                "\n{}",
                darp_masq_move_warning(&String::from_utf8_lossy(&output.stderr), dns_port)
            );
            return Ok(false);
        }

        if let Err(e) = self.remove_container(current) {
            let _ = self.remove_container(replacement);
            eprintln!("\n{}", darp_masq_move_warning(&e.to_string(), dns_port));
            return Ok(false);
        }

        // The replacement is serving on the right port by now, so a failed rename only
        // leaves it under the wrong name; the next deploy clears that up.
        let bin = self.bin.expect("checked by start_darp_masq");
        let renamed = Command::new(bin)
            .arg("rename")
            .arg(replacement)
            .arg(current)
            .output()?;
        if !renamed.status.success() {
            eprintln!(
                "\nWarning: {current} is running as {replacement}, and could not be renamed: {}",
                String::from_utf8_lossy(&renamed.stderr).trim()
            );
        }
        Ok(true)
    }

    /// Pull darp-masq's image if it's missing, with the engine's progress on screen.
    /// A failed pull isn't reported here: the `run` that follows fails with the reason.
    fn ensure_darp_masq_image(&self, bin: &str) {
        let present = Command::new(bin)
            .arg("image")
            .arg("inspect")
            .arg(DARP_MASQ_IMAGE)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        if !present {
            println!("pulling {}", DARP_MASQ_IMAGE.green());
            let _ = Command::new(bin).arg("pull").arg(DARP_MASQ_IMAGE).status();
        }
    }

    /// `{engine} run` a darp-masq under `name`, returning the engine's output. Output
    /// is captured to report a failed start, so the image is pulled beforehand.
    fn run_darp_masq(
        &self,
        paths: &DarpPaths,
        name: &str,
        dns_port: Option<u16>,
    ) -> Result<std::process::Output> {
        let bin = self.bin.expect("checked by start_darp_masq");
        self.ensure_darp_masq_image(bin);

        let mut cmd = Command::new(bin);
        cmd.arg("run").arg("-d").arg("--rm").arg("--name").arg(name);
        for publish in darp_masq_publish_args(dns_port) {
            cmd.arg("-p").arg(publish);
        }
        cmd.arg("-v")
            .arg(format!("{}:/etc/dnsmasq.d", paths.dnsmasq_dir.display()))
            .arg("--cap-add=NET_ADMIN");

        if self.is_docker() {
            cmd.arg("--add-host")
                .arg("host.docker.internal:host-gateway");
        }

        Ok(cmd.arg(DARP_MASQ_IMAGE).output()?)
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
                    println!("restarting {} with code {}", container_name.cyan(), code);
                    continue;
                }
                println!("exiting with status code {}", code);
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

const DARP_MASQ_IMAGE: &str = "dockurr/dnsmasq";

/// darp-masq's `-p` values: darp's default `53:53` on every interface, or
/// `127.0.0.1:{port}:53` once `dns_port` moves it. dnsmasq listens on 53 inside the
/// container either way.
pub fn darp_masq_publish_args(dns_port: Option<u16>) -> Vec<String> {
    let host = match dns_port {
        Some(port) => format!("127.0.0.1:{port}"),
        None => "53".to_string(),
    };
    ["udp", "tcp"]
        .iter()
        .map(|proto| format!("{host}:53/{proto}"))
        .collect()
}

/// Where darp-masq is published, as printed by deploy and doctor.
pub fn darp_masq_bind_label(dns_port: Option<u16>) -> String {
    match dns_port {
        Some(port) => format!("127.0.0.1:{port}"),
        None => "port 53".to_string(),
    }
}

/// The warning deploy prints when the engine refuses to start darp-masq, carrying the
/// engine's own error.
pub fn darp_masq_start_warning(stderr: &str, dns_port: Option<u16>) -> String {
    let headline = format!(
        "Warning: darp-masq did not start on {}, so .test names that aren't in /etc/hosts won't resolve:",
        darp_masq_bind_label(dns_port)
    );
    darp_masq_warning(&headline, stderr, dns_port)
}

/// The warning deploy prints when a running darp-masq couldn't be moved. Nothing was
/// lost: the old one is still serving and the resolver file still points at it.
pub fn darp_masq_move_warning(stderr: &str, dns_port: Option<u16>) -> String {
    let headline = format!(
        "Warning: could not move darp-masq to {}. The running darp-masq and /etc/resolver/test were left as they were:",
        darp_masq_bind_label(dns_port)
    );
    darp_masq_warning(&headline, stderr, dns_port)
}

/// `headline`, then the engine's error. A bind failure on port 53 also names
/// `dns-port`, since the usual cause is another program holding 53.
fn darp_masq_warning(headline: &str, stderr: &str, dns_port: Option<u16>) -> String {
    let mut msg = format!("{headline}\n");
    let error = stderr.trim();
    if error.is_empty() {
        msg.push_str("  (no error output from the container engine)\n");
    } else {
        for line in error.lines() {
            msg.push_str(&format!("  {line}\n"));
        }
    }
    if dns_port.is_none() && is_port_conflict_error(error) {
        msg.push_str(
            "\nIf another program holds port 53 (a VPN's DNS proxy, for example), move darp's\n\
             DNS server off it, then re-run 'darp deploy':\n\
             \n  darp config set dns-port <port>\n",
        );
    }
    msg.trim_end().to_string()
}

/// Whether an engine error means a host port was taken, as opposed to, say, a container
/// name being "already in use".
fn is_port_conflict_error(stderr: &str) -> bool {
    let stderr = stderr.to_lowercase();
    [
        "address already in use",
        "port is already allocated",
        "ports are not available",
    ]
    .iter()
    .any(|needle| stderr.contains(needle))
}

/// Whether `{engine} rm` failed only because the container doesn't exist, which some
/// engine versions report with a non-zero exit.
fn is_missing_container_error(stderr: &str) -> bool {
    stderr.to_lowercase().contains("no such container")
}

/// `(protocol, host address)` for each of darp-masq's port 53 lines in `{engine} port`
/// output (`53/udp -> 0.0.0.0:53`).
fn darp_masq_bindings(port_output: &str) -> Vec<(&str, &str)> {
    port_output
        .lines()
        .filter_map(|line| {
            let (container, host) = line.split_once("->")?;
            let proto = container.trim().strip_prefix("53/")?;
            Some((proto, host.trim()))
        })
        .collect()
}

/// Whether `{engine} port darp-masq` output shows it already holding the host port
/// `dns_port` calls for, on any interface. A replacement can't start beside it then.
pub fn darp_masq_holds_target_port(port_output: &str, dns_port: Option<u16>) -> bool {
    let target = dns_port.unwrap_or(crate::config::DEFAULT_DNS_PORT);
    darp_masq_bindings(port_output).iter().any(|(_, host)| {
        host.rsplit_once(':')
            .and_then(|(_, port)| port.parse::<u16>().ok())
            == Some(target)
    })
}

/// Whether `{engine} port darp-masq` output (`53/udp -> 0.0.0.0:53` lines) shows it
/// published where `dns_port` wants it. A custom port needs exactly UDP and TCP on
/// `127.0.0.1:{port}`. The default needs every binding on host port 53: a darp-masq
/// on 53, or one whose bindings can't be read, is left running, while one published
/// on any other port — moved there by `dns_port` or by hand — is replaced.
pub fn darp_masq_ports_match(port_output: &str, dns_port: Option<u16>) -> bool {
    let bindings = darp_masq_bindings(port_output);

    match dns_port {
        Some(port) => {
            let want = format!("127.0.0.1:{port}");
            bindings.iter().all(|(_, host)| *host == want)
                && ["udp", "tcp"]
                    .iter()
                    .all(|p| bindings.iter().any(|(proto, _)| proto == p))
        }
        None => bindings
            .iter()
            .all(|(_, host)| host.rsplit_once(':').map(|(_, port)| port) == Some("53")),
    }
}

/// True for nginx error-log lines, which start with a `YYYY/MM/DD HH:MM:SS` stamp.
fn is_nginx_log_line(line: &str) -> bool {
    let b = line.as_bytes();
    b.len() >= 19
        && b[4] == b'/'
        && b[7] == b'/'
        && b[10] == b' '
        && b[13] == b':'
        && b[16] == b':'
        && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
            .into_iter()
            .all(|r| b[r].iter().all(u8::is_ascii_digit))
}
