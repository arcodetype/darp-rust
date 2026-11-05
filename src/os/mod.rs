use crate::config::{self, Config, DarpPaths};
use crate::engine::EngineKind;
use anyhow::{Result, anyhow};
use colored::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

const NGINX_CONF: &str = include_str!("../../assets/nginx.conf");

/// darp's single-value `http` directives, in the order they appear in `nginx.conf`.
/// The global `nginx` map replaces any of these by name (so nginx never sees a
/// duplicate) and appends names not listed here.
pub const DEFAULT_HTTP_DIRECTIVES: &[(&str, &str)] = &[
    // Type for responses whose extension isn't in mime.types.
    ("default_type", "application/octet-stream"),
    // Don't tell nginx version to the clients. Default is 'on'.
    ("server_tokens", "off"),
    // Maximum accepted request body; larger requests get 413. 0 disables the check.
    ("client_max_body_size", "1m"),
    // Copy data between FDs within the kernel. Default is off.
    ("sendfile", "on"),
    // Send the response head in one packet. Default is 'off'.
    ("tcp_nopush", "on"),
    // Room for long alias hostnames and many vhosts.
    ("server_names_hash_bucket_size", "128"),
    ("server_names_hash_max_size", "512"),
    ("ssl_protocols", "TLSv1.1 TLSv1.2 TLSv1.3"),
    ("ssl_prefer_server_ciphers", "on"),
    ("ssl_session_cache", "shared:SSL:2m"),
    ("ssl_session_timeout", "1h"),
    // TLS session tickets are insecure. Default is 'on'.
    ("ssl_session_tickets", "off"),
    // Set the Vary HTTP header as defined in RFC 2616. Default is 'off'.
    ("gzip_vary", "on"),
];

/// Render the embedded `nginx.conf`, merging the global `nginx` map over
/// [`DEFAULT_HTTP_DIRECTIVES`]. Every override is validated first, because values
/// are spliced in verbatim.
pub fn render_nginx_conf(overrides: Option<&BTreeMap<String, String>>) -> Result<String> {
    let empty = BTreeMap::new();
    let overrides = overrides.unwrap_or(&empty);
    for (name, value) in overrides {
        config::validate_nginx_directive(name, value)?;
    }

    let mut lines = Vec::new();
    for (name, default) in DEFAULT_HTTP_DIRECTIVES {
        let value = overrides.get(*name).map_or(*default, String::as_str);
        lines.push(format!("        {name} {value};"));
    }
    for (name, value) in overrides {
        if !DEFAULT_HTTP_DIRECTIVES.iter().any(|(d, _)| d == name) {
            lines.push(format!("        {name} {value};"));
        }
    }

    Ok(NGINX_CONF.replace("{http_directives}", &lines.join("\n")))
}

pub const HOSTS_HEADER: &str = "# --- DARP HOSTS START ---";
pub const HOSTS_FOOTER: &str = "# --- DARP HOSTS END ---";

/// Parse an existing hosts file, replace the DARP block with new entries,
/// and return the full new file content.
pub fn build_hosts_content(current_raw: &str, hosts_container_lines: &[String]) -> String {
    let current = current_raw.replace("\r\n", "\n");

    let (before, after) = if let Some(s) = current.find(HOSTS_HEADER) {
        if let Some(e) = current[s..].find(HOSTS_FOOTER) {
            let end = s + e + HOSTS_FOOTER.len();
            (
                current[..s].trim_end_matches('\n').to_string(),
                current[end..].trim_start_matches('\n').to_string(),
            )
        } else {
            (current.trim_end_matches('\n').to_string(), String::new())
        }
    } else {
        (current.trim_end_matches('\n').to_string(), String::new())
    };

    let mut block = String::new();
    block.push_str(HOSTS_HEADER);
    block.push('\n');
    for line in hosts_container_lines {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            block.push_str(&format!("127.0.0.1   {}\n", parts[1]));
        }
    }
    block.push_str(HOSTS_FOOTER);
    block.push('\n');

    let mut new_contents = String::new();
    if !before.is_empty() {
        new_contents.push_str(before.trim_end_matches('\n'));
        new_contents.push('\n');
    }
    new_contents.push('\n');
    new_contents.push_str(block.trim_end_matches('\n'));
    new_contents.push('\n');
    if !after.is_empty() {
        new_contents.push('\n');
        new_contents.push_str(after.trim_start_matches('\n'));
        new_contents.push('\n');
    }

    new_contents
}

/// Render `/etc/resolver/test`, which sends `.test` lookups to darp-masq on
/// 127.0.0.1 — at `dns_port` when darp-masq has been moved off port 53.
pub fn render_resolver_conf(dns_port: Option<u16>) -> String {
    match dns_port {
        Some(port) => format!("nameserver 127.0.0.1\nport {port}\n"),
        None => "nameserver 127.0.0.1\n".to_string(),
    }
}

/// The port a resolver file's `port` line names, if it has one.
pub fn resolver_conf_port(contents: &str) -> Option<u16> {
    contents.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        match (words.next(), words.next()) {
            (Some("port"), Some(port)) => port.parse().ok(),
            _ => None,
        }
    })
}

/// Whether `darp deploy` should rewrite the resolver file. With a custom port the
/// file must match [`render_resolver_conf`] exactly. Without one, a file carrying any
/// `port` line — from an earlier `dns_port` or added by hand — is rewritten without
/// it, which is how `darp config rm dns-port` takes effect; any other file, or none,
/// is left alone.
pub fn resolver_conf_needs_sync(current: Option<&str>, dns_port: Option<u16>) -> bool {
    match (dns_port, current) {
        (Some(_), Some(contents)) => contents != render_resolver_conf(dns_port),
        (Some(_), None) => true,
        (None, Some(contents)) => resolver_conf_port(contents).is_some(),
        (None, None) => false,
    }
}

/// `Ok` when one sudo step of writing `file` succeeded; otherwise an error naming the
/// file and the step, so a declined or failed sudo isn't reported as a write.
pub fn ensure_resolver_step(
    status: std::process::ExitStatus,
    file: &str,
    step: &str,
) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("could not write {file}: {step} failed ({status})"))
    }
}

pub struct OsIntegration<'a> {
    paths: &'a DarpPaths,
    resolver_file: &'static str,
}

impl<'a> OsIntegration<'a> {
    pub fn new(paths: &'a DarpPaths, _config: &Config, _engine_kind: &'a EngineKind) -> Self {
        // In your Python version this is hard-coded to /etc/resolver/test
        Self {
            paths,
            resolver_file: "/etc/resolver/test",
        }
    }

    pub fn init_resolver(&self, dns_port: Option<u16>) -> Result<()> {
        self.write_resolver(&render_resolver_conf(dns_port), Stdio::inherit())?;
        println!("\n{} created", self.resolver_file.green());
        Ok(())
    }

    /// Rewrite the resolver file (via sudo) when [`resolver_conf_needs_sync`] says it
    /// no longer matches `dns_port`; otherwise leave it, and sudo, alone.
    pub fn sync_resolver(&self, dns_port: Option<u16>) -> Result<()> {
        let current = fs::read_to_string(self.resolver_file).ok();
        if !resolver_conf_needs_sync(current.as_deref(), dns_port) {
            return Ok(());
        }

        self.write_resolver(&render_resolver_conf(dns_port), Stdio::null())?;
        let port = dns_port.unwrap_or(config::DEFAULT_DNS_PORT);
        println!("{} now points at port {}", self.resolver_file.green(), port);
        Ok(())
    }

    /// Write `contents` to the resolver file via `sudo tee`; `tee_stdout` decides
    /// whether tee echoes them to the terminal. Fails, naming the file, when either
    /// sudo step does.
    fn write_resolver(&self, contents: &str, tee_stdout: Stdio) -> Result<()> {
        #[cfg(unix)]
        {
            let file = self.resolver_file;
            let status = Command::new("sudo")
                .arg("mkdir")
                .arg("-p")
                .arg("/etc/resolver")
                .status()?;
            ensure_resolver_step(status, file, "sudo mkdir -p /etc/resolver")?;

            let mut child = Command::new("sudo")
                .arg("tee")
                .arg(file)
                .stdin(Stdio::piped())
                .stdout(tee_stdout)
                .spawn()?;

            {
                let stdin = child
                    .stdin
                    .as_mut()
                    .ok_or_else(|| anyhow!("Could not open stdin"))?;
                stdin
                    .write_all(contents.as_bytes())
                    .map_err(|e| anyhow!("could not write {file}: {e}"))?;
            }

            ensure_resolver_step(child.wait()?, file, "sudo tee")
        }

        #[cfg(not(unix))]
        {
            let _ = (contents, tee_stdout);
            Err(anyhow!(
                "resolver initialization is currently implemented only on Unix-like systems"
            ))
        }
    }

    pub fn ensure_dnsmasq_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.paths.dnsmasq_dir)?;
        Ok(())
    }

    pub fn copy_nginx_conf(&self, overrides: Option<&BTreeMap<String, String>>) -> Result<()> {
        // Render embedded nginx.conf to $DARP_ROOT/nginx.conf
        let rendered = render_nginx_conf(overrides)?;
        if let Some(parent) = self.paths.nginx_conf_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&self.paths.nginx_conf_path, rendered)?;
        println!(
            "nginx.conf written to {}",
            self.paths.nginx_conf_path.display()
        );

        Ok(())
    }

    pub fn write_test_conf(&self) -> Result<()> {
        let test_conf = self.paths.dnsmasq_dir.join("test.conf");
        let mut file = fs::File::create(&test_conf)?;
        file.write_all(b"address=/.test/127.0.0.1\n")?;
        println!("{} created", test_conf.display().to_string().green());
        Ok(())
    }

    pub fn sync_system_hosts(&self, hosts_container_lines: &[String]) -> Result<()> {
        #[cfg(unix)]
        {
            let hosts_path = "/etc/hosts";

            let output = Command::new("sudo")
                .arg("cat")
                .arg(hosts_path)
                .output()
                .map_err(|e| anyhow!("unable to read {} via sudo: {}", hosts_path, e))?;

            let current = String::from_utf8_lossy(&output.stdout).into_owned();
            let new_contents = build_hosts_content(&current, hosts_container_lines);

            let mut child = Command::new("sudo")
                .arg("tee")
                .arg(hosts_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            {
                let stdin = child
                    .stdin
                    .as_mut()
                    .ok_or_else(|| anyhow!("Could not open stdin"))?;
                stdin.write_all(new_contents.as_bytes())?;
            }

            child.wait()?;
            println!(
                "{} updated with Darp URL mappings (127.0.0.1).",
                hosts_path.green()
            );
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(anyhow!(
                "/etc/hosts sync is only implemented for Unix-like systems right now"
            ))
        }
    }

    pub fn sync_windows_hosts(&self, hosts_container_lines: &[String]) -> Result<()> {
        let hosts_path = "/mnt/c/Windows/System32/drivers/etc/hosts";

        let current = fs::read_to_string(hosts_path).map_err(|e| {
            anyhow!(
                "Unable to read Windows hosts file at {}: {}. Ensure WSL is running as Administrator.",
                hosts_path,
                e
            )
        })?;

        let new_contents = build_hosts_content(&current, hosts_container_lines);

        fs::write(hosts_path, new_contents.as_bytes()).map_err(|e| {
            anyhow!(
                "Unable to write Windows hosts file at {}: {}. Ensure WSL is running as Administrator.",
                hosts_path,
                e
            )
        })?;

        println!(
            "{} updated with Darp URL mappings (127.0.0.1).",
            hosts_path.green()
        );
        Ok(())
    }

    pub fn uninstall(&self) -> Result<()> {
        #[cfg(unix)]
        {
            // Remove resolver file; leave Darp config directory intact.
            Command::new("sudo")
                .arg("rm")
                .arg("-f")
                .arg(self.resolver_file)
                .status()
                .map_err(|e| anyhow!("failed to remove resolver file: {}", e))?;
            println!("{} removed", self.resolver_file.green());
            println!(
                "Darp resolver removed. Config and data under $DARP_ROOT were left untouched."
            );
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(anyhow!(
                "Uninstall is currently implemented only on Unix-like systems"
            ))
        }
    }
}
