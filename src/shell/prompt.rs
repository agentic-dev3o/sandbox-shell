//! Prompt modification for sandbox indicator

use crate::config::schema::NetworkMode;

/// Prompt style configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptStyle {
    /// Default colored style
    #[default]
    Default,
    /// Plain text without colors
    Plain,
    /// Colored with ANSI codes
    Colored,
}

/// ANSI color codes
mod colors {
    pub const RED: &str = "\x1b[0;31m";
    pub const YELLOW: &str = "\x1b[0;33m";
    pub const GREEN: &str = "\x1b[0;32m";
    pub const RESET: &str = "\x1b[0m";
}

/// Format a prompt indicator for the given network mode and style
pub fn format_prompt_indicator(mode: NetworkMode, style: PromptStyle) -> String {
    let mode_str = match mode {
        NetworkMode::Offline => "offline",
        NetworkMode::Online => "online",
        NetworkMode::Localhost => "localhost",
    };

    match style {
        PromptStyle::Plain => format!("[sx:{}] ", mode_str),
        PromptStyle::Default | PromptStyle::Colored => {
            let color = match mode {
                NetworkMode::Offline => colors::RED,
                NetworkMode::Online => colors::GREEN,
                NetworkMode::Localhost => colors::YELLOW,
            };
            format!("{}[sx:{}]{} ", color, mode_str, colors::RESET)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_format() {
        assert_eq!(
            format_prompt_indicator(NetworkMode::Offline, PromptStyle::Plain),
            "[sx:offline] "
        );
        assert_eq!(
            format_prompt_indicator(NetworkMode::Online, PromptStyle::Plain),
            "[sx:online] "
        );
        assert_eq!(
            format_prompt_indicator(NetworkMode::Localhost, PromptStyle::Plain),
            "[sx:localhost] "
        );
    }

    #[test]
    fn test_colored_contains_ansi() {
        let indicator = format_prompt_indicator(NetworkMode::Offline, PromptStyle::Colored);
        assert!(indicator.contains("\x1b["));
    }
}
