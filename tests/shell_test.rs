//! Unit tests for shell integration

use sx::config::schema::NetworkMode;
use sx::shell::integration::{
    generate_bash_integration, generate_fish_integration, generate_zsh_integration, ShellType,
};
use sx::shell::prompt::{format_prompt_indicator, PromptStyle};

#[test]
fn test_prompt_indicator_offline() {
    let indicator = format_prompt_indicator(NetworkMode::Offline, PromptStyle::Default);
    assert!(indicator.contains("sx:offline"));
}

#[test]
fn test_prompt_indicator_online() {
    let indicator = format_prompt_indicator(NetworkMode::Online, PromptStyle::Default);
    assert!(indicator.contains("sx:online"));
}

#[test]
fn test_prompt_indicator_localhost() {
    let indicator = format_prompt_indicator(NetworkMode::Localhost, PromptStyle::Default);
    assert!(indicator.contains("sx:localhost"));
}

#[test]
fn test_prompt_style_plain() {
    let indicator = format_prompt_indicator(NetworkMode::Offline, PromptStyle::Plain);
    assert_eq!(indicator, "[sx:offline] ");
    assert!(!indicator.contains('\x1b')); // No ANSI codes
}

#[test]
fn test_prompt_style_colored() {
    let indicator = format_prompt_indicator(NetworkMode::Offline, PromptStyle::Colored);
    assert!(indicator.contains("sx:offline"));
}

#[test]
fn test_shell_type_detect_zsh() {
    assert_eq!(ShellType::from_path("/bin/zsh"), ShellType::Zsh);
    assert_eq!(ShellType::from_path("/usr/local/bin/zsh"), ShellType::Zsh);
}

#[test]
fn test_shell_type_detect_bash() {
    assert_eq!(ShellType::from_path("/bin/bash"), ShellType::Bash);
    assert_eq!(ShellType::from_path("/usr/local/bin/bash"), ShellType::Bash);
}

#[test]
fn test_shell_type_detect_fish() {
    assert_eq!(ShellType::from_path("/usr/local/bin/fish"), ShellType::Fish);
    assert_eq!(
        ShellType::from_path("/opt/homebrew/bin/fish"),
        ShellType::Fish
    );
}

#[test]
fn test_shell_type_detect_unknown() {
    assert_eq!(ShellType::from_path("/bin/sh"), ShellType::Unknown);
    assert_eq!(ShellType::from_path("/usr/bin/tcsh"), ShellType::Unknown);
}

#[test]
fn test_generate_zsh_integration() {
    let script = generate_zsh_integration();
    assert!(script.contains("_sx_prompt_indicator"));
    assert!(script.contains("SANDBOX_MODE"));
    assert!(script.contains("compdef"));
    assert!(script.contains("offline"));
    assert!(script.contains("online"));
    assert!(script.contains("localhost"));
}

#[test]
fn test_generate_bash_integration() {
    let script = generate_bash_integration();
    assert!(script.contains("_sx_prompt_indicator"));
    assert!(script.contains("SANDBOX_MODE"));
    assert!(script.contains("complete"));
    assert!(script.contains("_sx_completions"));
}

#[test]
fn test_generate_fish_integration() {
    let script = generate_fish_integration();
    assert!(script.contains("_sx_prompt_indicator"));
    assert!(script.contains("SANDBOX_MODE"));
    assert!(script.contains("complete -c sx"));
    assert!(script.contains("function fish_prompt"));
}

#[test]
fn test_zsh_integration_has_aliases() {
    let script = generate_zsh_integration();
    assert!(script.contains("alias sxo"));
    assert!(script.contains("alias sxl"));
}

#[test]
fn test_bash_integration_has_aliases() {
    let script = generate_bash_integration();
    assert!(script.contains("alias sxo"));
    assert!(script.contains("alias sxl"));
}

#[test]
fn test_fish_integration_has_aliases() {
    let script = generate_fish_integration();
    assert!(script.contains("alias sxo"));
    assert!(script.contains("alias sxl"));
}

#[test]
fn test_prompt_colors_differ_by_mode() {
    let offline = format_prompt_indicator(NetworkMode::Offline, PromptStyle::Colored);
    let online = format_prompt_indicator(NetworkMode::Online, PromptStyle::Colored);
    let localhost = format_prompt_indicator(NetworkMode::Localhost, PromptStyle::Colored);

    // Different modes should produce different colored outputs
    assert_ne!(offline, online);
    assert_ne!(online, localhost);
    assert_ne!(offline, localhost);
}
