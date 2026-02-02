//! Auto-detect project type based on marker files

use std::collections::HashMap;
use std::path::Path;

/// Known project types with their default detection markers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectType {
    Node,
    Python,
    Rust,
    Go,
    Bun,
}

impl ProjectType {
    /// Get the string representation used in profiles
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectType::Node => "node",
            ProjectType::Python => "python",
            ProjectType::Rust => "rust",
            ProjectType::Go => "go",
            ProjectType::Bun => "bun",
        }
    }

    /// Get the profile name for this project type
    pub fn to_profile(&self) -> &'static str {
        self.as_str()
    }

    /// Get all known project types
    fn all() -> &'static [ProjectType] {
        &[
            ProjectType::Bun, // Check Bun first (more specific than Node)
            ProjectType::Node,
            ProjectType::Python,
            ProjectType::Rust,
            ProjectType::Go,
        ]
    }

    /// Get default marker files for this project type
    fn markers(&self) -> &'static [&'static str] {
        match self {
            ProjectType::Node => &["package.json"],
            ProjectType::Python => &["requirements.txt", "pyproject.toml", "setup.py"],
            ProjectType::Rust => &["Cargo.toml"],
            ProjectType::Go => &["go.mod"],
            ProjectType::Bun => &["bun.lockb", "bunfig.toml"],
        }
    }
}

/// Detect all project types present in the given directory
pub fn detect_project_types(dir: &Path) -> Vec<ProjectType> {
    ProjectType::all()
        .iter()
        .filter(|pt| pt.markers().iter().any(|marker| dir.join(marker).exists()))
        .copied()
        .collect()
}

/// Detect the primary project type (returns first match)
pub fn detect_project_type(dir: &Path) -> Option<ProjectType> {
    detect_project_types(dir).into_iter().next()
}

/// Detect project types using custom rules
/// Returns a list of profile names (strings) that matched
pub fn detect_project_types_with_rules(
    dir: &Path,
    rules: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    rules
        .iter()
        .filter(|(_, markers)| markers.iter().any(|marker| dir.join(marker).exists()))
        .map(|(name, _)| name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_type_as_str() {
        assert_eq!(ProjectType::Node.as_str(), "node");
        assert_eq!(ProjectType::Python.as_str(), "python");
        assert_eq!(ProjectType::Rust.as_str(), "rust");
        assert_eq!(ProjectType::Go.as_str(), "go");
        assert_eq!(ProjectType::Bun.as_str(), "bun");
    }

    #[test]
    fn test_project_type_markers() {
        assert!(ProjectType::Node.markers().contains(&"package.json"));
        assert!(ProjectType::Python.markers().contains(&"requirements.txt"));
        assert!(ProjectType::Python.markers().contains(&"pyproject.toml"));
        assert!(ProjectType::Rust.markers().contains(&"Cargo.toml"));
        assert!(ProjectType::Go.markers().contains(&"go.mod"));
        assert!(ProjectType::Bun.markers().contains(&"bun.lockb"));
        assert!(ProjectType::Bun.markers().contains(&"bunfig.toml"));
    }
}
