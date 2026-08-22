pub mod integration;
pub mod prompt;

pub use integration::{
    generate_bash_integration, generate_fish_integration, generate_zsh_integration, ShellType,
};
pub use prompt::{format_prompt_indicator, PromptStyle};

/// Fallback interactive shell when `$SHELL` and the config are both unset.
///
/// macOS has shipped zsh as the login shell since Catalina; Linux
/// distributions do not reliably install it, so bash is the safe default there.
pub const fn default_shell() -> &'static str {
    if cfg!(target_os = "macos") {
        "/bin/zsh"
    } else {
        "/bin/bash"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_shell_is_absolute_and_platform_appropriate() {
        let shell = super::default_shell();
        assert!(shell.starts_with('/'));
        if cfg!(target_os = "macos") {
            assert_eq!(shell, "/bin/zsh");
        } else {
            assert_eq!(shell, "/bin/bash");
        }
    }
}
