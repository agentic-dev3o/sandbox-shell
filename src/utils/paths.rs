use std::path::{Component, Path, PathBuf};

/// Expand a path string, handling ~ for home directory and resolving symlinks.
///
/// After tilde/env expansion, resolves symlinks in the longest existing prefix
/// of the path. This is critical on macOS where `/tmp` → `/private/tmp`, and
/// Seatbelt operates on resolved paths.
///
/// For glob paths like `/tmp/playwright*`, only the non-glob prefix (`/tmp/`)
/// is resolved, preserving the glob suffix.
pub fn expand_path(path: &str) -> PathBuf {
    let expanded = if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            home.join(stripped)
        } else {
            shell_expand(path)
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            home
        } else {
            shell_expand(path)
        }
    } else {
        shell_expand(path)
    };

    resolve_symlinks(&expanded)
}

fn shell_expand(path: &str) -> PathBuf {
    PathBuf::from(
        shellexpand::full(path)
            .unwrap_or_else(|_| path.into())
            .into_owned(),
    )
}

/// Resolve symlinks in the longest existing prefix of a path.
///
/// Walks the path components to find where glob characters start (or the full
/// path if no globs). Then finds the longest existing ancestor and canonicalizes
/// it, appending any remaining unresolved components.
///
/// Example: `/tmp/playwright*` → `/private/tmp/playwright*` (macOS)
fn resolve_symlinks(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    // Find the first component index that contains a glob character
    let components: Vec<Component> = path.components().collect();
    let glob_start = components
        .iter()
        .position(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.contains('*') || s.contains('?')
        })
        .unwrap_or(components.len());

    // Build the prefix (everything before the glob) to resolve
    let prefix: PathBuf = components[..glob_start].iter().collect();

    // Build the suffix (glob and beyond) to preserve as-is
    let suffix: PathBuf = if glob_start < components.len() {
        components[glob_start..].iter().collect()
    } else {
        PathBuf::new()
    };

    // Walk up the prefix to find the longest existing ancestor we can canonicalize
    let (resolved_base, unresolved_tail) = resolve_existing_prefix(&prefix);

    let mut result = resolved_base;
    if !unresolved_tail.as_os_str().is_empty() {
        result.push(unresolved_tail);
    }
    if !suffix.as_os_str().is_empty() {
        result.push(suffix);
    }

    // Only return the resolved path if it's still logically the same path
    // (just with symlinks resolved). If canonicalize changed nothing meaningful, return as-is.
    if result.to_string_lossy() == path_str {
        path.to_path_buf()
    } else {
        result
    }
}

/// Walk up the path to find the longest existing prefix, canonicalize it,
/// and return (resolved_prefix, remaining_tail).
fn resolve_existing_prefix(path: &Path) -> (PathBuf, PathBuf) {
    let mut current = path.to_path_buf();
    let mut tail_components: Vec<std::ffi::OsString> = Vec::new();

    loop {
        if let Ok(resolved) = std::fs::canonicalize(&current) {
            let tail: PathBuf = tail_components.iter().rev().collect();
            return (resolved, tail);
        }

        match current.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                if let Some(file_name) = current.file_name() {
                    tail_components.push(file_name.to_os_string());
                }
                current = parent.to_path_buf();
            }
            _ => {
                // Nothing could be resolved, return the original path
                return (path.to_path_buf(), PathBuf::new());
            }
        }
    }
}

/// Expand all paths in a vector
pub fn expand_paths(paths: &[String]) -> Vec<PathBuf> {
    paths.iter().map(|p| expand_path(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmp_symlink_resolved() {
        // On macOS, /tmp is a symlink to /private/tmp
        if std::fs::read_link("/tmp").is_ok() {
            let result = expand_path("/tmp/playwright*");
            assert!(
                result.to_string_lossy().starts_with("/private/tmp/"),
                "Expected /private/tmp/ prefix, got: {}",
                result.display()
            );
            assert!(
                result.to_string_lossy().ends_with("playwright*"),
                "Glob suffix should be preserved, got: {}",
                result.display()
            );
        }
    }

    #[test]
    fn test_tmp_non_glob_resolved() {
        if std::fs::read_link("/tmp").is_ok() {
            let result = expand_path("/tmp/somedir");
            assert!(
                result.to_string_lossy().starts_with("/private/tmp/"),
                "Expected /private/tmp/ prefix, got: {}",
                result.display()
            );
        }
    }

    #[test]
    fn test_tilde_expansion_preserved() {
        let result = expand_path("~/test");
        assert!(
            !result.to_string_lossy().starts_with("~"),
            "Tilde should be expanded, got: {}",
            result.display()
        );
    }

    #[test]
    fn test_non_symlink_path_unchanged() {
        let result = expand_path("/usr/bin");
        assert_eq!(result, PathBuf::from("/usr/bin"));
    }

    #[test]
    fn test_glob_in_middle_preserved() {
        if std::fs::read_link("/tmp").is_ok() {
            let result = expand_path("/tmp/convex*");
            assert_eq!(
                result,
                PathBuf::from("/private/tmp/convex*"),
                "Got: {}",
                result.display()
            );
        }
    }
}
