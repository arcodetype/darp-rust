//! Argument parsing, argv assembly and exit-code mapping for `darp exec`.
//!
//! Actually running a command needs a container engine, so the argv construction is
//! kept separate from the process spawn and asserted on directly here. These tests
//! run under a plain `cargo test` with no engine present.

use clap::Parser;
use darp::cli::{Cli, Command};
use darp::commands::{
    EXEC_ARGV0, EXEC_SCRIPT, container_exec_argv, exec_wrapper_argv, exit_code_for,
};

// ---------------------------------------------------------------------------
// Argument parsing — where darp's own flags stop and the user's command begins
// ---------------------------------------------------------------------------

/// Parse `darp exec ...` and return (environment, dry_run, image, cmd).
fn parse_exec(args: &[&str]) -> (Option<String>, bool, Option<String>, Vec<String>) {
    let mut argv = vec!["darp", "exec"];
    argv.extend_from_slice(args);

    match Cli::try_parse_from(argv).expect("should parse").command {
        Some(Command::Exec {
            environment,
            dry_run,
            image,
            cmd,
        }) => (environment, dry_run, image, cmd),
        other => panic!("expected Command::Exec, got {other:?}"),
    }
}

fn parse_cmd(args: &[&str]) -> Vec<String> {
    parse_exec(args).3
}

#[test]
fn darps_own_flags_are_consumed_before_the_command() {
    let (environment, dry_run, image, cmd) = parse_exec(&[
        "-e",
        "prod",
        "--dry-run",
        "--image",
        "alpine",
        "php",
        "artisan",
        "migrate",
    ]);

    assert_eq!(environment.as_deref(), Some("prod"));
    assert!(dry_run);
    assert_eq!(image.as_deref(), Some("alpine"));
    assert_eq!(cmd, ["php", "artisan", "migrate"]);
}

#[test]
fn flags_after_the_command_belong_to_the_command() {
    // --force is the command's, not darp's.
    assert_eq!(
        parse_cmd(&["php", "artisan", "migrate", "--force"]),
        ["php", "artisan", "migrate", "--force"]
    );

    // Even a flag darp itself defines is passed through once the command has started.
    let (_, dry_run, _, cmd) = parse_exec(&["php", "artisan", "--dry-run"]);
    assert!(!dry_run, "--dry-run after the command is the command's");
    assert_eq!(cmd, ["php", "artisan", "--dry-run"]);
}

#[test]
fn double_dash_escapes_a_command_that_starts_with_a_flag() {
    assert_eq!(parse_cmd(&["--", "php", "-v"]), ["php", "-v"]);
}

#[test]
fn an_unknown_leading_flag_is_treated_as_the_command() {
    assert_eq!(parse_cmd(&["--force"]), ["--force"]);
}

#[test]
fn a_double_dash_inside_the_command_is_passed_through() {
    // `git diff -- path` must reach git intact.
    assert_eq!(
        parse_cmd(&["git", "diff", "--", "src/main.rs"]),
        ["git", "diff", "--", "src/main.rs"]
    );
}

#[test]
fn a_command_is_required() {
    assert!(Cli::try_parse_from(["darp", "exec"]).is_err());
    assert!(Cli::try_parse_from(["darp", "exec", "--dry-run"]).is_err());
}

// ---------------------------------------------------------------------------
// exec_wrapper_argv — the `sh -c <script> darp-exec <args...>` tail
// ---------------------------------------------------------------------------

#[test]
fn wrapper_wraps_argv_in_sh_with_positional_params() {
    let args = vec![
        "php".to_string(),
        "artisan".to_string(),
        "migrate".to_string(),
    ];
    assert_eq!(
        exec_wrapper_argv(&args),
        vec![
            "sh",
            "-c",
            r#"cd /app && exec "$@""#,
            "darp-exec",
            "php",
            "artisan",
            "migrate",
        ]
    );
}

#[test]
fn wrapper_script_cds_before_exec_and_fails_closed() {
    // `&&` rather than `;`: if /app is missing the command must not silently run in /.
    assert!(EXEC_SCRIPT.contains("cd /app &&"));
    assert!(EXEC_SCRIPT.contains(r#"exec "$@""#));
    assert_eq!(EXEC_ARGV0, "darp-exec");
}

/// The regression guard for the injection class of bug: shell metacharacters in a
/// single argument must stay in a single argument, never re-split by the inner sh.
#[test]
fn wrapper_never_resplits_arguments() {
    let args = vec![
        "php".to_string(),
        "-r".to_string(),
        r#"echo "hi"; $x; rm -rf /"#.to_string(),
    ];
    let argv = exec_wrapper_argv(&args);

    // 4 fixed elements (sh -c script $0) plus exactly 3 user arguments.
    assert_eq!(argv.len(), 7, "{argv:?}");
    assert_eq!(argv[6], r#"echo "hi"; $x; rm -rf /"#);
}

#[test]
fn wrapper_preserves_empty_and_whitespace_arguments() {
    let args = vec!["touch".to_string(), String::new(), "two words".to_string()];
    let argv = exec_wrapper_argv(&args);

    assert_eq!(argv.len(), 7, "{argv:?}");
    assert_eq!(argv[5], "");
    assert_eq!(argv[6], "two words");
}

#[test]
fn wrapper_passes_through_leading_hyphen_arguments() {
    let args = vec!["php".to_string(), "-v".to_string()];
    let argv = exec_wrapper_argv(&args);
    assert_eq!(&argv[4..], &["php", "-v"]);
}

// ---------------------------------------------------------------------------
// container_exec_argv — the full `<bin> exec ...` command
// ---------------------------------------------------------------------------

#[test]
fn exec_argv_allocates_a_tty_when_asked() {
    let args = vec!["sh".to_string()];
    let argv = container_exec_argv("docker", "darp_projects_myapp", true, &args);

    assert_eq!(
        &argv[..5],
        &["docker", "exec", "-i", "-t", "darp_projects_myapp"]
    );
}

#[test]
fn exec_argv_forwards_stdin_without_a_tty() {
    let args = vec!["cat".to_string()];
    let argv = container_exec_argv("docker", "darp_projects_myapp", false, &args);

    // stdin is always forwarded so piping works; the pty is what's conditional.
    assert_eq!(&argv[..4], &["docker", "exec", "-i", "darp_projects_myapp"]);
    assert!(!argv.contains(&"-t".to_string()), "{argv:?}");
}

#[test]
fn exec_argv_honours_the_engine_binary() {
    let args = vec!["true".to_string()];
    let argv = container_exec_argv("podman", "darp_a_b", false, &args);
    assert_eq!(argv[0], "podman");
}

#[test]
fn exec_argv_ends_with_the_shared_wrapper() {
    let args = vec!["php".to_string(), "artisan".to_string()];
    let argv = container_exec_argv("docker", "darp_a_b", true, &args);
    let wrapper = exec_wrapper_argv(&args);

    assert_eq!(&argv[argv.len() - wrapper.len()..], wrapper.as_slice());
}

// ---------------------------------------------------------------------------
// exit_code_for
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn exit_code_maps_normal_exits_verbatim() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    assert_eq!(exit_code_for(ExitStatus::from_raw(0)), 0);
    assert_eq!(exit_code_for(ExitStatus::from_raw(7 << 8)), 7);
    assert_eq!(exit_code_for(ExitStatus::from_raw(127 << 8)), 127);
}

#[cfg(unix)]
#[test]
fn exit_code_maps_signals_to_128_plus_signal() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    // SIGINT (2) -> 130, the same convention every shell uses.
    assert_eq!(exit_code_for(ExitStatus::from_raw(2)), 130);
    // SIGKILL (9) -> 137
    assert_eq!(exit_code_for(ExitStatus::from_raw(9)), 137);
}
