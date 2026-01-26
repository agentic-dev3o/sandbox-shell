// sandbox-exec invocation
use crate::sandbox::seatbelt::{generate_seatbelt_profile, SandboxParams};
use crate::sandbox::trace::TraceSession;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use tempfile::{NamedTempFile, TempDir};
use uuid::Uuid;

/// Exit codes for sandbox execution
pub mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const GENERAL_ERROR: i32 = 1;
    pub const CONFIG_ERROR: i32 = 2;
    pub const COMMAND_NOT_EXECUTABLE: i32 = 126;
    pub const COMMAND_NOT_FOUND: i32 = 127;
    pub const INTERRUPTED: i32 = 130;
    pub const SANDBOX_VIOLATION: i32 = 137;
}

/// Result of sandbox execution
#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub profile_path: Option<PathBuf>,
}

/// Execute a command inside a sandbox
pub fn execute_sandboxed(
    params: &SandboxParams,
    command: &[String],
    shell: Option<&str>,
) -> io::Result<ExecutionResult> {
    execute_sandboxed_with_trace(params, command, shell, false, None)
}

/// Execute a command inside a sandbox with optional tracing
pub fn execute_sandboxed_with_trace(
    params: &SandboxParams,
    command: &[String],
    shell: Option<&str>,
    trace: bool,
    trace_file: Option<&Path>,
) -> io::Result<ExecutionResult> {
    // Start trace session if requested
    let mut trace_session = if trace || trace_file.is_some() {
        if let Some(path) = trace_file {
            eprintln!(
                "\x1b[90m[sx:trace]\x1b[0m Writing sandbox violations to {}",
                path.display()
            );
            std::thread::sleep(std::time::Duration::from_millis(100));
            TraceSession::start_to_file(path).ok()
        } else {
            eprintln!("\x1b[90m[sx:trace]\x1b[0m Starting sandbox violation trace...");
            std::thread::sleep(std::time::Duration::from_millis(100));
            TraceSession::start().ok()
        }
    } else {
        None
    };

    // Generate the seatbelt profile
    let profile_content = generate_seatbelt_profile(params);

    // Write profile to temp file
    let profile_file = NamedTempFile::new()?;
    fs::write(profile_file.path(), &profile_content)?;

    // Build sandbox-exec command
    let mut cmd = Command::new("/usr/bin/sandbox-exec");
    cmd.arg("-f").arg(profile_file.path());

    // Set SANDBOX_MODE environment variable for shell prompt integration
    let mode_str = match params.network_mode {
        crate::config::schema::NetworkMode::Offline => "offline",
        crate::config::schema::NetworkMode::Online => "online",
        crate::config::schema::NetworkMode::Localhost => "localhost",
    };
    cmd.env("SANDBOX_MODE", mode_str);

    // If command is empty, spawn interactive shell
    let is_interactive_shell = command.is_empty();
    if is_interactive_shell {
        let shell_path = shell
            .map(String::from)
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".to_string());
        cmd.arg(&shell_path);
    } else {
        // Execute the provided command
        cmd.args(command);
    }

    // Disable shell history for security (prevents secrets leaking into history files)
    // We use ZDOTDIR for zsh to ensure history is disabled AFTER .zshrc runs
    let _zdotdir: Option<TempDir> = if is_interactive_shell {
        // Fish: disable history
        cmd.env("fish_history", "");

        // Bash: set env vars (bash respects these even if .bashrc sets HISTFILE)
        cmd.env("HISTFILE", "/dev/null");
        cmd.env("HISTSIZE", "0");
        cmd.env("HISTFILESIZE", "0");

        // Zsh: use ZDOTDIR to create a wrapper that disables history after sourcing user config
        let zdotdir = TempDir::new().ok();
        if let Some(ref dir) = zdotdir {
            let zshrc_content = r#"# sx sandbox wrapper - sources user config then disables history
[[ -f ~/.zshrc ]] && source ~/.zshrc
HISTFILE=/dev/null
HISTSIZE=0
SAVEHIST=0
"#;
            let _ = fs::write(dir.path().join(".zshrc"), zshrc_content);
            cmd.env("ZDOTDIR", dir.path());
        }
        zdotdir
    } else {
        None
    };

    // Inherit stdio for interactive use
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Execute and wait
    let status = cmd.spawn()?.wait()?;

    // Stop trace session
    if let Some(ref mut session) = trace_session {
        // Small delay to capture any final violations
        std::thread::sleep(std::time::Duration::from_millis(100));
        session.stop();
    }

    let exit_code = status.code().unwrap_or(exit_codes::GENERAL_ERROR);

    Ok(ExecutionResult {
        exit_code,
        profile_path: Some(profile_file.path().to_path_buf()),
    })
}

/// Execute a command in sandbox and capture output (for non-interactive use)
pub fn execute_sandboxed_captured(
    params: &SandboxParams,
    command: &[String],
) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    // Generate the seatbelt profile
    let profile_content = generate_seatbelt_profile(params);

    // Write profile to temp file
    let profile_file = NamedTempFile::new()?;
    fs::write(profile_file.path(), &profile_content)?;

    // Build sandbox-exec command
    let mut cmd = Command::new("/usr/bin/sandbox-exec");
    cmd.arg("-f").arg(profile_file.path());
    cmd.args(command);

    let output = cmd.output()?;

    Ok((output.status, output.stdout, output.stderr))
}

/// Print the generated seatbelt profile (dry-run mode)
pub fn dry_run(params: &SandboxParams) -> String {
    generate_seatbelt_profile(params)
}

/// Create a unique temp file path for the profile
pub fn temp_profile_path() -> PathBuf {
    let uuid = Uuid::new_v4();
    std::env::temp_dir().join(format!("sx-{}.sx", uuid))
}

/// Write profile to a file and return the path
pub fn write_profile_file(profile_content: &str) -> io::Result<PathBuf> {
    let path = temp_profile_path();
    fs::write(&path, profile_content)?;
    Ok(path)
}

/// Clean up a profile file
pub fn cleanup_profile(path: &PathBuf) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::NetworkMode;

    #[test]
    fn test_dry_run_returns_profile() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/tmp/test"),
            home_dir: PathBuf::from("/Users/test"),
            network_mode: NetworkMode::Offline,
            ..Default::default()
        };

        let profile = dry_run(&params);
        assert!(profile.contains("(version 1)"));
        assert!(profile.contains("(deny default)"));
    }

    #[test]
    fn test_temp_profile_path_is_unique() {
        let path1 = temp_profile_path();
        let path2 = temp_profile_path();
        assert_ne!(path1, path2);
    }

    #[test]
    fn test_write_and_cleanup_profile() {
        let content = "(version 1)\n(deny default)\n";
        let path = write_profile_file(content).unwrap();
        assert!(path.exists());

        cleanup_profile(&path).unwrap();
        assert!(!path.exists());
    }
}
