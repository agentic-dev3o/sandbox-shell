//! Seatbelt profile generation for macOS sandbox
//!
//! Generates Apple Seatbelt profiles that enforce filesystem and network restrictions.
//! Uses a deny-by-default security model where only explicitly allowed paths are accessible.

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
    /// Paths to allow reading (deny-by-default, only these paths are readable)
    pub allow_read: Vec<PathBuf>,
    /// Paths to explicitly deny reading (overrides allow_read, for sensitive subpaths)
    pub deny_read: Vec<PathBuf>,
    /// Paths to allow writing (restricted by default)
    pub allow_write: Vec<PathBuf>,
    /// Raw seatbelt rules to include verbatim
    pub raw_rules: Option<String>,
}

/// Generate a Seatbelt profile from the given parameters
///
/// Security model (deny-by-default):
/// - Reads: Denied by default, only explicit allow_read paths are accessible
/// - Writes: Denied by default, allow working dir + explicit paths
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

    // System operations required for macOS to function
    profile.push_str("; System operations\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow file-ioctl)\n\n");

    // Mach services required for system functionality
    profile.push_str("; Mach services\n");
    profile.push_str("(allow mach-lookup)\n\n");

    // Allowed read paths (deny-by-default model)
    // Root directory literal is required for path traversal
    profile.push_str("; Allowed read paths (deny-by-default)\n");
    profile.push_str("(allow file-read* (literal \"/\"))\n");
    for path in &params.allow_read {
        let p = path.display();
        profile.push_str(&format!("(allow file-read* (subpath \"{p}\"))\n"));
    }
    profile.push('\n');

    // Deny sensitive paths (overrides allow_read for nested sensitive paths)
    // Uses last-match-wins: deny after allow takes precedence
    if !params.deny_read.is_empty() {
        profile.push_str("; Denied read paths (sensitive data)\n");
        for path in &params.deny_read {
            let p = path.display();
            profile.push_str(&format!("(deny file-read* (subpath \"{p}\"))\n"));
        }
        profile.push('\n');
    }

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
            // Use regex for files (to include lock files), subpath for directories
            if path.is_file() {
                // Escape special regex characters in path and allow file + any suffix (for .LOCK files)
                let escaped = p.to_string().replace('.', "\\.");
                profile.push_str(&format!("(allow file* (regex #\"^{escaped}.*\"))\n"));
            } else {
                profile.push_str(&format!("(allow file-write* (subpath \"{p}\"))\n"));
            }
        }
        profile.push('\n');
    }

    // Device access (stdout, stderr, tty)
    profile.push_str("; Device access\n");
    profile.push_str("(allow file-write* (subpath \"/dev\"))\n");
    profile.push_str("(allow file-read* (subpath \"/dev\"))\n\n");

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
    fn test_deny_by_default_no_global_read() {
        let params = SandboxParams::default();
        let profile = generate_seatbelt_profile(&params);
        // Should NOT have global read access - deny by default
        assert!(!profile.contains("(allow file-read* (subpath \"/\"))"));
    }

    #[test]
    fn test_allow_read_paths_included() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/usr"), PathBuf::from("/bin")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);
        assert!(profile.contains("(allow file-read* (subpath \"/usr\"))"));
        assert!(profile.contains("(allow file-read* (subpath \"/bin\"))"));
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
    fn test_deny_rules_come_after_allow_read() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/home")],
            deny_read: vec![PathBuf::from("/home/.ssh")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params);

        let deny_pos = profile
            .find("(deny file-read* (subpath \"/home/.ssh\"))")
            .expect("deny rule should exist");
        let allow_pos = profile
            .find("(allow file-read* (subpath \"/home\"))")
            .expect("allow rule should exist");

        assert!(
            deny_pos > allow_pos,
            "deny rules must come after allow rules for Seatbelt last-match-wins semantics"
        );
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
