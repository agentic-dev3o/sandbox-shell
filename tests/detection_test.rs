//! Unit tests for project type detection

use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

use sx::detection::project_type::{detect_project_type, detect_project_types, ProjectType};

#[test]
fn test_project_type_display() {
    assert_eq!(ProjectType::Node.as_str(), "node");
    assert_eq!(ProjectType::Python.as_str(), "python");
    assert_eq!(ProjectType::Rust.as_str(), "rust");
    assert_eq!(ProjectType::Go.as_str(), "go");
}

#[test]
fn test_detect_node_project() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Node));
}

#[test]
fn test_detect_python_requirements() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("requirements.txt"), "requests\n").unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Python));
}

#[test]
fn test_detect_python_pyproject() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("pyproject.toml"),
        "[project]\nname = \"test\"\n",
    )
    .unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Python));
}

#[test]
fn test_detect_python_setup() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("setup.py"),
        "from setuptools import setup\n",
    )
    .unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Python));
}

#[test]
fn test_detect_rust_project() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\n",
    )
    .unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Rust));
}

#[test]
fn test_detect_go_project() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("go.mod"), "module example.com/test\n").unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Go));
}

#[test]
fn test_detect_multiple_project_types() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]\n").unwrap();

    let types = detect_project_types(temp.path());
    assert!(types.contains(&ProjectType::Node));
    assert!(types.contains(&ProjectType::Rust));
    assert_eq!(types.len(), 2);
}

#[test]
fn test_detect_no_project_type() {
    let temp = TempDir::new().unwrap();
    // Empty directory

    let types = detect_project_types(temp.path());
    assert!(types.is_empty());
}

#[test]
fn test_detect_primary_project_type() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("Cargo.toml"), "[package]\n").unwrap();

    let primary = detect_project_type(temp.path());
    assert_eq!(primary, Some(ProjectType::Rust));
}

#[test]
fn test_detect_primary_none() {
    let temp = TempDir::new().unwrap();

    let primary = detect_project_type(temp.path());
    assert_eq!(primary, None);
}

#[test]
fn test_detect_with_custom_rules() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("custom.marker"), "").unwrap();

    // Custom rules should be extensible
    let mut custom_rules: HashMap<String, Vec<String>> = HashMap::new();
    custom_rules.insert("custom".to_string(), vec!["custom.marker".to_string()]);

    let types =
        sx::detection::project_type::detect_project_types_with_rules(temp.path(), &custom_rules);
    assert!(types.contains(&"custom".to_string()));
}

#[test]
fn test_project_type_to_profile() {
    assert_eq!(ProjectType::Node.to_profile(), "node");
    assert_eq!(ProjectType::Python.to_profile(), "python");
    assert_eq!(ProjectType::Rust.to_profile(), "rust");
    assert_eq!(ProjectType::Go.to_profile(), "go");
}
