use crate::config::{Config, DarpPaths};
use crate::engine::EngineKind;
use anyhow::{anyhow, Result};
use colored::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

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

    pub fn init_resolver(&self) -> Result<()> {
        #[cfg(unix)]
        {
            Command::new("sudo")
                .arg("mkdir")
                .arg("-p")
                .arg("/etc/resolver")
                .status()?;

            let mut child = Command::new("sudo")
                .arg("tee")
                .arg(self.resolver_file)
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .spawn()?;

            {
                let stdin = child.stdin.as_mut().ok_or_else(|| anyhow!("Could not open stdin"))?;
                stdin.write_all(b"nameserver 127.0.0.1\n")?;
            }

            child.wait()?;
            println!("\n{} created", self.resolver_file.green());
            Ok(())
        }

        #[cfg(not(unix))]
        {
            Err(anyhow!(
                "resolver initialization is currently implemented only on Unix-like systems"
            ))
        }
    }

    pub fn ensure_dnsmasq_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.paths.dnsmasq_dir)?;
        Ok(())
    }

    pub fn copy_nginx_conf(&self) -> Result<()> {
        // Mirrors: cp /usr/local/opt/darp/nginx.conf $DARP_ROOT
        let src = Path::new("/usr/local/opt/darp/nginx.conf");
        if !src.exists() {
            return Err(anyhow!(
                "Expected nginx.conf at /usr/local/opt/darp/nginx.conf not found"
            ));
        }
        fs::copy(src, &self.paths.nginx_conf_path)?;
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
            let header = "# --- DARP HOSTS START ---";
            let footer = "# --- DARP HOSTS END ---";
            let hosts_path = "/etc/hosts";

            let output = Command::new("sudo")
                .arg("cat")
                .arg(hosts_path)
                .output()
                .map_err(|e| anyhow!("unable to read {} via sudo: {}", hosts_path, e))?;

            let mut current = String::from_utf8_lossy(&output.stdout).into_owned();
            current = current.replace("\r\n", "\n");

            let start = current.find(header);
            let before: String;
            let after: String;

            if let Some(s) = start {
                if let Some(e) = current[s..].find(footer) {
                    let end = s + e + footer.len();
                    before = current[..s].trim_end_matches('\n').to_string();
                    after = current[end..].trim_start_matches('\n').to_string();
                } else {
                    before = current.trim_end_matches('\n').to_string();
                    after = String::new();
                }
            } else {
                before = current.trim_end_matches('\n').to_string();
                after = String::new();
            }

            // Build new block
            let mut block = String::new();
            block.push_str(header);
            block.push('\n');
            for line in hosts_container_lines {
                let parts: Vec<_> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let host = parts[1];
                    block.push_str(&format!("127.0.0.1   {}\n", host));
                }
            }
            block.push_str(footer);
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

            let mut child = Command::new("sudo")
                .arg("tee")
                .arg(hosts_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;

            {
                let stdin = child.stdin.as_mut().ok_or_else(|| anyhow!("Could not open stdin"))?;
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
            println!("Darp resolver removed. Config and data under $DARP_ROOT were left untouched.");
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
