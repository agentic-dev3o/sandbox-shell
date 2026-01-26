use std::path::PathBuf;

/// Expand a path string, handling ~ for home directory
pub fn expand_path(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }

    PathBuf::from(shellexpand::full(path).unwrap_or_else(|_| path.into()).into_owned())
}

/// Expand all paths in a vector
pub fn expand_paths(paths: &[String]) -> Vec<PathBuf> {
    paths.iter().map(|p| expand_path(p)).collect()
}
