mod completions;
mod config_cmds;
mod deploy;
mod doctor;
mod run;

pub use completions::{install_shell_completions, uninstall_shell_completions};
pub use config_cmds::{cmd_add, cmd_pull, cmd_rm, cmd_set, cmd_show, cmd_urls};
pub use deploy::{
    HostnameSource, PlannedService, UnresolvedAlias, build_container_hosts, build_host_proxy_vhost,
    build_hosts_lines, build_portmap, build_vhost_container_conf, cmd_deploy,
    collect_unresolved_aliases, detect_hostname_collisions, plan_deployment,
    render_unresolved_alias_warning, validate_nginx_directives, validate_plan_aliases,
};
pub use doctor::{cmd_check_image, cmd_doctor};
pub use run::{
    EXEC_ARGV0, EXEC_SCRIPT, cmd_exec, cmd_serve, cmd_shell, container_exec_argv,
    exec_wrapper_argv, exit_code_for,
};
