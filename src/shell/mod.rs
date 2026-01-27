pub mod integration;
pub mod prompt;

pub use integration::{
    generate_bash_integration, generate_fish_integration, generate_zsh_integration, ShellType,
};
pub use prompt::{format_prompt_indicator, PromptStyle};
