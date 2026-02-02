use super::schema::{Config, FilesystemConfig, SandboxConfig, ShellConfig};
use std::collections::HashSet;

/// Merge two configurations, with project taking precedence
///
/// - Network mode: project overrides global
/// - Profiles: merged (both applied)
/// - Filesystem paths: merged (allows union, denies union)
/// - Environment: merged
pub fn merge_configs(global: &Config, project: &Config) -> Config {
    Config {
        sandbox: merge_sandbox(&global.sandbox, &project.sandbox),
        filesystem: merge_filesystem(&global.filesystem, &project.filesystem),
        shell: merge_shell(&global.shell, &project.shell),
        profiles: global.profiles.clone(), // Profile detection stays from global
    }
}

fn merge_sandbox(global: &SandboxConfig, project: &SandboxConfig) -> SandboxConfig {
    // Filter base from global profiles if inherit_base is false
    let global_profiles = if project.inherit_base {
        global.default_profiles.clone()
    } else {
        global
            .default_profiles
            .iter()
            .filter(|p| *p != "base")
            .cloned()
            .collect()
    };

    SandboxConfig {
        // Project network mode overrides global, or use project.network if set
        default_network: project.network.unwrap_or(project.default_network),
        // Merge profiles (base filtered if inherit_base is false)
        default_profiles: merge_unique_strings(&global_profiles, &project.default_profiles),
        // Project shell overrides global
        shell: project.shell.clone().or_else(|| global.shell.clone()),
        // Project prompt_indicator overrides global
        prompt_indicator: project.prompt_indicator,
        // Project log_file overrides global
        log_file: project.log_file.clone().or_else(|| global.log_file.clone()),
        // Keep project settings
        inherit_global: project.inherit_global,
        // Project inherit_base overrides global
        inherit_base: project.inherit_base,
        // Merge profiles (using pre-filtered global_profiles)
        profiles: merge_unique_strings(&global_profiles, &project.profiles),
        network: project.network,
    }
}

fn merge_filesystem(global: &FilesystemConfig, project: &FilesystemConfig) -> FilesystemConfig {
    FilesystemConfig {
        allow_read: merge_unique_strings(&global.allow_read, &project.allow_read),
        deny_read: merge_unique_strings(&global.deny_read, &project.deny_read),
        allow_write: merge_unique_strings(&global.allow_write, &project.allow_write),
        allow_list_dirs: merge_unique_strings(&global.allow_list_dirs, &project.allow_list_dirs),
    }
}

fn merge_shell(global: &ShellConfig, project: &ShellConfig) -> ShellConfig {
    let mut set_env = global.set_env.clone();
    set_env.extend(project.set_env.clone());

    ShellConfig {
        pass_env: merge_unique_strings(&global.pass_env, &project.pass_env),
        deny_env: merge_unique_strings(&global.deny_env, &project.deny_env),
        set_env,
    }
}

/// Merge two string vectors, keeping unique values.
/// Uses HashSet for O(1) lookups instead of O(n) contains() checks.
fn merge_unique_strings(a: &[String], b: &[String]) -> Vec<String> {
    let mut seen: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let mut result = a.to_vec();
    for item in b {
        if seen.insert(item.as_str()) {
            result.push(item.clone());
        }
    }
    result
}
