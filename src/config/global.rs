use anyhow::Result;
use std::path::Path;

use super::schema::Config;

/// Default global config path
///
/// Uses ~/.config/sx/config.toml (XDG Base Directory standard) for consistency
/// across platforms. This is the common convention for CLI tools.
pub fn default_config_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|p| p.join(".config").join("sx").join("config.toml"))
}

/// Load global configuration from file
///
/// If path is None, uses the default location (~/.config/sx/config.toml).
/// If the file doesn't exist, returns the default configuration.
pub fn load_global_config(path: Option<&Path>) -> Result<Config> {
    let config_path = match path {
        Some(p) => p.to_path_buf(),
        None => match default_config_path() {
            Some(p) => p,
            None => return Ok(Config::default()),
        },
    };

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}
