use sx::cli::args::{Args, NetworkMode};

#[test]
fn test_parse_no_args_defaults_to_interactive_shell() {
    let args = Args::try_parse_from(["sx"]).unwrap();
    assert!(args.command.is_none());
    assert!(args.profiles.is_empty());
    assert_eq!(args.network_mode(), NetworkMode::Offline);
}

#[test]
fn test_parse_online_profile() {
    let args = Args::try_parse_from(["sx", "online"]).unwrap();
    assert!(args.profiles.contains(&"online".to_string()));
}

#[test]
fn test_parse_multiple_profiles() {
    let args = Args::try_parse_from(["sx", "online", "node", "claude"]).unwrap();
    assert_eq!(args.profiles, vec!["online", "node", "claude"]);
}

#[test]
fn test_parse_command_after_double_dash() {
    let args = Args::try_parse_from(["sx", "--", "npm", "install"]).unwrap();
    assert_eq!(args.command, Some(vec!["npm".to_string(), "install".to_string()]));
}

#[test]
fn test_parse_profiles_and_command() {
    let args = Args::try_parse_from(["sx", "online", "node", "--", "npm", "install"]).unwrap();
    assert_eq!(args.profiles, vec!["online", "node"]);
    assert_eq!(args.command, Some(vec!["npm".to_string(), "install".to_string()]));
}

#[test]
fn test_verbose_flag() {
    let args = Args::try_parse_from(["sx", "-v"]).unwrap();
    assert!(args.verbose);
}

#[test]
fn test_debug_flag() {
    let args = Args::try_parse_from(["sx", "-d"]).unwrap();
    assert!(args.debug);
}

#[test]
fn test_dry_run_flag() {
    let args = Args::try_parse_from(["sx", "-n"]).unwrap();
    assert!(args.dry_run);
}

#[test]
fn test_explain_flag() {
    let args = Args::try_parse_from(["sx", "--explain"]).unwrap();
    assert!(args.explain);
}

#[test]
fn test_init_flag() {
    let args = Args::try_parse_from(["sx", "--init"]).unwrap();
    assert!(args.init);
}

#[test]
fn test_offline_network_mode() {
    let args = Args::try_parse_from(["sx", "--offline"]).unwrap();
    assert_eq!(args.network_mode(), NetworkMode::Offline);
}

#[test]
fn test_online_network_mode() {
    let args = Args::try_parse_from(["sx", "--online"]).unwrap();
    assert_eq!(args.network_mode(), NetworkMode::Online);
}

#[test]
fn test_localhost_network_mode() {
    let args = Args::try_parse_from(["sx", "--localhost"]).unwrap();
    assert_eq!(args.network_mode(), NetworkMode::Localhost);
}

#[test]
fn test_allow_domain() {
    let args = Args::try_parse_from(["sx", "--allow-domain", "github.com"]).unwrap();
    assert_eq!(args.allow_domains, vec!["github.com"]);
}

#[test]
fn test_multiple_allow_domains() {
    let args = Args::try_parse_from([
        "sx",
        "--allow-domain", "github.com",
        "--allow-domain", "npmjs.org",
    ]).unwrap();
    assert_eq!(args.allow_domains, vec!["github.com", "npmjs.org"]);
}

#[test]
fn test_allow_read_path() {
    let args = Args::try_parse_from(["sx", "--allow-read", "/tmp/foo"]).unwrap();
    assert_eq!(args.allow_read, vec!["/tmp/foo"]);
}

#[test]
fn test_allow_write_path() {
    let args = Args::try_parse_from(["sx", "--allow-write", "/tmp/bar"]).unwrap();
    assert_eq!(args.allow_write, vec!["/tmp/bar"]);
}

#[test]
fn test_deny_read_path() {
    let args = Args::try_parse_from(["sx", "--deny-read", "~/.ssh"]).unwrap();
    assert_eq!(args.deny_read, vec!["~/.ssh"]);
}

#[test]
fn test_config_path() {
    let args = Args::try_parse_from(["sx", "-c", "/path/to/config.toml"]).unwrap();
    assert_eq!(args.config, Some("/path/to/config.toml".into()));
}

#[test]
fn test_no_config_flag() {
    let args = Args::try_parse_from(["sx", "--no-config"]).unwrap();
    assert!(args.no_config);
}
