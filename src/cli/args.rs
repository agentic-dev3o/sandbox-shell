use clap::Parser;
use std::path::PathBuf;

/// Network access mode for the sandbox
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NetworkMode {
    /// Block all network access (default)
    #[default]
    Offline,
    /// Allow all network access
    Online,
    /// Allow localhost only
    Localhost,
}

/// sx - Lightweight sandbox for macOS development
///
/// Wraps shell sessions and commands in a macOS Seatbelt sandbox,
/// restricting filesystem and network access to protect the user's system.
#[derive(Parser, Debug)]
#[command(name = "sx")]
#[command(author, version, about, long_about = None)]
#[command(after_help = "PROFILES:\n    \
    base        Minimal sandbox (always included)\n    \
    online      Full network access\n    \
    localhost   Localhost network only\n    \
    node        Node.js/npm toolchain\n    \
    python      Python toolchain\n    \
    rust        Rust/Cargo toolchain\n    \
    go          Go toolchain\n    \
    claude      Claude Code (~/.claude access)\n    \
    gpg         GPG signing support\n    \
    git         Git with signing support")]
pub struct Args {
    /// Enable verbose output (show sandbox config)
    #[arg(short, long)]
    pub verbose: bool,

    /// Enable debug mode (log all denials)
    #[arg(short, long)]
    pub debug: bool,

    /// Print generated sandbox profile without executing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Use specific config file
    #[arg(short, long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Ignore all config files
    #[arg(long)]
    pub no_config: bool,

    /// Initialize .sandbox.toml in current directory
    #[arg(long)]
    pub init: bool,

    /// Show what would be allowed/denied
    #[arg(long)]
    pub explain: bool,

    /// Block all network (default)
    #[arg(long, group = "network")]
    pub offline: bool,

    /// Allow all network
    #[arg(long, group = "network")]
    pub online: bool,

    /// Allow localhost only
    #[arg(long, group = "network")]
    pub localhost: bool,

    /// Allow specific domain (can repeat)
    #[arg(long = "allow-domain", value_name = "DOMAIN")]
    pub allow_domains: Vec<String>,

    /// Allow read access to path
    #[arg(long = "allow-read", value_name = "PATH")]
    pub allow_read: Vec<String>,

    /// Allow write access to path
    #[arg(long = "allow-write", value_name = "PATH")]
    pub allow_write: Vec<String>,

    /// Deny read access to path (override allows)
    #[arg(long = "deny-read", value_name = "PATH")]
    pub deny_read: Vec<String>,

    /// Profiles to apply (e.g., online, node, claude)
    #[arg(value_name = "PROFILES")]
    pub profiles: Vec<String>,

    /// Command to run in sandbox (after --)
    #[arg(last = true, value_name = "COMMAND")]
    pub command: Option<Vec<String>>,
}

impl Args {
    /// Parse arguments from command line
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Try to parse from an iterator (for testing)
    pub fn try_parse_from<I, T>(iter: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        <Self as Parser>::try_parse_from(iter)
    }

    /// Determine the network mode from flags
    pub fn network_mode(&self) -> NetworkMode {
        if self.online {
            NetworkMode::Online
        } else if self.localhost {
            NetworkMode::Localhost
        } else {
            NetworkMode::Offline
        }
    }
}
