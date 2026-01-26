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
    /// Network configuration
    pub network: ProfileNetwork,
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

/// Profile network configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileNetwork {
    pub allow_domains: Vec<String>,
    pub deny_domains: Vec<String>,
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

    /// Load the profile data
    pub fn load(&self) -> Profile {
        match self {
            Self::Base => base_profile(),
            Self::Online => online_profile(),
            Self::Localhost => localhost_profile(),
            Self::Node => node_profile(),
            Self::Python => python_profile(),
            Self::Rust => rust_profile(),
            Self::Go => go_profile(),
            Self::Claude => claude_profile(),
            Self::Gpg => gpg_profile(),
        }
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

        // Network: merge unique domains
        merge_unique(&mut result.network.allow_domains, &profile.network.allow_domains);
        merge_unique(&mut result.network.deny_domains, &profile.network.deny_domains);

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

// --- Built-in profile definitions ---

fn base_profile() -> Profile {
    Profile {
        network_mode: Some(NetworkMode::Offline),
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "/usr".to_string(),
                "/bin".to_string(),
                "/sbin".to_string(),
                "/opt".to_string(),
                "/Library".to_string(),
                "/System".to_string(),
                "/private/var/folders".to_string(),
                "/var/folders".to_string(),
                "/tmp".to_string(),
                "/dev".to_string(),
                "~/.gitconfig".to_string(),
                "~/.config/git".to_string(),
            ],
            deny_read: vec![
                "~/.ssh".to_string(),
                "~/.aws".to_string(),
                "~/.gnupg".to_string(),
                "~/.config/gh".to_string(),
                "~/.netrc".to_string(),
                "~/.docker/config.json".to_string(),
                "~/Documents".to_string(),
                "~/Desktop".to_string(),
                "~/Downloads".to_string(),
            ],
            allow_write: vec![
                "/tmp".to_string(),
                "/private/var/folders".to_string(),
                "/var/folders".to_string(),
            ],
        },
        network: ProfileNetwork::default(),
        shell: ProfileShell {
            pass_env: vec![
                "TERM".to_string(),
                "COLORTERM".to_string(),
                "LANG".to_string(),
                "LC_ALL".to_string(),
                "HOME".to_string(),
                "USER".to_string(),
                "SHELL".to_string(),
                "PATH".to_string(),
            ],
            deny_env: vec![
                "AWS_*".to_string(),
                "*_SECRET*".to_string(),
                "*_PASSWORD*".to_string(),
                "*_KEY".to_string(),
            ],
        },
        seatbelt: None,
    }
}

fn online_profile() -> Profile {
    Profile {
        network_mode: Some(NetworkMode::Online),
        ..Default::default()
    }
}

fn localhost_profile() -> Profile {
    Profile {
        network_mode: Some(NetworkMode::Localhost),
        ..Default::default()
    }
}

fn node_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/.npm".to_string(),
                "~/.npmrc".to_string(),
                "~/.node-gyp".to_string(),
                "~/.nvm".to_string(),
                "~/.node_repl_history".to_string(),
            ],
            allow_write: vec![
                "~/.npm/_cacache".to_string(),
            ],
            ..Default::default()
        },
        network: ProfileNetwork {
            allow_domains: vec![
                "registry.npmjs.org".to_string(),
                "npmjs.org".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn python_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/.pyenv".to_string(),
                "~/.local/share/virtualenvs".to_string(),
                "~/.python_history".to_string(),
                "/Library/Frameworks/Python.framework".to_string(),
            ],
            allow_write: vec![
                "~/.cache/pip".to_string(),
            ],
            ..Default::default()
        },
        network: ProfileNetwork {
            allow_domains: vec![
                "pypi.org".to_string(),
                "files.pythonhosted.org".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn rust_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/.cargo".to_string(),
                "~/.rustup".to_string(),
            ],
            allow_write: vec![
                "~/.cargo/registry".to_string(),
            ],
            ..Default::default()
        },
        network: ProfileNetwork {
            allow_domains: vec![
                "crates.io".to_string(),
                "static.crates.io".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn go_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/go".to_string(),
                "~/.config/go".to_string(),
            ],
            allow_write: vec![
                "~/go/pkg".to_string(),
            ],
            ..Default::default()
        },
        network: ProfileNetwork {
            allow_domains: vec![
                "proxy.golang.org".to_string(),
                "sum.golang.org".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn claude_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/.claude".to_string(),
            ],
            allow_write: vec![
                "~/.claude".to_string(),
            ],
            ..Default::default()
        },
        network: ProfileNetwork {
            allow_domains: vec![
                "api.anthropic.com".to_string(),
            ],
            ..Default::default()
        },
        shell: ProfileShell {
            pass_env: vec![
                "ANTHROPIC_API_KEY".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}

fn gpg_profile() -> Profile {
    Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec![
                "~/.gnupg".to_string(),
            ],
            allow_write: vec![
                "~/.gnupg".to_string(),
            ],
            ..Default::default()
        },
        ..Default::default()
    }
}
