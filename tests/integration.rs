// Integration tests for sx sandbox CLI
// These tests verify end-to-end sandbox functionality
//
// Note: On macOS 10.15+ (Catalina and later), sandbox-exec with custom profiles
// may be restricted. Tests that invoke sandbox-exec will check if the sandbox
// is available and skip gracefully if not.

use sx::config::global::load_global_config;
use sx::config::merge::merge_configs;
use sx::config::profile::{
    compose_profiles, load_profiles, BuiltinProfile, Profile, ProfileFilesystem, ProfileShell,
};
use sx::config::project::load_project_config;
use sx::config::schema::{Config, FilesystemConfig, NetworkMode, SandboxConfig};
use sx::config::project::PROJECT_CONFIG_NAME;
use sx::sandbox::executor::execute_sandboxed_captured;
use sx::sandbox::seatbelt::{generate_seatbelt_profile, SandboxParams};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Check if sandbox-exec with custom deny-default profiles is available
/// On newer macOS versions, custom sandbox profiles with deny-default may be restricted
fn is_custom_sandbox_available() -> bool {
    // Test with a deny-default profile that should allow basic execution
    let profile = r#"(version 1)
(deny default)
(allow process-fork)
(allow process-exec)
(allow signal (target self))
(allow sysctl-read)
(allow file-read-metadata)
(allow mach-lookup)
(allow file-read* (subpath "/usr"))
(allow file-read* (subpath "/bin"))
(allow file-read* (subpath "/dev"))
"#;
    let temp = tempfile::NamedTempFile::new().ok();
    if let Some(ref f) = temp {
        if fs::write(f.path(), profile).is_err() {
            return false;
        }
        let result = Command::new("/usr/bin/sandbox-exec")
            .arg("-f")
            .arg(f.path())
            .arg("/bin/echo")
            .arg("test")
            .output();
        if let Ok(output) = result {
            return output.status.success();
        }
    }
    false
}

/// Skip test if custom sandbox is not available
macro_rules! skip_if_no_sandbox {
    () => {
        if !is_custom_sandbox_available() {
            eprintln!("Skipping test: custom sandbox profiles not available on this system");
            return;
        }
    };
}

// ============================================================================
// Filesystem Integration Tests
// ============================================================================

/// Helper to create sandbox params for filesystem tests
fn fs_sandbox_params(working_dir: PathBuf) -> SandboxParams {
    SandboxParams {
        working_dir,
        home_dir: dirs::home_dir().unwrap_or_default(),
        network_mode: NetworkMode::Offline,
        allow_read: vec![
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
        ],
        deny_read: vec![],
        allow_write: vec![],
        raw_rules: None,
    }
}

#[test]
fn test_sandbox_allows_read_in_working_dir() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("test.txt");
    fs::write(&test_file, "hello").unwrap();

    let params = fs_sandbox_params(temp.path().to_path_buf());
    let (status, stdout, _) = execute_sandboxed_captured(
        &params,
        &["/bin/cat".to_string(), test_file.to_string_lossy().to_string()],
    )
    .unwrap();

    assert!(status.success(), "Should allow reading file in working dir");
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "hello");
}

#[test]
fn test_sandbox_allows_write_in_working_dir() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let test_file = temp.path().join("output.txt");

    let params = fs_sandbox_params(temp.path().to_path_buf());
    let (status, _, _) = execute_sandboxed_captured(
        &params,
        &[
            "/bin/sh".to_string(),
            "-c".to_string(),
            format!("echo 'written' > {}", test_file.to_string_lossy()),
        ],
    )
    .unwrap();

    assert!(status.success(), "Should allow writing file in working dir");
    assert!(test_file.exists(), "File should have been created");
    assert_eq!(fs::read_to_string(&test_file).unwrap().trim(), "written");
}

#[test]
fn test_sandbox_allows_reading_system_binaries() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let params = fs_sandbox_params(temp.path().to_path_buf());

    let (status, _, _) = execute_sandboxed_captured(
        &params,
        &["/bin/ls".to_string(), "/bin".to_string()],
    )
    .unwrap();

    assert!(status.success(), "Should allow listing /bin directory");
}

#[test]
fn test_sandbox_can_execute_basic_commands() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let params = fs_sandbox_params(temp.path().to_path_buf());

    let (status, stdout, _) = execute_sandboxed_captured(
        &params,
        &["/bin/echo".to_string(), "sandbox test".to_string()],
    )
    .unwrap();

    assert!(status.success(), "Should execute echo command");
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "sandbox test");
}

#[test]
fn test_sandbox_with_deny_read_paths() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let secret_dir = temp.path().join("secrets");
    fs::create_dir(&secret_dir).unwrap();
    fs::write(secret_dir.join("key.pem"), "secret key").unwrap();

    let mut params = fs_sandbox_params(temp.path().to_path_buf());
    params.deny_read.push(secret_dir.clone());

    let (status, _, _) = execute_sandboxed_captured(
        &params,
        &["/bin/cat".to_string(), secret_dir.join("key.pem").to_string_lossy().to_string()],
    )
    .unwrap();

    // Should fail to read denied path
    assert!(!status.success(), "Should deny reading from denied path");
}

#[test]
fn test_sandbox_with_explicit_allow_write() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let write_dir = TempDir::new().unwrap();

    let mut params = fs_sandbox_params(temp.path().to_path_buf());
    params.allow_write.push(write_dir.path().to_path_buf());

    let test_file = write_dir.path().join("allowed.txt");
    let (status, _, _) = execute_sandboxed_captured(
        &params,
        &[
            "/bin/sh".to_string(),
            "-c".to_string(),
            format!("echo 'allowed' > {}", test_file.to_string_lossy()),
        ],
    )
    .unwrap();

    assert!(status.success(), "Should allow writing to explicitly allowed path");
    assert!(test_file.exists(), "File should have been created");
}

#[test]
fn test_sandbox_temp_file_creation() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let mut params = fs_sandbox_params(temp.path().to_path_buf());
    params.allow_write.push(PathBuf::from("/tmp"));
    params.allow_read.push(PathBuf::from("/tmp"));

    let (status, stdout, _) = execute_sandboxed_captured(
        &params,
        &[
            "/bin/sh".to_string(),
            "-c".to_string(),
            "TMPFILE=$(mktemp) && echo 'temp data' > $TMPFILE && cat $TMPFILE && rm $TMPFILE".to_string(),
        ],
    )
    .unwrap();

    assert!(status.success(), "Should allow temp file operations");
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "temp data");
}

// ============================================================================
// Network Integration Tests
// ============================================================================

/// Helper to create sandbox params for network tests
fn network_sandbox_params(working_dir: PathBuf, mode: NetworkMode) -> SandboxParams {
    SandboxParams {
        working_dir,
        home_dir: dirs::home_dir().unwrap_or_default(),
        network_mode: mode,
        allow_read: vec![
            PathBuf::from("/usr"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/etc"),
            PathBuf::from("/private/etc"),
        ],
        deny_read: vec![],
        allow_write: vec![],
        raw_rules: None,
    }
}

#[test]
fn test_offline_mode_blocks_network() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Offline);

    // Try to resolve DNS (should fail in offline mode)
    let (status, _, _) = execute_sandboxed_captured(
        &params,
        &[
            "/usr/bin/host".to_string(),
            "example.com".to_string(),
        ],
    )
    .unwrap();

    // In offline mode, network operations should fail
    assert!(!status.success(), "Offline mode should block network access");
}

#[test]
fn test_online_mode_allows_execution() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Online);

    let (status, stdout, _) = execute_sandboxed_captured(
        &params,
        &["/bin/echo".to_string(), "network enabled".to_string()],
    )
    .unwrap();

    assert!(status.success(), "Online mode should allow execution");
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "network enabled");
}

#[test]
fn test_localhost_mode_profile_content() {
    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Localhost);

    let profile = generate_seatbelt_profile(&params);
    assert!(profile.contains("localhost"), "Localhost mode should allow localhost");
    assert!(profile.contains("127.0.0.1"), "Localhost mode should allow 127.0.0.1");
}

#[test]
fn test_network_mode_offline_profile_content() {
    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Offline);

    let profile = generate_seatbelt_profile(&params);

    assert!(!profile.contains("(allow network*)"), "Offline should not allow network*");
    assert!(profile.contains("Network disabled"), "Offline should have disabled comment");
}

#[test]
fn test_network_mode_online_profile_content() {
    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Online);

    let profile = generate_seatbelt_profile(&params);

    assert!(profile.contains("(allow network*)"), "Online should allow network*");
}

#[test]
fn test_offline_sandbox_still_allows_execution() {
    skip_if_no_sandbox!();

    let temp = TempDir::new().unwrap();
    let params = network_sandbox_params(temp.path().to_path_buf(), NetworkMode::Offline);

    let (status, stdout, _) = execute_sandboxed_captured(
        &params,
        &["/bin/echo".to_string(), "offline test".to_string()],
    )
    .unwrap();

    assert!(status.success(), "Offline mode should still allow local execution");
    assert_eq!(String::from_utf8_lossy(&stdout).trim(), "offline test");
}

#[test]
fn test_sandbox_default_is_offline() {
    let params = SandboxParams::default();
    assert_eq!(params.network_mode, NetworkMode::Offline, "Default should be offline");
}

// ============================================================================
// Config Integration Tests
// ============================================================================

#[test]
fn test_load_default_global_config() {
    let config = load_global_config(None).unwrap();
    assert_eq!(config.sandbox.default_network, NetworkMode::Offline);
    assert!(config.sandbox.prompt_indicator);
}

#[test]
fn test_load_project_config_from_file() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join(PROJECT_CONFIG_NAME);

    fs::write(
        &config_path,
        r#"
[sandbox]
network = "online"
profiles = ["node", "online"]

[filesystem]
allow_read = ["/custom/path"]
"#,
    )
    .unwrap();

    let config = load_project_config(temp.path()).unwrap();
    assert!(config.is_some(), "Should load project config");

    let config = config.unwrap();
    assert_eq!(config.sandbox.network, Some(NetworkMode::Online));
    assert_eq!(config.sandbox.profiles, vec!["node", "online"]);
    assert!(config.filesystem.allow_read.contains(&"/custom/path".to_string()));
}

#[test]
fn test_load_project_config_missing_file() {
    let temp = TempDir::new().unwrap();
    let config = load_project_config(temp.path()).unwrap();
    assert!(config.is_none(), "Should return None for missing config");
}

#[test]
fn test_merge_global_and_project_configs() {
    let global = Config {
        sandbox: SandboxConfig {
            default_network: NetworkMode::Offline,
            default_profiles: vec!["base".to_string()],
            prompt_indicator: true,
            ..Default::default()
        },
        filesystem: FilesystemConfig {
            allow_read: vec!["/global/path".to_string()],
            deny_read: vec!["~/.ssh".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let project = Config {
        sandbox: SandboxConfig {
            network: Some(NetworkMode::Online),
            profiles: vec!["node".to_string()],
            ..Default::default()
        },
        filesystem: FilesystemConfig {
            allow_read: vec!["/project/path".to_string()],
            deny_read: vec!["~/Documents".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let merged = merge_configs(&global, &project);

    assert_eq!(merged.sandbox.network, Some(NetworkMode::Online));
    assert!(merged.filesystem.allow_read.contains(&"/global/path".to_string()));
    assert!(merged.filesystem.allow_read.contains(&"/project/path".to_string()));
    assert!(merged.filesystem.deny_read.contains(&"~/.ssh".to_string()));
    assert!(merged.filesystem.deny_read.contains(&"~/Documents".to_string()));
}

#[test]
fn test_config_default_profiles() {
    let config = Config::default();
    assert!(config.sandbox.default_profiles.contains(&"base".to_string()));
}

#[test]
fn test_project_config_network_mode_parsing() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join(PROJECT_CONFIG_NAME);

    for (mode_str, expected_mode) in [
        ("offline", NetworkMode::Offline),
        ("online", NetworkMode::Online),
        ("localhost", NetworkMode::Localhost),
    ] {
        fs::write(
            &config_path,
            format!(r#"[sandbox]
network = "{}""#, mode_str),
        )
        .unwrap();

        let config = load_project_config(temp.path()).unwrap().unwrap();
        assert_eq!(config.sandbox.network, Some(expected_mode));
    }
}

#[test]
fn test_shell_config_env_vars() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join(PROJECT_CONFIG_NAME);

    fs::write(
        &config_path,
        r#"
[shell]
pass_env = ["CUSTOM_VAR", "MY_TOKEN"]
deny_env = ["SECRET_KEY", "AWS_*"]
"#,
    )
    .unwrap();

    let config = load_project_config(temp.path()).unwrap().unwrap();
    assert!(config.shell.pass_env.contains(&"CUSTOM_VAR".to_string()));
    assert!(config.shell.deny_env.contains(&"AWS_*".to_string()));
}

// ============================================================================
// Profile Integration Tests
// ============================================================================

#[test]
fn test_load_all_builtin_profiles() {
    let builtin_names = ["base", "online", "localhost", "rust", "claude", "gpg"];

    for name in builtin_names {
        let profiles = load_profiles(&[name.to_string()], None);
        assert_eq!(profiles.len(), 1, "Should load builtin profile: {}", name);
    }
}

#[test]
fn test_base_profile_has_required_paths() {
    let profile = BuiltinProfile::Base.load();

    assert!(profile.filesystem.allow_read.iter().any(|p| p.contains("/usr")));
    assert!(profile.filesystem.allow_read.iter().any(|p| p.contains("/bin")));
    assert!(profile.filesystem.allow_read.iter().any(|p| p.contains("/tmp")));

    assert!(profile.filesystem.deny_read.iter().any(|p| p.contains(".ssh")));
    assert!(profile.filesystem.deny_read.iter().any(|p| p.contains(".aws")));
}

#[test]
fn test_online_profile_sets_network_mode() {
    let profile = BuiltinProfile::Online.load();
    assert_eq!(profile.network_mode, Some(NetworkMode::Online));
}

#[test]
fn test_localhost_profile_sets_network_mode() {
    let profile = BuiltinProfile::Localhost.load();
    assert_eq!(profile.network_mode, Some(NetworkMode::Localhost));
}

#[test]
fn test_rust_profile_allows_cargo() {
    let profile = BuiltinProfile::Rust.load();

    assert!(profile.filesystem.allow_read.iter().any(|p| p.contains(".cargo")));
    assert!(profile.filesystem.allow_read.iter().any(|p| p.contains(".rustup")));
}

#[test]
fn test_compose_multiple_profiles() {
    let profiles = vec![
        BuiltinProfile::Base.load(),
        BuiltinProfile::Rust.load(),
        BuiltinProfile::Online.load(),
    ];

    let composed = compose_profiles(&profiles);

    assert_eq!(composed.network_mode, Some(NetworkMode::Online));
    assert!(composed.filesystem.allow_read.iter().any(|p| p.contains("/usr")));
    assert!(composed.filesystem.allow_read.iter().any(|p| p.contains(".cargo")));
}

#[test]
fn test_compose_profiles_network_mode_last_wins() {
    let profiles = vec![
        BuiltinProfile::Online.load(),
        BuiltinProfile::Localhost.load(),
    ];

    let composed = compose_profiles(&profiles);
    assert_eq!(composed.network_mode, Some(NetworkMode::Localhost));
}

#[test]
fn test_compose_profiles_merges_unique_paths() {
    let profile1 = Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec!["/path/a".to_string(), "/path/b".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let profile2 = Profile {
        filesystem: ProfileFilesystem {
            allow_read: vec!["/path/b".to_string(), "/path/c".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };

    let composed = compose_profiles(&[profile1, profile2]);

    assert_eq!(composed.filesystem.allow_read.len(), 3);
    assert!(composed.filesystem.allow_read.contains(&"/path/a".to_string()));
    assert!(composed.filesystem.allow_read.contains(&"/path/b".to_string()));
    assert!(composed.filesystem.allow_read.contains(&"/path/c".to_string()));
}

#[test]
fn test_load_custom_profile_from_directory() {
    let temp = TempDir::new().unwrap();
    let profile_path = temp.path().join("custom.toml");

    fs::write(
        &profile_path,
        r#"
network_mode = "localhost"

[filesystem]
allow_read = ["/custom/read"]
allow_write = ["/custom/write"]
"#,
    )
    .unwrap();

    let profiles = load_profiles(&["custom".to_string()], Some(temp.path()));
    assert_eq!(profiles.len(), 1, "Should load custom profile");

    let profile = &profiles[0];
    assert_eq!(profile.network_mode, Some(NetworkMode::Localhost));
    assert!(profile.filesystem.allow_read.contains(&"/custom/read".to_string()));
}

#[test]
fn test_load_missing_profile_returns_empty() {
    let profiles = load_profiles(&["nonexistent".to_string()], None);
    assert!(profiles.is_empty(), "Should return empty for missing profile");
}

#[test]
fn test_profile_shell_env_merging() {
    let profile1 = Profile {
        shell: ProfileShell {
            pass_env: vec!["VAR1".to_string(), "VAR2".to_string()],
            deny_env: vec!["SECRET1".to_string()],
        },
        ..Default::default()
    };

    let profile2 = Profile {
        shell: ProfileShell {
            pass_env: vec!["VAR2".to_string(), "VAR3".to_string()],
            deny_env: vec!["SECRET2".to_string()],
        },
        ..Default::default()
    };

    let composed = compose_profiles(&[profile1, profile2]);

    assert_eq!(composed.shell.pass_env.len(), 3);
    assert_eq!(composed.shell.deny_env.len(), 2);
}
