//! Platform-neutral sandbox parameters.
//!
//! `SandboxParams` is the resolved intermediate representation produced by the
//! config/profile layer and consumed by a platform backend (Seatbelt on macOS,
//! Landlock on Linux). It is serialisable so the Linux backend can hand it to
//! the sandbox helper process.

use crate::config::schema::{ExecSugid, NetworkMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parameters for generating a sandbox policy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxParams {
    /// Working directory (project root) - gets full read/write access
    pub working_dir: PathBuf,
    /// Home directory
    pub home_dir: PathBuf,
    /// Network mode (offline, online, localhost)
    pub network_mode: NetworkMode,
    /// Paths to allow reading (deny-by-default, only these paths are accessible)
    pub allow_read: Vec<PathBuf>,
    /// Paths to explicitly deny reading (overrides allow_read, for sensitive subpaths)
    pub deny_read: Vec<PathBuf>,
    /// Paths to allow writing (restricted by default)
    pub allow_write: Vec<PathBuf>,
    /// Paths to allow directory listing only (readdir), not file contents.
    ///
    /// On macOS this uses the Seatbelt `literal` filter - only the exact
    /// directory is listable, not its children. On Linux, Landlock rules are
    /// always hierarchical, so nested directories become listable too (names
    /// only; file contents stay denied).
    pub allow_list_dirs: Vec<PathBuf>,
    /// Raw seatbelt rules to include verbatim (macOS only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_rules: Option<String>,
    /// Allow execution of setuid/setgid binaries
    pub allow_exec_sugid: ExecSugid,
    /// Environment variables to pass through (glob patterns supported)
    pub pass_env: Vec<String>,
    /// Environment variables to deny (glob patterns, takes precedence over pass_env)
    pub deny_env: Vec<String>,
    /// Environment variables to explicitly set
    pub set_env: std::collections::HashMap<String, String>,
}
