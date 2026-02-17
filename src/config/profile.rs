use crate::config::schema::NetworkMode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

/// Error type for profile loading
#[derive(Debug)]
pub enum ProfileError {
    /// IO error reading profile file
    Io(std::io::Error),
    /// TOML parsing error
    Parse(toml::de::Error),
    /// Built-in profile has invalid TOML (should never happen)
    InvalidBuiltin {
        name: &'static str,
        error: toml::de::Error,
    },
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfileError::Io(e) => write!(f, "IO error: {}", e),
            ProfileError::Parse(e) => write!(f, "TOML parse error: {}", e),
            ProfileError::InvalidBuiltin { name, error } => {
                write!(f, "Built-in profile '{}' is invalid: {}", name, error)
            }
        }
    }
}

impl std::error::Error for ProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProfileError::Io(e) => Some(e),
            ProfileError::Parse(e) => Some(e),
            ProfileError::InvalidBuiltin { error, .. } => Some(error),
        }
    }
}

impl From<std::io::Error> for ProfileError {
    fn from(e: std::io::Error) -> Self {
        ProfileError::Io(e)
    }
}

impl From<toml::de::Error> for ProfileError {
    fn from(e: toml::de::Error) -> Self {
        ProfileError::Parse(e)
    }
}

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
    /// Paths to allow directory listing only (readdir), not file contents
    pub allow_list_dirs: Vec<String>,
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
    Rust,
    Claude,
    Gpg,
    Bun,
    Opencode,
}

impl BuiltinProfile {
    /// Get a builtin profile by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "base" => Some(Self::Base),
            "online" => Some(Self::Online),
            "localhost" => Some(Self::Localhost),
            "rust" => Some(Self::Rust),
            "claude" => Some(Self::Claude),
            "gpg" => Some(Self::Gpg),
            "bun" => Some(Self::Bun),
            "opencode" => Some(Self::Opencode),
            _ => None,
        }
    }

    /// Get the name of this builtin profile
    pub fn name(&self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Online => "online",
            Self::Localhost => "localhost",
            Self::Rust => "rust",
            Self::Claude => "claude",
            Self::Gpg => "gpg",
            Self::Bun => "bun",
            Self::Opencode => "opencode",
        }
    }

    /// Load the profile data from embedded TOML files
    ///
    /// # Errors
    /// Returns `ProfileError::InvalidBuiltin` if the embedded TOML is invalid.
    /// This should never happen with properly tested builtin profiles.
    pub fn load(&self) -> Result<Profile, ProfileError> {
        let toml_str = match self {
            Self::Base => include_str!("../../profiles/base.toml"),
            Self::Online => include_str!("../../profiles/online.toml"),
            Self::Localhost => include_str!("../../profiles/localhost.toml"),
            Self::Rust => include_str!("../../profiles/rust.toml"),
            Self::Claude => include_str!("../../profiles/claude.toml"),
            Self::Gpg => include_str!("../../profiles/gpg.toml"),
            Self::Bun => include_str!("../../profiles/bun.toml"),
            Self::Opencode => include_str!("../../profiles/opencode.toml"),
        };
        toml::from_str(toml_str).map_err(|e| ProfileError::InvalidBuiltin {
            name: self.name(),
            error: e,
        })
    }
}

/// Load a profile from a TOML file
pub fn load_profile(path: &Path) -> Result<Profile, ProfileError> {
    let content = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

/// Load profiles by name, optionally searching in a custom directory.
/// Logs warnings for profiles that fail to load instead of silently skipping them.
pub fn load_profiles(names: &[String], custom_dir: Option<&Path>) -> Vec<Profile> {
    names
        .iter()
        .filter_map(|name| {
            // First try builtin profiles
            if let Some(builtin) = BuiltinProfile::from_name(name) {
                match builtin.load() {
                    Ok(profile) => return Some(profile),
                    Err(e) => {
                        // This should never happen with properly tested builtin profiles
                        eprintln!(
                            "\x1b[31m[sx:error]\x1b[0m Failed to load builtin profile '{}': {}",
                            name, e
                        );
                        return None;
                    }
                }
            }

            // Then try custom directory
            if let Some(dir) = custom_dir {
                let path = dir.join(format!("{}.toml", name));
                if path.exists() {
                    match load_profile(&path) {
                        Ok(profile) => return Some(profile),
                        Err(e) => {
                            eprintln!(
                                "\x1b[33m[sx:warn]\x1b[0m Failed to load profile '{}' from {}: {}",
                                name,
                                path.display(),
                                e
                            );
                            return None;
                        }
                    }
                }
            }

            // Try global profile directory
            if let Some(config_dir) = dirs::config_dir() {
                let path = config_dir
                    .join("sx/profiles")
                    .join(format!("{}.toml", name));
                if path.exists() {
                    match load_profile(&path) {
                        Ok(profile) => return Some(profile),
                        Err(e) => {
                            eprintln!(
                                "\x1b[33m[sx:warn]\x1b[0m Failed to load profile '{}' from {}: {}",
                                name,
                                path.display(),
                                e
                            );
                            return None;
                        }
                    }
                }
            }

            // Profile not found - warn and fallback to online
            eprintln!(
                "\x1b[33m[sx:warn]\x1b[0m Unknown profile '{}', falling back to 'online'",
                name
            );
            match BuiltinProfile::Online.load() {
                Ok(profile) => Some(profile),
                Err(e) => {
                    eprintln!(
                        "\x1b[31m[sx:error]\x1b[0m Failed to load fallback 'online' profile: {}",
                        e
                    );
                    None
                }
            }
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
        merge_unique(
            &mut result.filesystem.allow_read,
            &profile.filesystem.allow_read,
        );
        merge_unique(
            &mut result.filesystem.deny_read,
            &profile.filesystem.deny_read,
        );
        merge_unique(
            &mut result.filesystem.allow_write,
            &profile.filesystem.allow_write,
        );
        merge_unique(
            &mut result.filesystem.allow_list_dirs,
            &profile.filesystem.allow_list_dirs,
        );

        // Shell: merge unique env vars
        merge_unique(&mut result.shell.pass_env, &profile.shell.pass_env);
        merge_unique(&mut result.shell.deny_env, &profile.shell.deny_env);
    }

    result
}

/// Merge unique strings from source into target.
/// Uses HashSet for O(1) lookups instead of O(n) contains() checks.
fn merge_unique(target: &mut Vec<String>, source: &[String]) {
    // Build set of existing items (owned strings to avoid borrow conflicts)
    let existing: HashSet<String> = target.iter().cloned().collect();
    for item in source {
        if !existing.contains(item) {
            target.push(item.clone());
        }
    }
}
