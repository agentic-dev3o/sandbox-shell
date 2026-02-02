//! CLI command implementations
//!
//! Wires together config loading, profile composition, and sandbox execution.

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

use super::args::Args;
use crate::config::project::{load_project_config, PROJECT_CONFIG_NAME};
use crate::config::{
    compose_profiles, load_global_config, load_profiles, merge_configs, Config, NetworkMode,
    Profile,
};
use crate::detection::project_type::detect_project_types;
use crate::sandbox::executor::execute_sandboxed_with_trace;
use crate::sandbox::seatbelt::{generate_seatbelt_profile, SandboxParams};
use crate::utils::paths::expand_paths;

/// Initialize a .sandbox.toml config in the current directory
pub fn init_config() -> Result<()> {
    let config_path = env::current_dir()?.join(PROJECT_CONFIG_NAME);

    if config_path.exists() {
        anyhow::bail!("{} already exists in this directory", PROJECT_CONFIG_NAME);
    }

    let template = generate_config_template();
    std::fs::write(&config_path, template)
        .with_context(|| format!("Failed to write {}", PROJECT_CONFIG_NAME))?;

    println!("Created {}", config_path.display());
    println!("Edit this file to customize sandbox settings for this project.");
    Ok(())
}

/// Show what would be allowed/denied
pub fn explain(args: &Args) -> Result<()> {
    let context = build_sandbox_context(args)?;

    println!("=== Sandbox Configuration ===\n");

    // Network mode
    println!("Network Mode: {:?}", context.params.network_mode);
    println!();

    // Working directory
    println!("Working Directory (full access):");
    println!("  {}", context.params.working_dir.display());
    println!();

    // Allowed read paths
    if !context.params.allow_read.is_empty() {
        println!("Allowed Read Paths:");
        for path in &context.params.allow_read {
            println!("  + {}", path.display());
        }
        println!();
    }

    // Denied read paths
    if !context.params.deny_read.is_empty() {
        println!("Denied Read Paths:");
        for path in &context.params.deny_read {
            println!("  - {}", path.display());
        }
        println!();
    }

    // Directory listing only paths
    if !context.params.allow_list_dirs.is_empty() {
        println!("Directory Listing Only (readdir without file access):");
        for path in &context.params.allow_list_dirs {
            println!("  ~ {}", path.display());
        }
        println!();
    }

    // Allowed write paths
    if !context.params.allow_write.is_empty() {
        println!("Allowed Write Paths:");
        for path in &context.params.allow_write {
            println!("  + {}", path.display());
        }
        println!();
    }

    // Profiles applied
    if !context.profile_names.is_empty() {
        println!("Profiles Applied:");
        for name in &context.profile_names {
            println!("  - {}", name);
        }
        println!();
    }

    // Command
    if let Some(cmd) = &args.command {
        if !cmd.is_empty() {
            println!("Command: {}", cmd.join(" "));
        }
    } else {
        let shell = context
            .config
            .sandbox
            .shell
            .clone()
            .or_else(|| env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".to_string());
        println!("Mode: Interactive shell ({})", shell);
    }

    Ok(())
}

/// Print generated sandbox profile without executing
pub fn dry_run(args: &Args) -> Result<()> {
    let context = build_sandbox_context(args)?;
    let profile = generate_seatbelt_profile(&context.params)
        .context("Failed to generate seatbelt profile")?;

    if args.verbose {
        println!("# Profiles: {}", context.profile_names.join(", "));
        println!("# Network: {:?}", context.params.network_mode);
        println!("# Working dir: {}", context.params.working_dir.display());
        println!();
    }

    println!("{}", profile);
    Ok(())
}

/// Execute the sandbox with the given configuration
pub fn execute(args: &Args) -> Result<()> {
    let context = build_sandbox_context(args)?;

    if args.verbose {
        eprintln!("[sx] Network: {:?}", context.params.network_mode);
        eprintln!("[sx] Profiles: {}", context.profile_names.join(", "));
        eprintln!("[sx] Working dir: {}", context.params.working_dir.display());
    }

    let command: Vec<String> = args.command.clone().unwrap_or_default();
    let shell = context.config.sandbox.shell.as_deref();

    let result = execute_sandboxed_with_trace(
        &context.params,
        &command,
        shell,
        args.trace,
        args.trace_file.as_deref(),
    )
    .context("Failed to execute sandboxed command")?;

    std::process::exit(result.exit_code);
}

// --- Internal Implementation ---

/// Context built from args, config, and profiles
struct SandboxContext {
    params: SandboxParams,
    config: Config,
    profile_names: Vec<String>,
}

/// Build the complete sandbox context from CLI args
fn build_sandbox_context(args: &Args) -> Result<SandboxContext> {
    let working_dir = env::current_dir().context("Failed to get current directory")?;
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

    // Load and merge configurations
    let config = load_effective_config(args, &working_dir)?;

    // Collect all profile names to load
    let profile_names = collect_profile_names(args, &config, &working_dir);

    // Load and compose profiles
    let profiles = load_profiles(&profile_names, None);
    let composed = compose_profiles(&profiles);

    // Build sandbox params with all overrides applied
    let params = build_sandbox_params(args, &config, &composed, working_dir, home_dir);

    Ok(SandboxContext {
        params,
        config,
        profile_names,
    })
}

/// Load effective configuration by merging global and project configs
fn load_effective_config(args: &Args, working_dir: &Path) -> Result<Config> {
    if args.no_config {
        return Ok(Config::default());
    }

    // Load global config
    let global =
        load_global_config(args.config.as_deref()).context("Failed to load global config")?;

    // Load project config
    let project = load_project_config(working_dir).context("Failed to load project config")?;

    // Merge if project config exists and inherits
    match project {
        Some(proj) if proj.sandbox.inherit_global => Ok(merge_configs(&global, &proj)),
        Some(proj) => Ok(proj),
        None => Ok(global),
    }
}

/// Collect all profile names to apply
fn collect_profile_names(args: &Args, config: &Config, working_dir: &Path) -> Vec<String> {
    let mut names = Vec::new();

    // Start with base unless inherit_base is false
    if config.sandbox.inherit_base {
        names.push("base".to_string());
    }

    // Add default profiles from config (skip base if inherit_base is false)
    for p in &config.sandbox.default_profiles {
        if !names.contains(p) && (config.sandbox.inherit_base || p != "base") {
            names.push(p.clone());
        }
    }

    // Add project-specific profiles from config
    for p in &config.sandbox.profiles {
        if !names.contains(p) {
            names.push(p.clone());
        }
    }

    // Auto-detect project types if enabled
    if config.profiles.auto_detect {
        let detected = detect_project_types(working_dir);
        for pt in detected {
            let profile_name = pt.to_profile().to_string();
            if !names.contains(&profile_name) {
                names.push(profile_name);
            }
        }
    }

    // Add CLI-specified profiles
    for p in &args.profiles {
        if !names.contains(p) {
            names.push(p.clone());
        }
    }

    names
}

/// Build SandboxParams from config, profile, and CLI overrides
fn build_sandbox_params(
    args: &Args,
    config: &Config,
    profile: &Profile,
    working_dir: PathBuf,
    home_dir: PathBuf,
) -> SandboxParams {
    // Determine network mode (CLI > profile > config)
    let network_mode = determine_network_mode(args, profile, config);

    // Collect paths with expansions
    let mut allow_read = collect_allow_read_paths(config, profile, &args.allow_read);
    let mut deny_read = collect_deny_read_paths(config, profile, &args.deny_read);
    let mut allow_write = collect_allow_write_paths(config, profile, &args.allow_write);
    let mut allow_list_dirs = collect_allow_list_dirs_paths(config, profile);

    // If allow_list_dirs is configured, add all parent directories of working_dir
    // This is needed for runtimes like Bun that scan ALL parent directories
    if !allow_list_dirs.is_empty() {
        let mut parent = working_dir.parent();
        while let Some(p) = parent {
            let path_str = p.to_string_lossy().to_string();
            if !path_str.is_empty() && path_str != "/" && !allow_list_dirs.contains(&path_str) {
                allow_list_dirs.push(path_str);
            }
            parent = p.parent();
        }
    }

    // Expand all paths
    allow_read = expand_paths(&allow_read)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    deny_read = expand_paths(&deny_read)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    allow_write = expand_paths(&allow_write)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    allow_list_dirs = expand_paths(&allow_list_dirs)
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    // Build raw rules if present
    let raw_rules = profile.seatbelt.as_ref().and_then(|s| s.raw.clone());

    SandboxParams {
        working_dir,
        home_dir,
        network_mode,
        allow_read: allow_read.into_iter().map(PathBuf::from).collect(),
        deny_read: deny_read.into_iter().map(PathBuf::from).collect(),
        allow_write: allow_write.into_iter().map(PathBuf::from).collect(),
        allow_list_dirs: allow_list_dirs.into_iter().map(PathBuf::from).collect(),
        raw_rules,
    }
}

/// Determine network mode with precedence: CLI > profile > config
fn determine_network_mode(args: &Args, profile: &Profile, config: &Config) -> NetworkMode {
    // CLI flags take highest precedence
    if args.online || args.localhost || args.offline {
        return args.network_mode();
    }

    // Profile network mode
    if let Some(mode) = profile.network_mode {
        return mode;
    }

    // Config network mode (project then global)
    config
        .sandbox
        .network
        .unwrap_or(config.sandbox.default_network)
}

/// Collect allow-read paths from config, profile, and CLI
fn collect_allow_read_paths(config: &Config, profile: &Profile, cli: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(config.filesystem.allow_read.iter().cloned());
    paths.extend(profile.filesystem.allow_read.iter().cloned());
    paths.extend(cli.iter().cloned());
    paths
}

/// Collect deny-read paths from config, profile, and CLI
fn collect_deny_read_paths(config: &Config, profile: &Profile, cli: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(config.filesystem.deny_read.iter().cloned());
    paths.extend(profile.filesystem.deny_read.iter().cloned());
    paths.extend(cli.iter().cloned());
    paths
}

/// Collect allow-write paths from config, profile, and CLI
fn collect_allow_write_paths(config: &Config, profile: &Profile, cli: &[String]) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(config.filesystem.allow_write.iter().cloned());
    paths.extend(profile.filesystem.allow_write.iter().cloned());
    paths.extend(cli.iter().cloned());
    paths
}

/// Collect allow-list-dirs paths from config and profile (directory listing only)
fn collect_allow_list_dirs_paths(config: &Config, profile: &Profile) -> Vec<String> {
    let mut paths = Vec::new();
    paths.extend(config.filesystem.allow_list_dirs.iter().cloned());
    paths.extend(profile.filesystem.allow_list_dirs.iter().cloned());
    paths
}

/// Generate the default .sandbox.toml template
fn generate_config_template() -> &'static str {
    r#"# .sandbox.toml
# Project-specific sandbox configuration for sx
# See: https://github.com/agentic-dev3o/sandbox-shell

[sandbox]
# Inherit from global config (~/.config/sx/config.toml)
inherit_global = true

# Profiles to apply for this project
# Available: base, online, localhost, rust, claude, gpg
profiles = []

# Default network mode: "offline", "online", or "localhost"
# network = "offline"

[filesystem]
# Additional paths this project needs to read
allow_read = []

# Additional paths this project needs to write
allow_write = []

# Paths to deny even if globally allowed
deny_read = []

# Directories to allow listing (readdir) but not file access inside.
# Useful for runtimes like Bun that scan parent directories.
# Example: ["/Users", "~"] allows listing these directories' contents
# without reading files or subdirectories within them.
allow_list_dirs = []

[shell]
# Additional environment variables to pass through
pass_env = []
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config_template_is_valid_toml() {
        let template = generate_config_template();
        let result: Result<Config, _> = toml::from_str(template);
        assert!(result.is_ok(), "Template should be valid TOML");
    }

    #[test]
    fn test_collect_profile_names_includes_base() {
        let args = Args::try_parse_from(["sx"]).unwrap();
        let config = Config::default();
        let working_dir = PathBuf::from("/tmp");

        let names = collect_profile_names(&args, &config, &working_dir);
        assert!(names.contains(&"base".to_string()));
    }

    #[test]
    fn test_collect_profile_names_excludes_base_when_inherit_base_false() {
        let args = Args::try_parse_from(["sx"]).unwrap();
        let mut config = Config::default();
        config.sandbox.inherit_base = false;
        let working_dir = PathBuf::from("/tmp");

        let names = collect_profile_names(&args, &config, &working_dir);
        assert!(!names.contains(&"base".to_string()));
    }

    #[test]
    fn test_collect_profile_names_adds_cli_profiles() {
        let args = Args::try_parse_from(["sx", "online", "rust"]).unwrap();
        let config = Config::default();
        let working_dir = PathBuf::from("/tmp");

        let names = collect_profile_names(&args, &config, &working_dir);
        assert!(names.contains(&"online".to_string()));
        assert!(names.contains(&"rust".to_string()));
    }

    #[test]
    fn test_determine_network_mode_cli_precedence() {
        let args = Args::try_parse_from(["sx", "--online"]).unwrap();
        let profile = Profile::default();
        let config = Config::default();

        let mode = determine_network_mode(&args, &profile, &config);
        assert_eq!(mode, NetworkMode::Online);
    }

    #[test]
    fn test_determine_network_mode_profile_precedence() {
        let args = Args::try_parse_from(["sx"]).unwrap();
        let mut profile = Profile::default();
        profile.network_mode = Some(NetworkMode::Localhost);
        let config = Config::default();

        let mode = determine_network_mode(&args, &profile, &config);
        assert_eq!(mode, NetworkMode::Localhost);
    }
}
