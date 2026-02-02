//! Seatbelt profile generation for macOS sandbox
//!
//! Generates Apple Seatbelt profiles that enforce filesystem and network restrictions.
//! Uses a deny-by-default security model where only explicitly allowed paths are accessible.

use crate::config::schema::NetworkMode;
use std::path::PathBuf;

/// Error type for seatbelt profile generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatbeltError {
    /// Path contains invalid characters that could break seatbelt syntax
    InvalidPath { path: String, reason: &'static str },
}

impl std::fmt::Display for SeatbeltError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SeatbeltError::InvalidPath { path, reason } => {
                write!(f, "Invalid path '{}': {}", path, reason)
            }
        }
    }
}

impl std::error::Error for SeatbeltError {}

/// Validate and sanitize a path for use in seatbelt profiles.
/// Returns an error if the path contains characters that could break seatbelt syntax
/// or potentially inject additional rules.
fn validate_seatbelt_path(path: &str) -> Result<&str, SeatbeltError> {
    // Check for null bytes (could truncate the path)
    if path.contains('\0') {
        return Err(SeatbeltError::InvalidPath {
            path: path.to_string(),
            reason: "path contains null byte",
        });
    }

    // Check for unescaped double quotes (could break string literals)
    if path.contains('"') {
        return Err(SeatbeltError::InvalidPath {
            path: path.to_string(),
            reason: "path contains unescaped double quote",
        });
    }

    // Check for newlines (could inject new rules)
    if path.contains('\n') || path.contains('\r') {
        return Err(SeatbeltError::InvalidPath {
            path: path.to_string(),
            reason: "path contains newline character",
        });
    }

    Ok(path)
}

/// Check if a path string contains glob wildcard characters
fn contains_glob(path: &str) -> bool {
    path.contains('*') || path.contains('?')
}

/// Convert a glob pattern to a Seatbelt regex pattern
/// Escapes regex special characters and converts glob wildcards to regex equivalents
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::with_capacity(pattern.len() * 2);
    regex.push('^');

    for c in pattern.chars() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            // Escape regex special characters
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\' => {
                regex.push('\\');
                regex.push(c);
            }
            _ => regex.push(c),
        }
    }

    regex
}

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
    /// Paths to allow directory listing only (readdir), not file contents.
    /// Uses Seatbelt `literal` filter - allows listing a directory's entries
    /// without granting access to files or subdirectories within it.
    pub allow_list_dirs: Vec<PathBuf>,
    /// Raw seatbelt rules to include verbatim
    pub raw_rules: Option<String>,
}

/// Generate a Seatbelt profile from the given parameters
///
/// Security model (deny-by-default):
/// - Reads: Denied by default, only explicit allow_read paths are accessible
/// - Writes: Denied by default, allow working dir + explicit paths
/// - Network: Configurable (offline/localhost/online)
///
/// # Errors
/// Returns `SeatbeltError::InvalidPath` if any path contains characters that could
/// break seatbelt syntax or inject additional rules.
pub fn generate_seatbelt_profile(params: &SandboxParams) -> Result<String, SeatbeltError> {
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
    profile.push_str("(allow file-ioctl)\n");
    profile.push_str("(allow user-preference-read)\n\n");

    // Mach services required for system functionality
    profile.push_str("; Mach services\n");
    profile.push_str("(allow mach*)\n\n");

    // IPC for Unix sockets (needed for DNS resolution via mDNSResponder)
    profile.push_str("; IPC (Unix sockets)\n");
    profile.push_str("(allow ipc-posix*)\n");
    profile.push_str("(allow system-socket)\n\n");

    // Allowed read paths (deny-by-default model)
    // Root directory literal is required for path traversal
    // file-read-metadata is needed globally for DNS resolution and path lookups
    profile.push_str("; Allowed read paths (deny-by-default)\n");
    profile.push_str("(allow file-read-metadata)\n");
    profile.push_str("(allow file-read* (literal \"/\"))\n");
    for path in &params.allow_read {
        let p = path.display().to_string();
        let validated = validate_seatbelt_path(&p)?;
        if contains_glob(validated) {
            let regex = glob_to_regex(validated);
            profile.push_str(&format!("(allow file-read* (regex #\"{regex}\"))\n"));
        } else {
            profile.push_str(&format!("(allow file-read* (subpath \"{validated}\"))\n"));
        }
    }
    profile.push('\n');

    // Directory listing only (readdir) - uses literal filter
    // Allows listing directory contents without reading files or subdirectories.
    // Useful for runtimes like Bun that scan parent directories during module resolution.
    if !params.allow_list_dirs.is_empty() {
        profile.push_str("; Directory listing only (readdir without file access)\n");
        for path in &params.allow_list_dirs {
            let p = path.display().to_string();
            let validated = validate_seatbelt_path(&p)?;
            // Use literal filter - only matches the exact path, not children
            profile.push_str(&format!(
                "(allow file-read-data (literal \"{validated}\"))\n"
            ));
        }
        profile.push('\n');
    }

    // Deny sensitive paths (overrides allow_read for nested sensitive paths)
    // Uses last-match-wins: deny after allow takes precedence
    if !params.deny_read.is_empty() {
        profile.push_str("; Denied read paths (sensitive data)\n");
        for path in &params.deny_read {
            let p = path.display().to_string();
            let validated = validate_seatbelt_path(&p)?;
            if contains_glob(validated) {
                let regex = glob_to_regex(validated);
                profile.push_str(&format!("(deny file-read* (regex #\"{regex}\"))\n"));
            } else {
                profile.push_str(&format!("(deny file-read* (subpath \"{validated}\"))\n"));
            }
        }
        profile.push('\n');
    }

    // Working directory - full read/write access
    profile.push_str("; Working directory (full access)\n");
    if !params.working_dir.as_os_str().is_empty() {
        let wd = params.working_dir.display().to_string();
        let validated_wd = validate_seatbelt_path(&wd)?;
        profile.push_str(&format!("(allow file* (subpath \"{validated_wd}\"))\n\n"));
    }

    // Allowed write paths (beyond working directory)
    if !params.allow_write.is_empty() {
        profile.push_str("; Allowed write paths\n");
        for path in &params.allow_write {
            let p = path.display().to_string();
            let validated = validate_seatbelt_path(&p)?;
            if contains_glob(validated) {
                // Glob patterns use regex filter
                let regex = glob_to_regex(validated);
                profile.push_str(&format!("(allow file-write* (regex #\"{regex}\"))\n"));
            } else if path.is_file() {
                // Use regex for files (to include lock files)
                let escaped = validated.replace('.', "\\.");
                profile.push_str(&format!("(allow file* (regex #\"^{escaped}.*\"))\n"));
            } else {
                profile.push_str(&format!("(allow file-write* (subpath \"{validated}\"))\n"));
            }
        }
        profile.push('\n');
    }

    // Device access (stdout, stderr, tty)
    profile.push_str("; Device access\n");
    profile.push_str("(allow file-write* (subpath \"/dev\"))\n");
    profile.push_str("(allow file-read* (subpath \"/dev\"))\n");
    // Pseudo-tty is required for interactive terminal features (backspace, arrow keys, etc.)
    profile.push_str("(allow pseudo-tty)\n\n");

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
            // Note: seatbelt only accepts "localhost" or "*" as host, not IP addresses
            profile.push_str("(allow network-outbound (to ip \"localhost:*\"))\n");
            profile.push_str("(allow network-inbound (from ip \"localhost:*\"))\n");
        }
    }

    // Raw rules if provided
    if let Some(raw) = &params.raw_rules {
        profile.push_str("\n; Custom rules\n");
        profile.push_str(raw);
        profile.push('\n');
    }

    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params_produces_valid_profile() {
        let params = SandboxParams::default();
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
    }

    // === Path Validation Tests ===

    #[test]
    fn test_validate_path_rejects_null_bytes() {
        let result = validate_seatbelt_path("/path/with\0null");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SeatbeltError::InvalidPath { reason, .. } if reason.contains("null")
        ));
    }

    #[test]
    fn test_validate_path_rejects_double_quotes() {
        let result = validate_seatbelt_path("/path/with\"quote");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SeatbeltError::InvalidPath { reason, .. } if reason.contains("quote")
        ));
    }

    #[test]
    fn test_validate_path_rejects_newlines() {
        let result = validate_seatbelt_path("/path/with\nnewline");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SeatbeltError::InvalidPath { reason, .. } if reason.contains("newline")
        ));
    }

    #[test]
    fn test_validate_path_accepts_valid_paths() {
        assert!(validate_seatbelt_path("/usr/bin").is_ok());
        assert!(validate_seatbelt_path("/Users/test/.config").is_ok());
        assert!(validate_seatbelt_path("/private/tmp/zsh*").is_ok());
        assert!(validate_seatbelt_path("~/.ssh").is_ok());
    }

    #[test]
    fn test_generate_profile_fails_on_invalid_path() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/path/with\"injection")],
            ..Default::default()
        };
        let result = generate_seatbelt_profile(&params);
        assert!(result.is_err());
    }

    // === Glob/Wildcard Detection Tests ===

    #[test]
    fn test_contains_glob_with_asterisk() {
        assert!(contains_glob("/private/tmp/zsh*"));
        assert!(contains_glob("/tmp/*.log"));
        assert!(contains_glob("*"));
    }

    #[test]
    fn test_contains_glob_with_question_mark() {
        assert!(contains_glob("/tmp/file?.txt"));
        assert!(contains_glob("?"));
    }

    #[test]
    fn test_contains_glob_without_wildcards() {
        assert!(!contains_glob("/private/tmp/zsh"));
        assert!(!contains_glob("/usr/bin"));
        assert!(!contains_glob("/home/user/.config"));
    }

    // === Glob to Regex Conversion Tests ===

    #[test]
    fn test_glob_to_regex_asterisk() {
        // /private/tmp/zsh* should match /private/tmp/zshXXXXXX
        let regex = glob_to_regex("/private/tmp/zsh*");
        assert_eq!(regex, r"^/private/tmp/zsh.*");
    }

    #[test]
    fn test_glob_to_regex_question_mark() {
        let regex = glob_to_regex("/tmp/file?.txt");
        assert_eq!(regex, r"^/tmp/file.\.txt");
    }

    #[test]
    fn test_glob_to_regex_escapes_dots() {
        let regex = glob_to_regex("/path/to/file.log");
        assert_eq!(regex, r"^/path/to/file\.log");
    }

    #[test]
    fn test_glob_to_regex_escapes_special_chars() {
        let regex = glob_to_regex("/path/with(parens)/file");
        assert_eq!(regex, r"^/path/with\(parens\)/file");
    }

    #[test]
    fn test_glob_to_regex_complex_pattern() {
        // /var/log/*.log should match /var/log/anything.log
        let regex = glob_to_regex("/var/log/*.log");
        assert_eq!(regex, r"^/var/log/.*\.log");
    }

    // === Seatbelt Profile Generation with Globs ===

    #[test]
    fn test_allow_read_glob_uses_regex() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/private/tmp/zsh*")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Should use regex filter, not subpath
        assert!(
            profile.contains(r#"(allow file-read* (regex #"^/private/tmp/zsh.*"))"#),
            "Glob pattern should generate regex rule, got:\n{}",
            profile
        );
        assert!(
            !profile.contains(r#"(subpath "/private/tmp/zsh*")"#),
            "Glob pattern should NOT use subpath filter"
        );
    }

    #[test]
    fn test_allow_read_non_glob_uses_subpath() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/private/tmp/claude")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Non-glob paths should still use subpath
        assert!(
            profile.contains(r#"(allow file-read* (subpath "/private/tmp/claude"))"#),
            "Non-glob path should use subpath filter"
        );
    }

    #[test]
    fn test_allow_write_glob_uses_regex() {
        let params = SandboxParams {
            allow_write: vec![PathBuf::from("/private/tmp/zsh*")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Should use regex filter for write as well
        assert!(
            profile.contains(r#"(allow file-write* (regex #"^/private/tmp/zsh.*"))"#),
            "Glob pattern should generate regex rule for writes, got:\n{}",
            profile
        );
    }

    #[test]
    fn test_deny_read_glob_uses_regex() {
        let params = SandboxParams {
            deny_read: vec![PathBuf::from("/home/*/.ssh")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Dot in .ssh is escaped to match literal dot, not any character
        assert!(
            profile.contains(r#"(deny file-read* (regex #"^/home/.*/\.ssh"))"#),
            "Glob pattern in deny should generate regex rule, got:\n{}",
            profile
        );
    }

    #[test]
    fn test_mixed_glob_and_regular_paths() {
        let params = SandboxParams {
            allow_read: vec![
                PathBuf::from("/usr"),
                PathBuf::from("/private/tmp/zsh*"),
                PathBuf::from("/bin"),
            ],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Regular paths use subpath
        assert!(profile.contains(r#"(allow file-read* (subpath "/usr"))"#));
        assert!(profile.contains(r#"(allow file-read* (subpath "/bin"))"#));
        // Glob path uses regex
        assert!(profile.contains(r#"(allow file-read* (regex #"^/private/tmp/zsh.*"))"#));
    }

    #[test]
    fn test_deny_by_default_no_global_read() {
        let params = SandboxParams::default();
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Should NOT have global read access - deny by default
        assert!(!profile.contains("(allow file-read* (subpath \"/\"))"));
    }

    #[test]
    fn test_allow_read_paths_included() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/usr"), PathBuf::from("/bin")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(allow file-read* (subpath \"/usr\"))"));
        assert!(profile.contains("(allow file-read* (subpath \"/bin\"))"));
    }

    #[test]
    fn test_deny_paths_included() {
        let params = SandboxParams {
            deny_read: vec![PathBuf::from("/secret")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(deny file-read* (subpath \"/secret\"))"));
    }

    #[test]
    fn test_deny_rules_come_after_allow_read() {
        let params = SandboxParams {
            allow_read: vec![PathBuf::from("/home")],
            deny_read: vec![PathBuf::from("/home/.ssh")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();

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
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(allow file* (subpath \"/projects/myapp\"))"));
    }

    #[test]
    fn test_network_offline() {
        let params = SandboxParams {
            network_mode: NetworkMode::Offline,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("Network disabled"));
        assert!(!profile.contains("(allow network"));
    }

    #[test]
    fn test_network_online() {
        let params = SandboxParams {
            network_mode: NetworkMode::Online,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(allow network*)"));
    }

    #[test]
    fn test_network_localhost() {
        let params = SandboxParams {
            network_mode: NetworkMode::Localhost,
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("(allow network-outbound (to ip \"localhost:*\"))"));
        assert!(profile.contains("(allow network-inbound (from ip \"localhost:*\"))"));
        // seatbelt doesn't accept IP addresses, only "localhost" or "*"
        assert!(!profile.contains("127.0.0.1"));
    }

    // === Directory Listing (allow_list_dirs) Tests ===

    #[test]
    fn test_allow_list_dirs_uses_literal() {
        let params = SandboxParams {
            allow_list_dirs: vec![PathBuf::from("/Users")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        // Should use literal filter (exact match only), not subpath
        assert!(
            profile.contains(r#"(allow file-read-data (literal "/Users"))"#),
            "allow_list_dirs should use literal filter, got:\n{}",
            profile
        );
        // Should NOT use subpath (which would allow reading all contents)
        assert!(
            !profile.contains(r#"(allow file-read* (subpath "/Users"))"#),
            "allow_list_dirs should NOT use subpath filter"
        );
    }

    #[test]
    fn test_allow_list_dirs_multiple_paths() {
        let params = SandboxParams {
            allow_list_dirs: vec![PathBuf::from("/Users"), PathBuf::from("/Users/testuser")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains(r#"(allow file-read-data (literal "/Users"))"#));
        assert!(profile.contains(r#"(allow file-read-data (literal "/Users/testuser"))"#));
    }

    #[test]
    fn test_allow_list_dirs_with_deny_read() {
        // Verify deny_read still takes precedence over allow_list_dirs
        let params = SandboxParams {
            allow_list_dirs: vec![PathBuf::from("/Users/testuser")],
            deny_read: vec![PathBuf::from("/Users/testuser/secret")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();

        let list_pos = profile
            .find(r#"(allow file-read-data (literal "/Users/testuser"))"#)
            .expect("allow_list_dirs rule should exist");
        let deny_pos = profile
            .find(r#"(deny file-read* (subpath "/Users/testuser/secret"))"#)
            .expect("deny rule should exist");

        // deny_read comes after allow_list_dirs (last-match-wins)
        assert!(
            deny_pos > list_pos,
            "deny rules must come after allow_list_dirs for Seatbelt last-match-wins semantics"
        );
    }

    #[test]
    fn test_allow_list_dirs_section_comment() {
        let params = SandboxParams {
            allow_list_dirs: vec![PathBuf::from("/Users")],
            ..Default::default()
        };
        let profile = generate_seatbelt_profile(&params).unwrap();
        assert!(profile.contains("; Directory listing only"));
    }
}
