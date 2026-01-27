use sx::config::profile::{compose_profiles, load_profile, load_profiles, BuiltinProfile, Profile};
use sx::config::schema::NetworkMode;
use tempfile::TempDir;

#[test]
fn test_builtin_profile_base() {
    let profile = BuiltinProfile::Base.load();
    assert!(profile
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains("/usr")));
    assert!(profile
        .filesystem
        .deny_read
        .iter()
        .any(|p| p.contains(".ssh")));
}

#[test]
fn test_builtin_profile_online() {
    let profile = BuiltinProfile::Online.load();
    assert_eq!(profile.network_mode, Some(NetworkMode::Online));
}

#[test]
fn test_builtin_profile_localhost() {
    let profile = BuiltinProfile::Localhost.load();
    assert_eq!(profile.network_mode, Some(NetworkMode::Localhost));
}

#[test]
fn test_builtin_profile_rust() {
    let profile = BuiltinProfile::Rust.load();
    assert!(profile
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains(".cargo")));
    assert!(profile
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains(".rustup")));
}

#[test]
fn test_builtin_profile_claude() {
    let profile = BuiltinProfile::Claude.load();
    assert!(profile
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains(".claude")));
}

#[test]
fn test_builtin_profile_gpg() {
    let profile = BuiltinProfile::Gpg.load();
    assert!(profile
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains(".gnupg")));
}

#[test]
fn test_parse_profile_from_toml() {
    let toml_str = r#"
[filesystem]
allow_read = ["~/.config/myapp"]
allow_write = ["~/.cache/myapp"]

[shell]
pass_env = ["MY_API_KEY"]
"#;
    let profile: Profile = toml::from_str(toml_str).unwrap();
    assert!(profile
        .filesystem
        .allow_read
        .contains(&"~/.config/myapp".to_string()));
}

#[test]
fn test_load_profile_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("custom.toml");

    std::fs::write(
        &profile_path,
        r#"
[filesystem]
allow_read = ["~/custom-path"]
"#,
    )
    .unwrap();

    let profile = load_profile(&profile_path).unwrap();
    assert!(profile
        .filesystem
        .allow_read
        .contains(&"~/custom-path".to_string()));
}

#[test]
fn test_load_profile_not_found() {
    let result = load_profile(std::path::Path::new("/nonexistent/profile.toml"));
    assert!(result.is_err());
}

#[test]
fn test_load_profiles_by_name() {
    let profiles = load_profiles(&["base".to_string(), "online".to_string()], None);
    assert_eq!(profiles.len(), 2);
}

#[test]
fn test_load_profiles_with_custom_dir() {
    let temp_dir = TempDir::new().unwrap();
    let profile_path = temp_dir.path().join("myprofile.toml");

    std::fs::write(
        &profile_path,
        r#"
[filesystem]
allow_read = ["~/mypath"]
"#,
    )
    .unwrap();

    let profiles = load_profiles(&["myprofile".to_string()], Some(temp_dir.path()));
    assert_eq!(profiles.len(), 1);
    assert!(profiles[0]
        .filesystem
        .allow_read
        .contains(&"~/mypath".to_string()));
}

#[test]
fn test_compose_profiles_empty() {
    let composed = compose_profiles(&[]);
    assert!(composed.filesystem.allow_read.is_empty());
    assert!(composed.filesystem.deny_read.is_empty());
}

#[test]
fn test_compose_profiles_single() {
    let profile = BuiltinProfile::Base.load();
    let composed = compose_profiles(&[profile.clone()]);
    assert_eq!(
        composed.filesystem.allow_read,
        profile.filesystem.allow_read
    );
}

#[test]
fn test_compose_profiles_multiple() {
    let base = BuiltinProfile::Base.load();
    let rust = BuiltinProfile::Rust.load();

    let composed = compose_profiles(&[base, rust]);

    // Should have base paths
    assert!(composed
        .filesystem
        .deny_read
        .iter()
        .any(|p| p.contains(".ssh")));
    // Should have rust paths
    assert!(composed
        .filesystem
        .allow_read
        .iter()
        .any(|p| p.contains(".cargo")));
}

#[test]
fn test_compose_profiles_network_mode_last_wins() {
    let offline = Profile {
        network_mode: Some(NetworkMode::Offline),
        ..Default::default()
    };
    let online = Profile {
        network_mode: Some(NetworkMode::Online),
        ..Default::default()
    };

    let composed = compose_profiles(&[offline, online]);
    assert_eq!(composed.network_mode, Some(NetworkMode::Online));
}

#[test]
fn test_builtin_profile_from_name() {
    assert_eq!(
        BuiltinProfile::from_name("base"),
        Some(BuiltinProfile::Base)
    );
    assert_eq!(
        BuiltinProfile::from_name("online"),
        Some(BuiltinProfile::Online)
    );
    assert_eq!(
        BuiltinProfile::from_name("localhost"),
        Some(BuiltinProfile::Localhost)
    );
    assert_eq!(
        BuiltinProfile::from_name("rust"),
        Some(BuiltinProfile::Rust)
    );
    assert_eq!(
        BuiltinProfile::from_name("claude"),
        Some(BuiltinProfile::Claude)
    );
    assert_eq!(BuiltinProfile::from_name("gpg"), Some(BuiltinProfile::Gpg));
    assert_eq!(BuiltinProfile::from_name("unknown"), None);
}
