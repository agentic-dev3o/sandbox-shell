use std::path::PathBuf;
use sx::config::schema::{
    Config, FilesystemConfig, NetworkMode, SandboxConfig, ShellConfig,
};
use sx::config::global::load_global_config;
use sx::config::project::load_project_config;
use sx::config::merge::merge_configs;
use sx::utils::paths::expand_path;
use tempfile::TempDir;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.sandbox.default_network, NetworkMode::Offline);
    assert!(config.sandbox.default_profiles.contains(&"base".to_string()));
}

#[test]
fn test_parse_minimal_config() {
    let toml_str = r#"
[sandbox]
default_network = "offline"
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Offline);
}

#[test]
fn test_parse_full_config() {
    let toml_str = r#"
[sandbox]
default_network = "online"
default_profiles = ["base", "node"]
shell = "/bin/zsh"
prompt_indicator = true

[filesystem]
allow_read = ["~/.gitconfig", "~/.cargo"]
deny_read = ["~/.ssh", "~/.aws"]
allow_write = ["~/.npm/_cacache"]

[shell]
pass_env = ["TERM", "PATH"]
deny_env = ["AWS_*", "GITHUB_TOKEN"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Online);
    assert_eq!(config.sandbox.default_profiles, vec!["base", "node"]);
    assert_eq!(config.sandbox.shell, Some("/bin/zsh".to_string()));
    assert!(config.filesystem.allow_read.contains(&"~/.gitconfig".to_string()));
    assert!(config.filesystem.deny_read.contains(&"~/.ssh".to_string()));
    assert!(config.shell.pass_env.contains(&"TERM".to_string()));
}

#[test]
fn test_network_mode_in_config() {
    // Test offline
    let config: Config = toml::from_str(r#"
[sandbox]
default_network = "offline"
"#).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Offline);

    // Test online
    let config: Config = toml::from_str(r#"
[sandbox]
default_network = "online"
"#).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Online);

    // Test localhost
    let config: Config = toml::from_str(r#"
[sandbox]
default_network = "localhost"
"#).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Localhost);
}

#[test]
fn test_expand_tilde_path() {
    let home = dirs::home_dir().unwrap();
    let expanded = expand_path("~/.ssh");
    assert_eq!(expanded, home.join(".ssh"));
}

#[test]
fn test_expand_absolute_path() {
    let expanded = expand_path("/usr/bin");
    assert_eq!(expanded, PathBuf::from("/usr/bin"));
}

#[test]
fn test_expand_relative_path() {
    let expanded = expand_path("./foo/bar");
    assert!(expanded.ends_with("foo/bar"));
}

#[test]
fn test_load_global_config_missing_file() {
    let result = load_global_config(Some(&PathBuf::from("/nonexistent/config.toml")));
    // Should return default config when file doesn't exist
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Offline);
}

#[test]
fn test_load_global_config_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    std::fs::write(&config_path, r#"
[sandbox]
default_network = "localhost"
default_profiles = ["base", "python"]
"#).unwrap();

    let config = load_global_config(Some(&config_path)).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Localhost);
    assert!(config.sandbox.default_profiles.contains(&"python".to_string()));
}

#[test]
fn test_load_project_config_missing_file() {
    let temp_dir = TempDir::new().unwrap();
    let result = load_project_config(temp_dir.path());
    // Should return None when .sandbox.toml doesn't exist
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_load_project_config_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join(".sandbox.toml");

    std::fs::write(&config_path, r#"
[sandbox]
profiles = ["node", "claude"]
network = "online"

[filesystem]
allow_read = ["~/.claude"]
"#).unwrap();

    let result = load_project_config(temp_dir.path()).unwrap();
    assert!(result.is_some());
    let config = result.unwrap();
    assert!(config.sandbox.profiles.contains(&"claude".to_string()));
}

#[test]
fn test_merge_configs_project_overrides_global() {
    let global = Config {
        sandbox: SandboxConfig {
            default_network: NetworkMode::Offline,
            default_profiles: vec!["base".to_string()],
            ..Default::default()
        },
        filesystem: FilesystemConfig {
            allow_read: vec!["~/.gitconfig".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let project = Config {
        sandbox: SandboxConfig {
            default_network: NetworkMode::Online,
            profiles: vec!["node".to_string()],
            ..Default::default()
        },
        filesystem: FilesystemConfig {
            allow_read: vec!["~/.claude".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let merged = merge_configs(&global, &project);

    // Project network mode overrides global
    assert_eq!(merged.sandbox.default_network, NetworkMode::Online);
    // Filesystem allows are merged
    assert!(merged.filesystem.allow_read.contains(&"~/.gitconfig".to_string()));
    assert!(merged.filesystem.allow_read.contains(&"~/.claude".to_string()));
}

#[test]
fn test_merge_configs_deny_takes_precedence() {
    let global = Config {
        filesystem: FilesystemConfig {
            allow_read: vec!["~/.cargo".to_string()],
            deny_read: vec!["~/.ssh".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let project = Config {
        filesystem: FilesystemConfig {
            deny_read: vec!["~/.cargo".to_string()], // Project denies what global allows
            ..Default::default()
        },
        ..Default::default()
    };

    let merged = merge_configs(&global, &project);

    // Deny lists are merged
    assert!(merged.filesystem.deny_read.contains(&"~/.ssh".to_string()));
    assert!(merged.filesystem.deny_read.contains(&"~/.cargo".to_string()));
}

#[test]
fn test_merge_configs_shell_env() {
    let global = Config {
        shell: ShellConfig {
            pass_env: vec!["TERM".to_string(), "PATH".to_string()],
            deny_env: vec!["AWS_*".to_string()],
            set_env: [("SANDBOX_MODE".to_string(), "active".to_string())].into_iter().collect(),
        },
        ..Default::default()
    };

    let project = Config {
        shell: ShellConfig {
            pass_env: vec!["ANTHROPIC_API_KEY".to_string()],
            deny_env: vec![],
            set_env: Default::default(),
        },
        ..Default::default()
    };

    let merged = merge_configs(&global, &project);

    assert!(merged.shell.pass_env.contains(&"TERM".to_string()));
    assert!(merged.shell.pass_env.contains(&"ANTHROPIC_API_KEY".to_string()));
    assert!(merged.shell.deny_env.contains(&"AWS_*".to_string()));
    assert_eq!(merged.shell.set_env.get("SANDBOX_MODE"), Some(&"active".to_string()));
}
