//! Seatbelt profile generation for macOS sandbox
//!
//! Generates Apple Seatbelt profiles that enforce filesystem and network restrictions.
//! Uses a "allow reads, deny sensitive paths, restrict writes" security model.

use crate::config::schema::NetworkMode;
use std::path::PathBuf;

/// Parameters for generating a Seatbelt sandbox profile
#[derive(Debug, Clone, Default)]
pub struct SandboxParams {
    /// Working directory (project root) - gets full read/write access
    pub working_dir: PathBuf,
    /// Home directory
    pub home_dir: PathBuf,
    /// Network mode (offline, online, localhost)
    pub network_mode: NetworkMode,
    /// Additional paths to allow reading (beyond global read access)
    pub allow_read: Vec<PathBuf>,
    /// Paths to explicitly deny reading (overrides global allow)
    pub deny_read: Vec<PathBuf>,
    /// Paths to allow writing (restricted by default)
    pub allow_write: Vec<PathBuf>,
    /// Raw seatbelt rules to include verbatim
    pub raw_rules: Option<String>,
}

/// Generate a Seatbelt profile from the given parameters
///
/// Security model:
/// - Reads: Allow globally, deny specific sensitive paths
/// - Writes: Deny by default, allow working dir + explicit paths
/// - Network: Configurable (offline/localhost/online)
pub fn generate_seatbelt_profile(params: &SandboxParams) -> String {
    let mut profile = String::new();

    // Version and default deny
    profile.push_str("(version 1)\n");
    profile.push_str("(deny default)\n\n");

    // Process operations
    profile.push_str("; Process operations\n");
    profile.push_str("(allow process-fork)\n");
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow signal)\n\n");

    // Global read access (required for macOS to function)
    // Security: reads without network access cannot exfiltrate data
    profile.push_str("; Global read access\n");
    profile.push_str("(allow file-read-data)\n");
    profile.push_str("(allow file-read-xattr)\n");
    profile.push_str("(allow file-read-metadata)\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow file-ioctl)\n\n");

    // Deny sensitive paths (overrides global read allow)
    // These paths contain credentials and sensitive data
    profile.push_str("; Denied read paths (sensitive data)\n");
    for path in &params.deny_read {
        let p = path.display();
        profile.push_str(&format!("(deny file-read* (subpath \"{p}\"))\n"));
    }
    if !params.deny_read.is_empty() {
        profile.push('\n');
    }

    // Mach services required for system functionality
    profile.push_str("; Mach services\n");
    profile.push_str("(allow mach-lookup)\n\n");

    // Working directory - full read/write access
    profile.push_str("; Working directory (full access)\n");
    if !params.working_dir.as_os_str().is_empty() {
        let wd = params.working_dir.display();
        profile.push_str(&format!("(allow file* (subpath \"{wd}\"))\n\n"));
    }

    // Allowed write paths (beyond working directory)
    if !params.allow_write.is_empty() {
        profile.push_str("; Allowed write paths\n");
        for path in &params.allow_write {
            let p = path.display();
            profile.push_str(&format!("(allow file-write* (subpath \"{p}\"))\n"));
        }
        profile.push('\n');
    }

    // Device access (stdout, stderr, tty)
    profile.push_str("; Device access\n");
    profile.push_str("(allow file-write* (subpath \"/dev\"))\n\n");

    // Network rules based on mode
    profile.push_str("; Network access\n");
    match params.network_mode {
        NetworkMode::Offline => {
            profile.push_str("; Network disabled (offline mode)\n");
        }
        NetworkMode::Online => {
            profile.push_str("(allow network*)\n");
        }
        NetworkMode::Localhost => {
            profile.push_str("(allow network-outbound (to ip \"localhost:*\"))\n");
            profile.push_str("(allow network-outbound (to ip \"127.0.0.1:*\"))\n");
            profile.push_str("(allow network-inbound (from ip \"localhost:*\"))\n");
            profile.push_str("(allow network-inbound (from ip \"127.0.0.1:*\"))\n");
        }
    }

    // Raw rules if provided
    if let Some(raw) = &params.raw_rules {
        profile.push_str("\n; Custom rules\n");
        profile.push_str(raw);
        profile.push('\n');
    }

    profile
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params_produces_valid_profile() {
        let params = SandboxParams::default();
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
    }

    #[test]
    fn test_profile_has_global_read() {
        let params = SandboxParams::default();
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(allow file-read-data)"));
        assert!(profile.contains("(allow file-read-xattr)"));
    }

    #[test]
    fn test_deny_paths_included() {
        let params = SandboxParams {
            deny_read: vec![PathBuf::from("/secret")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(deny file-read* (subpath \"/secret\"))"));
    }

    #[test]
    fn test_working_dir_has_full_access() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/projects/myapp"),
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(allow file* (subpath \"/projects/myapp\"))"));
    }

    #[test]
    fn test_network_offline() {
        let params = SandboxParams {
            network_mode: NetworkMode::Offline,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("Network disabled"));
        assert!(!profile.contains("(allow network"));
    }

    #[test]
    fn test_network_online() {
        let params = SandboxParams {
            network_mode: NetworkMode::Online,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(allow network*)"));
    }

    #[test]
    fn test_network_localhost() {
        let params = SandboxParams {
            network_mode: NetworkMode::Localhost,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("localhost"));
        assert!(profile.contains("127.0.0.1"));
    }
}
