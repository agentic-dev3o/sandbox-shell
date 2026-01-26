// Seatbelt profile generation
use crate::config::schema::NetworkMode;
use std::path::PathBuf;

/// Parameters for generating a Seatbelt sandbox profile
#[derive(Debug, Clone, Default)]
pub struct SandboxParams {
    /// Working directory (project root)
    pub working_dir: PathBuf,
    /// Home directory
    pub home_dir: PathBuf,
    /// Network mode
    pub network_mode: NetworkMode,
    /// Paths to allow reading
    pub allow_read: Vec<PathBuf>,
    /// Paths to deny reading
    pub deny_read: Vec<PathBuf>,
    /// Paths to allow writing
    pub allow_write: Vec<PathBuf>,
    /// Raw seatbelt rules to include
    pub raw_rules: Option<String>,
}

/// Generate a Seatbelt profile from the given parameters
pub fn generate_seatbelt_profile(params: &SandboxParams) -> String {
    let mut profile = String::new();

    // Version header
    profile.push_str("(version 1)\n");
    profile.push_str("(deny default)\n\n");

    // Allow essential process operations
    profile.push_str("; Process operations\n");
    profile.push_str("(allow process-fork)\n");
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow signal (target self))\n\n");

    // System read paths required for basic operation
    profile.push_str("; System read access\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow file-read-metadata)\n");

    // Mach services required for basic system functionality
    profile.push_str("\n; Mach services\n");
    profile.push_str("(allow mach-lookup)\n");

    // Working directory - full access
    profile.push_str("\n; Working directory (full access)\n");
    if !params.working_dir.as_os_str().is_empty() {
        let wd = params.working_dir.display();
        profile.push_str(&format!("(allow file* (subpath \"{wd}\"))\n"));
    }

    // Deny rules first (they should override allows)
    if !params.deny_read.is_empty() {
        profile.push_str("\n; Denied read paths\n");
        for path in &params.deny_read {
            let p = path.display();
            profile.push_str(&format!("(deny file-read* (subpath \"{p}\"))\n"));
        }
    }

    // Allow read paths
    if !params.allow_read.is_empty() {
        profile.push_str("\n; Allowed read paths\n");
        for path in &params.allow_read {
            let p = path.display();
            profile.push_str(&format!("(allow file-read* (subpath \"{p}\"))\n"));
        }
    }

    // Allow write paths
    if !params.allow_write.is_empty() {
        profile.push_str("\n; Allowed write paths\n");
        for path in &params.allow_write {
            let p = path.display();
            profile.push_str(&format!("(allow file-write* (subpath \"{p}\"))\n"));
        }
    }

    // /dev for device access
    profile.push_str("\n; Device access\n");
    profile.push_str("(allow file-read* (subpath \"/dev\"))\n");
    profile.push_str("(allow file-write* (subpath \"/dev/null\"))\n");
    profile.push_str("(allow file-write* (subpath \"/dev/tty\"))\n");
    profile.push_str("(allow file-ioctl (subpath \"/dev\"))\n");

    // Network rules based on mode
    profile.push_str("\n; Network access\n");
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
}
