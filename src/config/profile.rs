use crate::config::schema::NetworkMode;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Profile struct for composable sandbox configurations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// Optional network mode override
    pub network_mode: Option<NetworkMode>,
    /// Filesystem configuration
    pub filesystem: ProfileFilesystem,
    /// Shell configuration
    pub shell: ProfileShell,
    /// Raw seatbelt rules (advanced)
    #[serde(default)]
    pub seatbelt: Option<ProfileSeatbelt>,
}

/// Profile filesystem configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileFilesystem {
    pub allow_read: Vec<String>,
    pub deny_read: Vec<String>,
    pub allow_write: Vec<String>,
}

/// Profile shell configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileShell {
    pub pass_env: Vec<String>,
    pub deny_env: Vec<String>,
}

/// Raw Seatbelt rules for advanced configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileSeatbelt {
    pub raw: Option<String>,
}

/// Built-in profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinProfile {
    Base,
    Online,
    Localhost,
    Node,
    Python,
    Rust,
    Go,
    Claude,
    Gpg,
}

impl BuiltinProfile {
    /// Get a builtin profile by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "base" => Some(Self::Base),
            "online" => Some(Self::Online),
            "localhost" => Some(Self::Localhost),
            "node" => Some(Self::Node),
            "python" => Some(Self::Python),
            "rust" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "claude" => Some(Self::Claude),
            "gpg" => Some(Self::Gpg),
            _ => None,
        }
    }

    /// Load the profile data from embedded TOML files
    pub fn load(&self) -> Profile {
        let toml_str = match self {
            Self::Base => include_str!("../../profiles/base.toml"),
            Self::Online => include_str!("../../profiles/online.toml"),
            Self::Localhost => include_str!("../../profiles/localhost.toml"),
            Self::Node => include_str!("../../profiles/node.toml"),
            Self::Python => include_str!("../../profiles/python.toml"),
            Self::Rust => include_str!("../../profiles/rust.toml"),
            Self::Go => include_str!("../../profiles/go.toml"),
            Self::Claude => include_str!("../../profiles/claude.toml"),
            Self::Gpg => include_str!("../../profiles/gpg.toml"),
        };
        toml::from_str(toml_str).expect("builtin profile TOML is invalid")
    }
}

/// Load a profile from a TOML file
pub fn load_profile(path: &Path) -> Result<Profile, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Load profiles by name, optionally searching in a custom directory
pub fn load_profiles(names: &[String], custom_dir: Option<&Path>) -> Vec<Profile> {
    names
        .iter()
        .filter_map(|name| {
            // First try builtin profiles
            if let Some(builtin) = BuiltinProfile::from_name(name) {
                return Some(builtin.load());
            }
            // Then try custom directory
            if let Some(dir) = custom_dir {
                let path = dir.join(format!("{}.toml", name));
                if path.exists() {
                    return load_profile(&path).ok();
                }
            }
            // Try global profile directory
            if let Some(config_dir) = dirs::config_dir() {
                let path = config_dir.join("sx/profiles").join(format!("{}.toml", name));
                if path.exists() {
                    return load_profile(&path).ok();
                }
            }
            None
        })
        .collect()
}

/// Compose multiple profiles into a single merged profile
pub fn compose_profiles(profiles: &[Profile]) -> Profile {
    let mut result = Profile::default();

    for profile in profiles {
        // Network mode: last one with a value wins
        if profile.network_mode.is_some() {
            result.network_mode = profile.network_mode;
        }

        // Filesystem: merge unique paths
        merge_unique(&mut result.filesystem.allow_read, &profile.filesystem.allow_read);
        merge_unique(&mut result.filesystem.deny_read, &profile.filesystem.deny_read);
        merge_unique(&mut result.filesystem.allow_write, &profile.filesystem.allow_write);

        // Shell: merge unique env vars
        merge_unique(&mut result.shell.pass_env, &profile.shell.pass_env);
        merge_unique(&mut result.shell.deny_env, &profile.shell.deny_env);
    }

    result
}

fn merge_unique(target: &mut Vec<String>, source: &[String]) {
    for item in source {
        if !target.contains(item) {
            target.push(item.clone());
        }
    }
}
