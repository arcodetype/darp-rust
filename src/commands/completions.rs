use clap::CommandFactory;
use clap_complete::{generate, shells};
use dirs::home_dir;
use std::fs;
use std::path::Path;

use crate::cli::Cli;

pub const RC_START_MARKER: &str = "# >>> darp completion start >>>";
const RC_END_MARKER: &str = "# <<< darp completion end <<<";

pub fn detect_shell() -> Option<&'static str> {
    if let Ok(shell_path) = std::env::var("SHELL") {
        if shell_path.ends_with("zsh") {
            Some("zsh")
        } else if shell_path.ends_with("bash") {
            Some("bash")
        } else if shell_path.ends_with("fish") {
            Some("fish")
        } else if shell_path.ends_with("pwsh") || shell_path.ends_with("powershell") {
            Some("powershell")
        } else if shell_path.ends_with("elvish") {
            Some("elvish")
        } else {
            None
        }
    } else {
        None
    }
}

fn ensure_rc_block(rc_path: &Path, body: &str) -> anyhow::Result<()> {
    let contents = fs::read_to_string(rc_path).unwrap_or_default();

    if contents.contains(RC_START_MARKER) {
        return Ok(());
    }

    let mut new_contents = contents;
    if !new_contents.is_empty() && !new_contents.ends_with('\n') {
        new_contents.push('\n');
    }

    new_contents.push_str(RC_START_MARKER);
    new_contents.push('\n');
    new_contents.push_str(body);
    if !body.ends_with('\n') {
        new_contents.push('\n');
    }
    new_contents.push_str(RC_END_MARKER);
    new_contents.push('\n');

    if let Some(parent) = rc_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(rc_path, new_contents)?;
    Ok(())
}

fn remove_rc_block(rc_path: &Path) -> anyhow::Result<()> {
    let contents = match fs::read_to_string(rc_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };

    let start = if let Some(s) = contents.find(RC_START_MARKER) {
        s
    } else {
        return Ok(());
    };

    // Find end after start
    let after_start = &contents[start..];
    let end_rel = if let Some(e) = after_start.find(RC_END_MARKER) {
        e + RC_END_MARKER.len()
    } else {
        contents.len() - start
    };
    let end = start + end_rel;

    let mut new_contents = String::new();
    new_contents.push_str(contents[..start].trim_end_matches('\n'));
    if !new_contents.is_empty() {
        new_contents.push('\n');
    }
    let tail = contents[end..].trim_start_matches('\n');
    if !tail.is_empty() {
        if !new_contents.is_empty() {
            new_contents.push('\n');
        }
        new_contents.push_str(tail);
        new_contents.push('\n');
    }

    fs::write(rc_path, new_contents)?;
    Ok(())
}

struct ShellCompletionConfig {
    completion_file: &'static str,
    rc: Option<(&'static str, &'static str)>,
    generate: fn(cmd: &mut clap::Command, name: String, file: &mut fs::File),
}

fn gen_bash(cmd: &mut clap::Command, name: String, file: &mut fs::File) {
    generate(shells::Bash, cmd, name, file);
}
fn gen_zsh(cmd: &mut clap::Command, name: String, file: &mut fs::File) {
    generate(shells::Zsh, cmd, name, file);
}
fn gen_fish(cmd: &mut clap::Command, name: String, file: &mut fs::File) {
    generate(shells::Fish, cmd, name, file);
}

fn shell_completion_config(shell: &str) -> Option<ShellCompletionConfig> {
    match shell {
        "bash" => Some(ShellCompletionConfig {
            completion_file: ".local/share/bash-completion/completions/darp",
            rc: Some((
                ".bashrc",
                r#"if command -v darp >/dev/null 2>&1; then
  source "${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions/darp"
fi"#,
            )),
            generate: gen_bash,
        }),
        "zsh" => Some(ShellCompletionConfig {
            completion_file: ".zfunc/_darp",
            rc: Some((
                ".zshrc",
                r#"if command -v darp >/dev/null 2>&1; then
  fpath+=("$HOME/.zfunc")
  autoload -Uz compinit
  compinit
fi"#,
            )),
            generate: gen_zsh,
        }),
        "fish" => Some(ShellCompletionConfig {
            completion_file: ".config/fish/completions/darp.fish",
            rc: None,
            generate: gen_fish,
        }),
        _ => None,
    }
}

pub fn install_shell_completions() -> anyhow::Result<()> {
    let Some(shell) = detect_shell() else {
        println!("Could not detect shell from $SHELL; skipping shell completion install.");
        return Ok(());
    };

    let Some(home) = home_dir() else {
        println!("Could not determine home directory; skipping shell completion install.");
        return Ok(());
    };

    let Some(cfg) = shell_completion_config(shell) else {
        println!(
            "{} completion installation is not yet automated; skipping.",
            shell
        );
        return Ok(());
    };

    let path = home.join(cfg.completion_file);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = fs::File::create(&path)?;
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    (cfg.generate)(&mut cmd, name, &mut file);
    println!("Installed {} completions to {}", shell, path.display());

    if let Some((rc_rel, body)) = cfg.rc {
        let rc_path = home.join(rc_rel);
        ensure_rc_block(&rc_path, body)?;
        println!("Updated {} with darp completion block", rc_path.display());
    }

    Ok(())
}

pub fn uninstall_shell_completions() -> anyhow::Result<()> {
    let Some(shell) = detect_shell() else {
        println!("Could not detect shell from $SHELL; skipping shell completion removal.");
        return Ok(());
    };

    let Some(home) = home_dir() else {
        println!("Could not determine home directory; skipping shell completion removal.");
        return Ok(());
    };

    let Some(cfg) = shell_completion_config(shell) else {
        println!(
            "{} completion removal is not yet automated; skipping.",
            shell
        );
        return Ok(());
    };

    let path = home.join(cfg.completion_file);
    match fs::remove_file(&path) {
        Ok(()) => println!("Removed {} completions at {}", shell, path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    if let Some((rc_rel, _)) = cfg.rc {
        remove_rc_block(&home.join(rc_rel))?;
    }

    Ok(())
}
