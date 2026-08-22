//! Sandbox launcher invocation.
//!
//! Platform-neutral: process supervision (signal forwarding, process groups,
//! terminal foreground handling) and environment filtering live here, while the
//! actual confinement mechanism comes from [`crate::sandbox::backend`].
use crate::sandbox::backend::{self, PolicyError};
use crate::sandbox::params::SandboxParams;
use crate::sandbox::trace::TraceSession;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::fs;
use std::io;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::time::Duration;
use tempfile::TempDir;

/// Grace period between SIGTERM and SIGKILL when forwarding shutdown signals
/// to the sandboxed process group. Long enough for typical cleanup (closing
/// IPC peers, flushing buffers) but short enough that orphaned processes do
/// not linger after `sx` is signalled.
const SIGTERM_TO_SIGKILL_GRACE: Duration = Duration::from_secs(2);

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

/// Error type for sandbox execution
#[derive(Debug)]
pub enum ExecutionError {
    /// IO error during execution
    Io(io::Error),
    /// Sandbox policy could not be built
    Policy(PolicyError),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::Io(e) => write!(f, "IO error: {}", e),
            ExecutionError::Policy(e) => write!(f, "Sandbox policy error: {}", e),
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ExecutionError::Io(e) => Some(e),
            ExecutionError::Policy(e) => Some(e),
        }
    }
}

impl From<io::Error> for ExecutionError {
    fn from(e: io::Error) -> Self {
        ExecutionError::Io(e)
    }
}

impl From<PolicyError> for ExecutionError {
    fn from(e: PolicyError) -> Self {
        ExecutionError::Policy(e)
    }
}

/// Result of sandbox execution
#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
}

/// Execute a command inside a sandbox
pub fn execute_sandboxed(
    params: &SandboxParams,
    command: &[String],
    shell: Option<&str>,
) -> Result<ExecutionResult, ExecutionError> {
    execute_sandboxed_with_trace(params, command, shell, false, None)
}

/// Start a violation trace, if this platform can observe them.
///
/// macOS exposes sandbox denials to unprivileged users through the unified log.
/// Linux has no equivalent: Landlock denials are only recorded through the
/// kernel audit subsystem (Linux 6.15+), which needs `CAP_AUDIT_READ` to read,
/// so `--trace` reports the limitation instead of silently doing nothing.
#[cfg(target_os = "macos")]
fn start_trace(trace: bool, trace_file: Option<&Path>) -> Option<TraceSession> {
    if !trace && trace_file.is_none() {
        return None;
    }
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
}

#[cfg(not(target_os = "macos"))]
fn start_trace(trace: bool, trace_file: Option<&Path>) -> Option<TraceSession> {
    if trace || trace_file.is_some() {
        eprintln!(
            "\x1b[33m[sx:trace]\x1b[0m Violation tracing is unavailable on Linux: Landlock \
             denials are only visible through the kernel audit log, which requires privileges. \
             Use `sx --dry-run` to see the exact policy, or `sx --explain` for the resolved paths."
        );
    }
    None
}

/// Execute a command inside a sandbox with optional tracing
pub fn execute_sandboxed_with_trace(
    params: &SandboxParams,
    command: &[String],
    shell: Option<&str>,
    trace: bool,
    trace_file: Option<&Path>,
) -> Result<ExecutionResult, ExecutionError> {
    let mut trace_session = start_trace(trace, trace_file);

    // Build the platform sandbox launcher. `launch` owns any policy file and
    // must outlive the child that reads it.
    let launch = backend::prepare(params)?;
    let mut cmd = Command::new(&launch.program);
    cmd.args(&launch.args);

    // Apply environment filtering (clears env, then selectively passes through)
    apply_env_filter(&mut cmd, params);

    // Launcher variables go on after filtering so they are not stripped.
    for (key, value) in &launch.launcher_env {
        cmd.env(key, value);
    }

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
            .unwrap_or_else(|| crate::shell::default_shell().to_string());
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
            if let Err(e) = fs::write(dir.path().join(".zshrc"), zshrc_content) {
                eprintln!(
                    "\x1b[33m[sx:warn]\x1b[0m Failed to disable zsh history: {}",
                    e
                );
            }
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

    // Execute and wait, forwarding shutdown signals to the entire sandboxed subtree.
    // Without this, sx exits without signalling its sandboxed descendants — IPC
    // children (e.g. Node `--useNodeIpc` workers) get reparented to launchd and
    // accumulate forever.
    let status = spawn_with_signal_forwarding(cmd)?;

    // Stop trace session
    if let Some(ref mut session) = trace_session {
        // Small delay to capture any final violations
        std::thread::sleep(std::time::Duration::from_millis(100));
        session.stop();
    }

    let exit_code = status.code().unwrap_or(exit_codes::GENERAL_ERROR);

    Ok(ExecutionResult { exit_code })
}

/// Execute a command in sandbox and capture output (for non-interactive use)
pub fn execute_sandboxed_captured(
    params: &SandboxParams,
    command: &[String],
) -> Result<(ExitStatus, Vec<u8>, Vec<u8>), ExecutionError> {
    let launch = backend::prepare(params)?;
    let mut cmd = Command::new(&launch.program);
    cmd.args(&launch.args);

    // Apply environment filtering
    apply_env_filter(&mut cmd, params);
    for (key, value) in &launch.launcher_env {
        cmd.env(key, value);
    }

    cmd.args(command);

    let output = cmd.output()?;

    Ok((output.status, output.stdout, output.stderr))
}

/// Render the sandbox policy without executing anything (dry-run mode)
pub fn dry_run(params: &SandboxParams) -> Result<String, PolicyError> {
    backend::render_policy(params)
}

/// RAII guard that SIGKILLs an entire process group on drop.
/// Provides panic / early-return safety so the sandbox subtree is never
/// orphaned if `sx` exits along an unexpected path.
struct PgidKillGuard {
    pgid: Option<i32>,
}

impl PgidKillGuard {
    fn new(pgid: i32) -> Self {
        Self { pgid: Some(pgid) }
    }

    /// Disarm the guard after a clean exit so we do not signal a dead pgid.
    fn disarm(&mut self) {
        self.pgid = None;
    }
}

impl Drop for PgidKillGuard {
    fn drop(&mut self) {
        if let Some(pgid) = self.pgid {
            // Best-effort: ignore errors (group may already be gone).
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
        }
    }
}

/// Send `sig` to the entire process group identified by `pgid`.
fn signal_pgroup(pgid: i32, sig: libc::c_int) {
    unsafe {
        libc::kill(-pgid, sig);
    }
}

/// RAII guard that restores the terminal's foreground process group on drop.
/// Required for interactive sessions: when the child runs in its own pgrp,
/// it must own the tty foreground or any tty read raises SIGTTIN and the
/// shell hangs. The parent must reclaim the foreground on exit so the
/// user's shell does not end up stopped.
struct TtyForegroundGuard {
    tty_fd: libc::c_int,
    original_pgrp: libc::pid_t,
}

impl TtyForegroundGuard {
    /// If stdin is a tty, hand its foreground pgrp to `child_pgid` and
    /// return a guard that restores the previous foreground on drop.
    fn install(child_pgid: i32) -> Option<Self> {
        let tty_fd = libc::STDIN_FILENO;
        if unsafe { libc::isatty(tty_fd) } != 1 {
            return None;
        }
        let original_pgrp = unsafe { libc::tcgetpgrp(tty_fd) };
        if original_pgrp < 0 {
            return None;
        }
        // tcsetpgrp from a non-foreground pgrp would normally raise SIGTTOU
        // on the caller; ignore it briefly so the handoff succeeds.
        let prev = unsafe { libc::signal(libc::SIGTTOU, libc::SIG_IGN) };
        let rc = unsafe { libc::tcsetpgrp(tty_fd, child_pgid) };
        unsafe { libc::signal(libc::SIGTTOU, prev) };
        if rc != 0 {
            return None;
        }
        Some(Self {
            tty_fd,
            original_pgrp,
        })
    }
}

impl Drop for TtyForegroundGuard {
    fn drop(&mut self) {
        let prev = unsafe { libc::signal(libc::SIGTTOU, libc::SIG_IGN) };
        unsafe {
            libc::tcsetpgrp(self.tty_fd, self.original_pgrp);
            libc::signal(libc::SIGTTOU, prev);
        }
    }
}

/// Spawn `cmd` in its own process group and wait for it, forwarding
/// SIGINT/SIGTERM/SIGHUP to the sandboxed subtree with a SIGTERM →
/// grace-period → SIGKILL escalation.
fn spawn_with_signal_forwarding(mut cmd: Command) -> io::Result<ExitStatus> {
    // Put the child in its own process group so kill(-pgid, ...) reaches every
    // descendant in one syscall and does not loop back to sx itself.
    cmd.process_group(0);

    let mut child = cmd.spawn()?;
    let pgid = child.id() as i32;
    let mut kill_guard = PgidKillGuard::new(pgid);
    let _tty_guard = TtyForegroundGuard::install(pgid);

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP])?;
    let signal_handle = signals.handle();

    let signal_thread = std::thread::spawn(move || {
        if signals.forever().next().is_some() {
            signal_pgroup(pgid, libc::SIGTERM);
            std::thread::sleep(SIGTERM_TO_SIGKILL_GRACE);
            signal_pgroup(pgid, libc::SIGKILL);
        }
    });

    let status = child.wait()?;

    kill_guard.disarm();
    signal_handle.close();
    let _ = signal_thread.join();

    Ok(status)
}

/// Check if an environment variable name matches any glob-like pattern.
/// Supports `*` wildcards: `AWS_*` matches `AWS_SECRET_KEY`, `*_SECRET*` matches `MY_SECRET_VALUE`.
fn matches_env_pattern(name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if pattern == name {
            return true;
        }
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            let mut pos = 0;
            let mut matched = true;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                if i == 0 {
                    if !name.starts_with(part) {
                        matched = false;
                        break;
                    }
                    pos = part.len();
                } else if let Some(found) = name[pos..].find(part) {
                    pos += found + part.len();
                } else {
                    matched = false;
                    break;
                }
            }
            if matched && !pattern.ends_with('*') {
                if let Some(last) = parts.last() {
                    if !last.is_empty() && !name.ends_with(last) {
                        matched = false;
                    }
                }
            }
            if matched {
                return true;
            }
        }
    }
    false
}

/// Variables the dynamic loader uses to inject code into a process.
///
/// These are dropped unconditionally - they are never forwarded and cannot be
/// re-added through `set_env`. The sandbox launcher is itself a process, so a
/// preloaded library would run inside it *before* the policy is applied and
/// could stop the sandbox from being enforced at all. `pass_env` normally
/// filters them out as an allow-list, but it is empty when a project sets
/// `inherit_base = false`, which is exactly when this matters.
fn is_loader_injection_var(key: &str) -> bool {
    // DYLD_* is the macOS loader, LD_* the glibc/musl one. Both lists are
    // applied everywhere so behaviour does not depend on the build target.
    key.starts_with("DYLD_") || key.starts_with("LD_")
}

/// Apply environment filtering to a Command.
/// Clears all env, then selectively passes through allowed vars.
fn apply_env_filter(cmd: &mut Command, params: &SandboxParams) {
    cmd.env_clear();

    let parent_env: std::collections::HashMap<String, String> = std::env::vars().collect();

    for (key, value) in &parent_env {
        if is_loader_injection_var(key) {
            continue;
        }
        if matches_env_pattern(key, &params.deny_env) {
            continue;
        }
        if params.pass_env.is_empty() || matches_env_pattern(key, &params.pass_env) {
            cmd.env(key, value);
        }
    }

    for (key, value) in &params.set_env {
        if is_loader_injection_var(key) {
            continue;
        }
        if matches_env_pattern(key, &params.deny_env) {
            continue;
        }
        cmd.env(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::NetworkMode;
    use std::path::PathBuf;

    #[test]
    fn test_dry_run_returns_policy() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/tmp/test"),
            home_dir: PathBuf::from("/tmp/home"),
            network_mode: NetworkMode::Offline,
            ..Default::default()
        };

        let policy = dry_run(&params).unwrap();

        #[cfg(target_os = "macos")]
        {
            assert!(policy.contains("(version 1)"));
            assert!(policy.contains("(deny default)"));
        }
        #[cfg(target_os = "linux")]
        {
            assert!(policy.contains("landlock"));
            assert!(policy.contains("/tmp/test"));
        }
    }

    /// A Seatbelt profile is text, so an unescaped quote in a path could inject
    /// rules and must be rejected before it reaches `sandbox-exec`.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_dry_run_fails_on_invalid_path() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/tmp/test\"injection"),
            ..Default::default()
        };

        let result = dry_run(&params);
        assert!(result.is_err());
    }

    /// Landlock takes paths as opaque bytes through open(2) - there is no
    /// policy text to escape from, so odd characters are simply path bytes.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_dry_run_treats_quotes_as_ordinary_path_bytes() {
        let params = SandboxParams {
            working_dir: PathBuf::from("/tmp/test\"injection"),
            ..Default::default()
        };

        let policy = dry_run(&params).unwrap();
        assert!(policy.contains("/tmp/test\"injection"));
    }

    #[test]
    fn loader_injection_vars_are_always_dropped() {
        assert!(is_loader_injection_var("LD_PRELOAD"));
        assert!(is_loader_injection_var("LD_LIBRARY_PATH"));
        assert!(is_loader_injection_var("LD_AUDIT"));
        assert!(is_loader_injection_var("DYLD_INSERT_LIBRARIES"));
        assert!(is_loader_injection_var("DYLD_LIBRARY_PATH"));

        // Ordinary variables that merely start with the same letters stay.
        assert!(!is_loader_injection_var("LDFLAGS"));
        assert!(!is_loader_injection_var("PATH"));
        assert!(!is_loader_injection_var("HOME"));
    }

    #[test]
    fn test_matches_env_pattern_exact() {
        assert!(matches_env_pattern("HOME", &["HOME".to_string()]));
        assert!(!matches_env_pattern("HOME", &["PATH".to_string()]));
    }

    #[test]
    fn test_matches_env_pattern_prefix_wildcard() {
        assert!(matches_env_pattern(
            "AWS_SECRET_KEY",
            &["AWS_*".to_string()]
        ));
        assert!(!matches_env_pattern("HOME", &["AWS_*".to_string()]));
    }

    #[test]
    fn test_matches_env_pattern_contains_wildcard() {
        assert!(matches_env_pattern(
            "MY_SECRET_VALUE",
            &["*_SECRET*".to_string()]
        ));
        assert!(!matches_env_pattern("MY_VALUE", &["*_SECRET*".to_string()]));
    }

    #[test]
    fn test_matches_env_pattern_suffix_wildcard() {
        assert!(matches_env_pattern("GITHUB_KEY", &["*_KEY".to_string()]));
        assert!(!matches_env_pattern(
            "GITHUB_KEY_EXTRA",
            &["*_KEY".to_string()]
        ));
    }

    #[test]
    fn test_matches_env_pattern_dyld_always_dangerous() {
        assert!(matches_env_pattern(
            "DYLD_INSERT_LIBRARIES",
            &["DYLD_*".to_string()]
        ));
    }

    /// Verifies the foundational mechanism for issue #37: spawning a child via
    /// `Command::process_group(0)` isolates it into its own process group so we
    /// can deliver group-wide signals via `kill(-pgid, ...)`. This is a direct
    /// unit test of the kernel behavior `spawn_with_signal_forwarding` relies
    /// on; it does not require `sandbox-exec` and runs in any environment.
    #[test]
    fn test_process_group_isolation_for_signal_forwarding() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "sleep 5"])
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().expect("spawn /bin/sh");
        let child_pid = child.id() as i32;

        let child_pgid = unsafe { libc::getpgid(child_pid) };
        assert_eq!(
            child_pgid, child_pid,
            "child should lead its own process group (pgid == pid)"
        );

        let test_pgid = unsafe { libc::getpgid(0) };
        assert_ne!(
            test_pgid, child_pgid,
            "child pgid must be isolated from test process pgid"
        );

        // Cleanup: signal the entire group, mirroring what the production
        // signal handler does, then reap the child.
        unsafe {
            libc::kill(-child_pid, libc::SIGTERM);
        }
        let _ = child.wait();
    }
}
