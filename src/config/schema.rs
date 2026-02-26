use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Setuid/setgid binary execution policy
///
/// Controls whether sandboxed processes can execute setuid binaries like `/bin/ps`.
/// Default is deny (info disclosure risk: `ps` can expose command-line args with API keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecSugid {
    /// Allow all (`true`) or deny all (`false`)
    Allow(bool),
    /// Allow only specific binary paths
    Paths(Vec<String>),
}

impl Default for ExecSugid {
    fn default() -> Self {
        ExecSugid::Allow(false)
    }
}

impl ExecSugid {
    /// Returns true if this is the default deny-all policy
    pub fn is_default(&self) -> bool {
        matches!(self, ExecSugid::Allow(false))
    }
}

/// Network access mode for the sandbox
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    /// Block all network access (default)
    #[default]
    Offline,
    /// Allow all network access
    Online,
    /// Allow localhost only
    Localhost,
}

/// Main configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub sandbox: SandboxConfig,
    pub filesystem: FilesystemConfig,
    pub shell: ShellConfig,
    pub profiles: ProfilesConfig,
}

/// Sandbox-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    /// Default network mode
    pub default_network: NetworkMode,
    /// Always include these profiles
    pub default_profiles: Vec<String>,
    /// Shell to use inside sandbox
    pub shell: Option<String>,
    /// Show sandbox indicator in shell prompt
    pub prompt_indicator: bool,
    /// Log file for sandbox violations
    pub log_file: Option<String>,
    /// Inherit from global config (project config only)
    pub inherit_global: bool,
    /// Include base profile (set to false for full custom control)
    pub inherit_base: bool,
    /// Profiles to use for this project (project config only)
    pub profiles: Vec<String>,
    /// Default network mode for this project (project config only)
    pub network: Option<NetworkMode>,
    /// Allow execution of setuid/setgid binaries (default: deny all)
    pub allow_exec_sugid: ExecSugid,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            default_network: NetworkMode::Offline,
            default_profiles: vec!["base".to_string()],
            shell: None,
            prompt_indicator: true,
            log_file: None,
            inherit_global: true,
            inherit_base: true,
            profiles: Vec::new(),
            network: None,
            allow_exec_sugid: ExecSugid::default(),
        }
    }
}

/// Filesystem access configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesystemConfig {
    /// Paths to always allow reading
    pub allow_read: Vec<String>,
    /// Paths to always deny reading (override allows)
    pub deny_read: Vec<String>,
    /// Paths to always allow writing (beyond project dir)
    pub allow_write: Vec<String>,
    /// Paths to allow directory listing only (readdir), not file contents.
    /// Uses Seatbelt `literal` filter - only the exact directory is listable,
    /// not its children. Useful for runtimes like Bun that need to scan
    /// parent directories during module resolution.
    pub allow_list_dirs: Vec<String>,
}

/// Shell environment configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShellConfig {
    /// Environment variables to pass through to sandbox
    pub pass_env: Vec<String>,
    /// Environment variables to NEVER pass (secrets)
    pub deny_env: Vec<String>,
    /// Environment variables to set inside the sandbox
    #[serde(default)]
    pub set_env: HashMap<String, String>,
}

/// Profile detection configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfilesConfig {
    /// Auto-detect project type and apply profiles
    pub auto_detect: bool,
    /// Profile detection rules
    #[serde(default)]
    pub detect: HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_sugid_default_is_deny() {
        assert_eq!(ExecSugid::default(), ExecSugid::Allow(false));
        assert!(ExecSugid::default().is_default());
    }

    #[test]
    fn test_exec_sugid_is_default() {
        assert!(ExecSugid::Allow(false).is_default());
        assert!(!ExecSugid::Allow(true).is_default());
        assert!(!ExecSugid::Paths(vec!["/bin/ps".into()]).is_default());
    }

    #[derive(Deserialize)]
    struct Wrapper {
        value: ExecSugid,
    }

    fn parse_exec_sugid(toml_str: &str) -> ExecSugid {
        let w: Wrapper = toml::from_str(toml_str).unwrap();
        w.value
    }

    #[test]
    fn test_exec_sugid_deserialize_false() {
        assert_eq!(parse_exec_sugid("value = false"), ExecSugid::Allow(false));
    }

    #[test]
    fn test_exec_sugid_deserialize_true() {
        assert_eq!(parse_exec_sugid("value = true"), ExecSugid::Allow(true));
    }

    #[test]
    fn test_exec_sugid_deserialize_paths() {
        assert_eq!(
            parse_exec_sugid(r#"value = ["/bin/ps", "/usr/bin/newgrp"]"#),
            ExecSugid::Paths(vec!["/bin/ps".into(), "/usr/bin/newgrp".into()])
        );
    }

    #[test]
    fn test_sandbox_config_with_exec_sugid_false() {
        let config: Config = toml::from_str(
            r#"
[sandbox]
allow_exec_sugid = false
"#,
        )
        .unwrap();
        assert_eq!(config.sandbox.allow_exec_sugid, ExecSugid::Allow(false));
    }

    #[test]
    fn test_sandbox_config_with_exec_sugid_true() {
        let config: Config = toml::from_str(
            r#"
[sandbox]
allow_exec_sugid = true
"#,
        )
        .unwrap();
        assert_eq!(config.sandbox.allow_exec_sugid, ExecSugid::Allow(true));
    }

    #[test]
    fn test_sandbox_config_with_exec_sugid_paths() {
        let config: Config = toml::from_str(
            r#"
[sandbox]
allow_exec_sugid = ["/bin/ps"]
"#,
        )
        .unwrap();
        assert_eq!(
            config.sandbox.allow_exec_sugid,
            ExecSugid::Paths(vec!["/bin/ps".into()])
        );
    }

    #[test]
    fn test_sandbox_config_default_omits_exec_sugid() {
        let config: Config = toml::from_str(
            r#"
[sandbox]
default_network = "offline"
"#,
        )
        .unwrap();
        assert!(config.sandbox.allow_exec_sugid.is_default());
    }
}
