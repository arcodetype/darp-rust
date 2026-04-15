use colored::*;
use dirs::home_dir;
use std::fs;
use std::path::Path;

use crate::commands::completions::{RC_START_MARKER, detect_shell};
use crate::config::{self, Config, DarpPaths, ResolvedSettings};
use crate::engine::Engine;

enum CheckResult {
    Ok(String),
    Warn(String),
    Fail(String),
}

struct DoctorSection {
    name: String,
    results: Vec<CheckResult>,
}

impl DoctorSection {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            results: Vec::new(),
        }
    }

    fn ok(&mut self, msg: &str) {
        self.results.push(CheckResult::Ok(msg.to_string()));
    }

    fn warn(&mut self, msg: &str) {
        self.results.push(CheckResult::Warn(msg.to_string()));
    }

    fn fail(&mut self, msg: &str) {
        self.results.push(CheckResult::Fail(msg.to_string()));
    }

    fn passed(&self) -> bool {
        self.results.iter().all(|r| matches!(r, CheckResult::Ok(_)))
    }

    fn print(&self) {
        if self.passed() {
            println!("[{}] {}", "✓".green(), self.name);
        } else {
            println!("[{}] {}", "!".yellow(), self.name);
        }
        for r in &self.results {
            match r {
                CheckResult::Ok(msg) => println!("    {} {}", "•".green(), msg),
                CheckResult::Warn(msg) => println!("    {} {}", "!".yellow(), msg),
                CheckResult::Fail(msg) => println!("    {} {}", "✗".red(), msg),
            }
        }
    }
}

pub fn cmd_doctor(paths: &DarpPaths, config: &Config, engine: &Engine) -> anyhow::Result<()> {
    println!("Darp Doctor");

    let mut issue_count = 0u32;

    // 1. Darp root
    {
        let mut s = DoctorSection::new(&format!("Darp root ({})", paths._darp_root.display()));

        if paths._darp_root.is_dir() {
            if paths.config_path.is_file() {
                match fs::read_to_string(&paths.config_path) {
                    Ok(contents) => {
                        if serde_json::from_str::<serde_json::Value>(&contents).is_ok() {
                            s.ok("config.json exists and is valid JSON");
                        } else {
                            s.fail("config.json exists but is not valid JSON");
                        }
                    }
                    Err(_) => s.fail("config.json exists but cannot be read"),
                }
            } else {
                s.fail("config.json not found");
            }

            if paths.nginx_conf_path.is_file() {
                s.ok("nginx.conf exists");
            } else {
                s.warn("nginx.conf not found — run 'darp install'");
            }

            if paths.dnsmasq_dir.is_dir() {
                s.ok("dnsmasq.d/ exists");
            } else {
                s.warn("dnsmasq.d/ not found — run 'darp install'");
            }

            let test_conf = paths.dnsmasq_dir.join("test.conf");
            if test_conf.is_file() {
                match fs::read_to_string(&test_conf) {
                    Ok(contents) if contents.contains("address=/.test/127.0.0.1") => {
                        s.ok("dnsmasq.d/test.conf has correct DNS rule");
                    }
                    Ok(_) => s.warn("dnsmasq.d/test.conf has unexpected content"),
                    Err(_) => s.fail("dnsmasq.d/test.conf cannot be read"),
                }
            } else if paths.dnsmasq_dir.is_dir() {
                s.warn("dnsmasq.d/test.conf not found — run 'darp install'");
            }
        } else {
            s.fail(&format!(
                "{} does not exist — run 'darp install'",
                paths._darp_root.display()
            ));
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 2. Container engine
    let engine_ready;
    {
        let mut s = DoctorSection::new("Container engine");

        match &config.engine {
            Some(e) if e == "docker" || e == "podman" => {
                s.ok(&format!("Engine configured: {}", e));
            }
            Some(e) => {
                s.fail(&format!(
                    "Engine set to '{}' — must be 'docker' or 'podman'",
                    e
                ));
            }
            None => {
                s.fail("No engine configured — run 'darp config set engine docker' or 'darp config set engine podman'");
            }
        }

        if engine.is_engine_installed() {
            s.ok(&format!(
                "{} binary found in PATH",
                engine.bin.unwrap_or("(none)")
            ));
        } else if engine.bin.is_some() {
            s.fail(&format!("{} binary not found in PATH", engine.bin.unwrap()));
        }

        engine_ready = engine.require_ready().is_ok();
        if engine_ready {
            s.ok(&format!("{} is running", engine.bin.unwrap_or("engine")));
        } else if engine.bin.is_some() {
            s.warn(&format!("{} is not running", engine.bin.unwrap()));
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 3. DNS resolver
    {
        let mut s = DoctorSection::new("DNS resolver");
        let resolver_path = Path::new("/etc/resolver/test");

        if resolver_path.exists() {
            s.ok("/etc/resolver/test exists");
            match fs::read_to_string(resolver_path) {
                Ok(contents) if contents.contains("nameserver 127.0.0.1") => {
                    s.ok("Contains: nameserver 127.0.0.1");
                }
                Ok(_) => {
                    s.warn("/etc/resolver/test has unexpected content");
                }
                Err(_) => {
                    s.warn("/etc/resolver/test cannot be read (may need sudo)");
                }
            }
        } else {
            s.warn("/etc/resolver/test not found — run 'darp install'");
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 4. Infrastructure containers
    {
        let mut s = DoctorSection::new("Infrastructure containers");

        if engine_ready {
            if engine.is_container_running("darp-reverse-proxy") {
                s.ok("darp-reverse-proxy is running");
            } else {
                s.warn("darp-reverse-proxy is not running — run 'darp deploy'");
            }

            if engine.is_container_running("darp-masq") {
                s.ok("darp-masq is running");
            } else {
                s.warn("darp-masq is not running — run 'darp deploy'");
            }
        } else {
            s.warn("Skipped — container engine is not running");
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 5. Shell completions
    {
        let shell = detect_shell();
        let label = match shell {
            Some(sh) => format!("Shell completions ({})", sh),
            None => "Shell completions".to_string(),
        };
        let mut s = DoctorSection::new(&label);

        if let Some(home) = home_dir() {
            match shell {
                Some("bash") => {
                    let comp = home.join(".local/share/bash-completion/completions/darp");
                    if comp.is_file() {
                        s.ok("Bash completion file exists");
                    } else {
                        s.warn("Bash completion file not found — run 'darp install'");
                    }
                    let rc = home.join(".bashrc");
                    if rc.is_file() {
                        if let Ok(contents) = fs::read_to_string(&rc) {
                            if contents.contains(RC_START_MARKER) {
                                s.ok(".bashrc contains darp completion block");
                            } else {
                                s.warn(
                                    ".bashrc missing darp completion block — run 'darp install'",
                                );
                            }
                        }
                    }
                }
                Some("zsh") => {
                    let comp = home.join(".zfunc/_darp");
                    if comp.is_file() {
                        s.ok("Zsh completion file exists");
                    } else {
                        s.warn("Zsh completion file not found — run 'darp install'");
                    }
                    let rc = home.join(".zshrc");
                    if rc.is_file() {
                        if let Ok(contents) = fs::read_to_string(&rc) {
                            if contents.contains(RC_START_MARKER) {
                                s.ok(".zshrc contains darp completion block");
                            } else {
                                s.warn(".zshrc missing darp completion block — run 'darp install'");
                            }
                        }
                    }
                }
                Some("fish") => {
                    let comp = home.join(".config/fish/completions/darp.fish");
                    if comp.is_file() {
                        s.ok("Fish completion file exists");
                    } else {
                        s.warn("Fish completion file not found — run 'darp install'");
                    }
                }
                Some(other) => {
                    s.warn(&format!(
                        "Shell '{}' — completions not automatically managed",
                        other
                    ));
                }
                None => {
                    s.warn("Could not detect shell from $SHELL");
                }
            }
        } else {
            s.warn("Could not determine home directory");
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 6. Domains
    {
        let mut s = DoctorSection::new("Domains");

        match &config.domains {
            Some(domains) if !domains.is_empty() => {
                for (name, domain) in domains {
                    match config::resolve_location(&domain.location) {
                        Ok(loc) => {
                            if loc.is_dir() {
                                let mut service_count = 0u32;
                                let group_names: std::collections::HashSet<String> = domain
                                    .groups
                                    .as_ref()
                                    .map(|g| {
                                        g.keys().filter(|k| k.as_str() != ".").cloned().collect()
                                    })
                                    .unwrap_or_default();

                                if let Ok(entries) = fs::read_dir(&loc) {
                                    for entry in entries.flatten() {
                                        if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                                            let entry_name =
                                                entry.file_name().to_string_lossy().to_string();
                                            if !group_names.contains(&entry_name) {
                                                service_count += 1;
                                            }
                                        }
                                    }
                                }
                                for gn in &group_names {
                                    let gp = loc.join(gn);
                                    if let Ok(entries) = fs::read_dir(&gp) {
                                        for entry in entries.flatten() {
                                            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                                                service_count += 1;
                                            }
                                        }
                                    } else {
                                        s.warn(&format!(
                                            "{} — group '{}' directory not found at {}",
                                            name,
                                            gn,
                                            gp.display()
                                        ));
                                    }
                                }

                                s.ok(&format!(
                                    "{} → {} ({} service(s) found)",
                                    name,
                                    loc.display(),
                                    service_count
                                ));

                                if let Some(ref env_name) = domain.default_environment {
                                    let env_exists = config
                                        .environments
                                        .as_ref()
                                        .is_some_and(|e| e.contains_key(env_name));
                                    if !env_exists {
                                        s.warn(&format!(
                                            "{} — default_environment '{}' does not exist",
                                            name, env_name
                                        ));
                                    }
                                }
                            } else {
                                s.fail(&format!(
                                    "{} — location does not exist: {}",
                                    name,
                                    loc.display()
                                ));
                            }
                        }
                        Err(e) => {
                            s.fail(&format!("{} — cannot resolve location: {}", name, e));
                        }
                    }
                }
            }
            _ => {
                s.warn("No domains configured — run 'darp config add domain <name> <path>'");
            }
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 7. Deploy state
    {
        let mut s = DoctorSection::new("Deploy state");

        if paths.portmap_path.is_file() {
            match fs::read_to_string(&paths.portmap_path) {
                Ok(contents) => {
                    if let Ok(portmap) = serde_json::from_str::<serde_json::Value>(&contents) {
                        let domain_count = portmap.as_object().map_or(0, |o| o.len());
                        let service_count: usize = portmap.as_object().map_or(0, |o| {
                            o.values()
                                .filter_map(|v| v.as_object())
                                .map(|m| m.len())
                                .sum()
                        });
                        s.ok(&format!(
                            "portmap.json valid ({} domain(s), {} service(s))",
                            domain_count, service_count
                        ));
                    } else {
                        s.fail("portmap.json is not valid JSON");
                    }
                }
                Err(_) => s.fail("portmap.json cannot be read"),
            }
        } else {
            s.warn("portmap.json not found — run 'darp deploy'");
        }

        if paths.vhost_container_conf.is_file() {
            match fs::metadata(&paths.vhost_container_conf) {
                Ok(meta) if meta.len() > 0 => {
                    s.ok("vhost_container.conf exists");
                }
                Ok(_) => {
                    s.warn("vhost_container.conf is empty — run 'darp deploy'");
                }
                Err(_) => s.fail("vhost_container.conf cannot be read"),
            }
        } else {
            s.warn("vhost_container.conf not found — run 'darp deploy'");
        }

        if paths.hosts_container_path.is_file() {
            s.ok("hosts_container exists");
        } else {
            s.warn("hosts_container not found — run 'darp deploy'");
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 8. Environments
    {
        let mut s = DoctorSection::new("Environments");

        match &config.environments {
            Some(envs) if !envs.is_empty() => {
                for (name, env) in envs {
                    let mut details = Vec::new();
                    if env.serve_command.is_some() {
                        details.push("serve_command");
                    }
                    if env.default_container_image.is_some() {
                        details.push("default_container_image");
                    }
                    if env.shell_command.is_some() {
                        details.push("shell_command");
                    }
                    if env.image_repository.is_some() {
                        details.push("image_repository");
                    }
                    if details.is_empty() {
                        s.warn(&format!("{} — no settings configured", name));
                    } else {
                        s.ok(&format!("{} — has {}", name, details.join(", ")));
                    }
                }
            }
            _ => {
                s.warn("No environments configured");
            }
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // 9. WSL
    if config.wsl.unwrap_or(false) {
        let mut s = DoctorSection::new("WSL");

        match std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "[bool](([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))",
            ])
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if stdout == "True" {
                    s.ok("WSL is running as Administrator");
                } else {
                    s.fail(
                        "WSL is NOT running as Administrator — required to write Windows hosts file",
                    );
                }
            }
            Err(_) => {
                s.fail("powershell.exe not found — are you running inside WSL?");
            }
        }

        let win_hosts = Path::new("/mnt/c/Windows/System32/drivers/etc/hosts");
        if win_hosts.exists() {
            match fs::OpenOptions::new().append(true).open(win_hosts) {
                Ok(_) => {
                    s.ok("/mnt/c/Windows/System32/drivers/etc/hosts is writable");
                }
                Err(_) => {
                    s.fail("/mnt/c/Windows/System32/drivers/etc/hosts is not writable — run WSL as Administrator");
                }
            }
        } else {
            s.fail(
                "/mnt/c/Windows/System32/drivers/etc/hosts not found — is the C: drive mounted?",
            );
        }

        if !config.urls_in_hosts.unwrap_or(false) {
            s.warn("urls_in_hosts is not enabled — WSL hosts sync requires it. Run 'darp config set urls-in-hosts true'");
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Summary
    println!();
    if issue_count == 0 {
        println!("{}", "No issues found.".green());
    } else {
        println!(
            "{}",
            format!(
                "{} section(s) with issues. Run the suggested commands to fix them.",
                issue_count
            )
            .yellow()
        );
    }

    Ok(())
}

pub fn cmd_check_image(
    image_cli: Option<String>,
    environment_cli: Option<String>,
    _paths: &DarpPaths,
    config: &Config,
    engine: &Engine,
) -> anyhow::Result<()> {
    engine.require_ready()?;

    let ctx = config.service_context_from_cwd(environment_cli);

    let resolved = ctx.as_ref().map(|c| {
        ResolvedSettings::resolve(
            c.domain_name.clone(),
            c.group_name.clone(),
            c.current_directory_name.clone(),
            c.environment_name.clone(),
            c.service,
            c.group,
            c.domain,
            c.environment,
        )
    });

    let image_name = if let Some(img) = image_cli {
        img
    } else {
        match resolved
            .as_ref()
            .and_then(|r| r.resolve_full_image_name(None))
        {
            Some(img) => img,
            None => {
                eprintln!(
                    "No image specified and none could be resolved from current directory.\n\
                     Usage: darp check-image <image>"
                );
                std::process::exit(1);
            }
        }
    };

    let serve_command = resolved.as_ref().and_then(|r| r.serve_command.as_deref());
    let shell_command = resolved.as_ref().and_then(|r| r.shell_command.as_deref());

    println!("Darp Image Check: {}\n", image_name.cyan());

    let mut issue_count = 0u32;

    let mut probe_lines = Vec::new();
    probe_lines.push(r#"command -v sh >/dev/null 2>&1 && echo "DARP_CHECK:sh:ok" || echo "DARP_CHECK:sh:missing""#.to_string());
    probe_lines.push(r#"command -v nginx >/dev/null 2>&1 && echo "DARP_CHECK:nginx:ok" || echo "DARP_CHECK:nginx:missing""#.to_string());

    if let Some(cmd) = serve_command {
        let binary = cmd.split_whitespace().next().unwrap_or(cmd);
        if !binary.is_empty() {
            probe_lines.push(format!(
                r#"command -v {bin} >/dev/null 2>&1 && echo "DARP_CHECK:serve_cmd:{bin}:ok" || echo "DARP_CHECK:serve_cmd:{bin}:missing""#,
                bin = binary
            ));
        }
    }

    if let Some(cmd) = shell_command {
        let binary = cmd.split_whitespace().next().unwrap_or(cmd);
        if !binary.is_empty() && binary != "sh" {
            probe_lines.push(format!(
                r#"command -v {bin} >/dev/null 2>&1 && echo "DARP_CHECK:shell_cmd:{bin}:ok" || echo "DARP_CHECK:shell_cmd:{bin}:missing""#,
                bin = binary
            ));
        }
    }

    let probe_script = probe_lines.join("; ");

    let bin = engine.bin.expect("engine bin not set");

    // Check image availability
    {
        let mut s = DoctorSection::new("Image");

        let inspect = std::process::Command::new(bin)
            .arg("image")
            .arg("inspect")
            .arg(&image_name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match inspect {
            Ok(status) if status.success() => {
                s.ok(&format!("{} found locally", image_name));
            }
            _ => {
                s.warn(&format!(
                    "{} not found locally — will attempt to pull",
                    image_name
                ));

                let pull = std::process::Command::new(bin)
                    .arg("pull")
                    .arg(&image_name)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::piped())
                    .status();

                match pull {
                    Ok(status) if status.success() => {
                        s.ok(&format!("{} pulled successfully", image_name));
                    }
                    _ => {
                        s.fail(&format!("{} could not be found or pulled", image_name));
                        s.print();
                        println!("\n{}", "Cannot continue — image is not available.".red());
                        return Ok(());
                    }
                }
            }
        }

        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Run diagnostic container
    let output = std::process::Command::new(bin)
        .arg("run")
        .arg("--rm")
        .arg(&image_name)
        .arg("sh")
        .arg("-c")
        .arg(&probe_script)
        .output();

    let probe_output = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let mut s = DoctorSection::new("Shell support");
            s.fail(&format!(
                "Container failed to start with 'sh' — image may not have a shell: {}",
                stderr.lines().next().unwrap_or("unknown error")
            ));
            issue_count += 1;
            s.print();
            println!(
                "\n{}",
                "Cannot continue — 'sh' is required for darp to function.".red()
            );
            println!(
                "\n{} section(s) with issues.",
                issue_count.to_string().yellow()
            );
            return Ok(());
        }
        Err(e) => {
            eprintln!("Failed to run diagnostic container: {}", e);
            return Ok(());
        }
    };

    // Parse results
    let mut results: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for line in probe_output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("DARP_CHECK:") {
            let is_ok = rest.ends_with(":ok");
            let key = if is_ok {
                rest.trim_end_matches(":ok")
            } else {
                rest.trim_end_matches(":missing")
            };
            results.insert(key.to_string(), is_ok);
        }
    }

    // Shell support
    {
        let mut s = DoctorSection::new("Shell support");
        if results.get("sh").copied().unwrap_or(false) {
            s.ok("sh is available (required)");
        } else {
            s.fail("sh not found — darp requires sh to run commands in containers");
        }
        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Nginx
    {
        let mut s = DoctorSection::new("Nginx (in-container reverse proxy)");
        if results.get("nginx").copied().unwrap_or(false) {
            s.ok("nginx is available");
        } else {
            s.warn("nginx not found — container-level reverse proxy will be skipped");
            s.warn("Install nginx in your Dockerfile for full .test domain support inside the container");
        }
        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Serve command
    let serve_binary = serve_command.and_then(|c| {
        let b = c.split_whitespace().next().unwrap_or(c);
        if b.is_empty() { None } else { Some((c, b)) }
    });
    if let Some((cmd, binary)) = serve_binary {
        let key = format!("serve_cmd:{}", binary);
        let mut s = DoctorSection::new(&format!("Serve command ({})", cmd));
        if results.get(&key).copied().unwrap_or(false) {
            s.ok(&format!("'{}' is available", binary));
        } else {
            s.fail(&format!(
                "'{}' not found in image — 'darp serve' will fail",
                binary
            ));
        }
        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    } else {
        let mut s = DoctorSection::new("Serve command");
        s.warn("No serve_command configured — skipping check");
        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Shell command
    {
        let effective_shell = shell_command.unwrap_or("sh");
        let binary = effective_shell
            .split_whitespace()
            .next()
            .unwrap_or(effective_shell);
        let mut s = DoctorSection::new(&format!("Shell command ({})", effective_shell));
        if binary == "sh" {
            if results.get("sh").copied().unwrap_or(false) {
                s.ok(&format!("'{}' is available", binary));
            } else {
                s.fail(&format!("'{}' not found", binary));
            }
        } else {
            let key = format!("shell_cmd:{}", binary);
            if results.get(&key).copied().unwrap_or(false) {
                s.ok(&format!("'{}' is available", binary));
            } else {
                s.fail(&format!(
                    "'{}' not found in image — 'darp shell' will fall back to sh",
                    binary
                ));
            }
        }
        if !s.passed() {
            issue_count += 1;
        }
        s.print();
    }

    // Summary
    println!();
    if issue_count == 0 {
        println!("{}", "Image is fully compatible with darp.".green());
    } else {
        println!(
            "{}",
            format!("{} section(s) with issues.", issue_count).yellow()
        );
    }

    Ok(())
}
