use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
