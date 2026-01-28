use std::path::PathBuf;
use sx::config::profile::{compose_profiles, BuiltinProfile};
use sx::config::schema::NetworkMode;
use sx::sandbox::seatbelt::{generate_seatbelt_profile, SandboxParams};

#[test]
fn test_generate_deny_default() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Offline,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("(deny default)"),
        "Profile should deny by default"
    );
}

#[test]
fn test_allow_working_directory() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Offline,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains(r#"(subpath "/Users/test/project")"#),
        "Profile should allow working directory"
    );
}

#[test]
fn test_version_1() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("(version 1)"),
        "Profile should have version 1"
    );
}

#[test]
fn test_network_offline() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Offline,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    // In offline mode, no network-outbound should be allowed
    assert!(
        !profile.contains("(allow network-outbound"),
        "Offline mode should not allow network-outbound"
    );
}

#[test]
fn test_network_online() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Online,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("(allow network-outbound)") || profile.contains("(allow network*)"),
        "Online mode should allow network"
    );
}

#[test]
fn test_network_localhost() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Localhost,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("localhost") || profile.contains("127.0.0.1"),
        "Localhost mode should reference localhost"
    );
}

#[test]
fn test_deny_by_default_reads() {
    // Security model: deny-by-default, only explicit allow_read paths are accessible
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        allow_read: vec![PathBuf::from("/usr"), PathBuf::from("/bin")],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    // Should NOT have global read access
    assert!(
        !profile.contains(r#"(allow file-read* (subpath "/"))"#),
        "Should NOT allow global file-read (deny-by-default)"
    );
    // Should have explicit allow_read paths
    assert!(
        profile.contains(r#"(allow file-read* (subpath "/usr"))"#),
        "Should allow explicit /usr path"
    );
    assert!(
        profile.contains(r#"(allow file-read* (subpath "/bin"))"#),
        "Should allow explicit /bin path"
    );
}

#[test]
fn test_deny_read_paths() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        deny_read: vec![PathBuf::from("/Users/test/.ssh")],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains(r#"(subpath "/Users/test/.ssh")"#),
        "Should reference denied path"
    );
    // Deny rules should appear before allow rules
    let deny_pos = profile.find("deny file-read");
    assert!(deny_pos.is_some(), "Should have deny file-read rule");
}

#[test]
fn test_allow_write_paths() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        allow_write: vec![PathBuf::from("/tmp")],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("file-write") && profile.contains(r#"(subpath "/tmp")"#),
        "Should allow write to /tmp"
    );
}

#[test]
fn test_process_fork_allowed() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("(allow process-fork)"),
        "Should allow process-fork for child processes"
    );
}

#[test]
fn test_process_exec_allowed() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains("(allow process-exec)"),
        "Should allow process-exec for running commands"
    );
}

#[test]
fn test_base_profile_integration() {
    let base = BuiltinProfile::Base.load().unwrap();
    let composed = compose_profiles(&[base]);

    let expand_path = |p: &str| -> PathBuf {
        if p.starts_with("~/") {
            PathBuf::from("/Users/test").join(&p[2..])
        } else {
            PathBuf::from(p)
        }
    };

    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: composed.network_mode.unwrap_or(NetworkMode::Offline),
        allow_read: composed
            .filesystem
            .allow_read
            .iter()
            .map(|p| expand_path(p))
            .collect(),
        deny_read: composed
            .filesystem
            .deny_read
            .iter()
            .map(|p| expand_path(p))
            .collect(),
        allow_write: composed
            .filesystem
            .allow_write
            .iter()
            .map(|p| expand_path(p))
            .collect(),
        ..Default::default()
    };

    let profile = generate_seatbelt_profile(&params).unwrap();

    // Base profile should deny SSH
    assert!(
        profile.contains(".ssh"),
        "Base profile should reference .ssh in deny rules"
    );
    // Security model: deny-by-default with explicit allow paths
    assert!(
        !profile.contains(r#"(allow file-read* (subpath "/"))"#),
        "Base profile should NOT have global file-read (deny-by-default)"
    );
    // Should have explicit system paths allowed
    assert!(
        profile.contains(r#"(allow file-read* (subpath "/usr"))"#),
        "Base profile should allow /usr"
    );
}

#[test]
fn test_raw_seatbelt_rules() {
    let raw_rules = r#"(allow mach-lookup (global-name "com.apple.SecurityServer"))"#;
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        raw_rules: Some(raw_rules.to_string()),
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    assert!(
        profile.contains(raw_rules),
        "Should include raw seatbelt rules"
    );
}

#[test]
fn test_profile_is_valid_sexp() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        network_mode: NetworkMode::Offline,
        allow_read: vec![PathBuf::from("/usr")],
        deny_read: vec![PathBuf::from("/Users/test/.ssh")],
        allow_write: vec![PathBuf::from("/tmp")],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();

    // Basic s-expression validation: count parens
    let open_parens = profile.chars().filter(|c| *c == '(').count();
    let close_parens = profile.chars().filter(|c| *c == ')').count();
    assert_eq!(
        open_parens, close_parens,
        "Profile should have balanced parentheses"
    );
}

#[test]
fn test_mach_lookup_required() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    // mach-lookup is required for basic system functionality
    assert!(
        profile.contains("mach-lookup") || profile.contains("mach*"),
        "Should allow mach-lookup for system services"
    );
}

#[test]
fn test_signal_allowed() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    // signal is required for process control
    assert!(
        profile.contains("(allow signal"),
        "Should allow signal for process control"
    );
}

#[test]
fn test_system_read_paths() {
    let params = SandboxParams::default();
    let profile = generate_seatbelt_profile(&params).unwrap();
    // System read paths like /dev should be allowed
    assert!(
        profile.contains("/dev") || profile.contains("sysctl-read"),
        "Should allow reading system paths"
    );
}

#[test]
fn test_empty_paths_handled() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/Users/test/project"),
        home_dir: PathBuf::from("/Users/test"),
        allow_read: vec![],
        deny_read: vec![],
        allow_write: vec![],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&params).unwrap();
    // Should still produce valid profile
    assert!(profile.contains("(version 1)"));
    assert!(profile.contains("(deny default)"));
}

#[test]
fn test_invalid_path_rejected() {
    let params = SandboxParams {
        working_dir: PathBuf::from("/tmp"),
        home_dir: PathBuf::from("/Users/test"),
        allow_read: vec![PathBuf::from("/path/with\"quote")],
        ..Default::default()
    };
    let result = generate_seatbelt_profile(&params);
    assert!(result.is_err(), "Paths with quotes should be rejected");
}
