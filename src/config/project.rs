use anyhow::Result;
use std::path::Path;

use super::schema::Config;

/// Project config filename
pub const PROJECT_CONFIG_NAME: &str = ".sandbox.toml";

/// Load project configuration from the given directory
///
/// Looks for .sandbox.toml in the directory.
/// Returns None if the file doesn't exist.
pub fn load_project_config(dir: &Path) -> Result<Option<Config>> {
    let config_path = dir.join(PROJECT_CONFIG_NAME);

    if !config_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&config_path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(Some(config))
}

/// Find project root by looking for .sandbox.toml up the directory tree
pub fn find_project_config(start_dir: &Path) -> Option<std::path::PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let config_path = current.join(PROJECT_CONFIG_NAME);
        if config_path.exists() {
            return Some(config_path);
        }

        if !current.pop() {
            break;
        }
    }

    None
}
