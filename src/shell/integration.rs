//! Shell integration scripts generation

use std::path::Path;

/// Shell type for integration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Zsh,
    Bash,
    Fish,
    Unknown,
}

impl ShellType {
    /// Detect shell type from a shell path
    pub fn from_path(path: &str) -> Self {
        let path = Path::new(path);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match name {
            "zsh" => ShellType::Zsh,
            "bash" => ShellType::Bash,
            "fish" => ShellType::Fish,
            _ => ShellType::Unknown,
        }
    }
}

/// Generate the Zsh integration script
pub fn generate_zsh_integration() -> &'static str {
    ZSH_INTEGRATION
}

/// Generate the Bash integration script
pub fn generate_bash_integration() -> &'static str {
    BASH_INTEGRATION
}

/// Generate the Fish integration script
pub fn generate_fish_integration() -> &'static str {
    FISH_INTEGRATION
}

const ZSH_INTEGRATION: &str = r#"# sx.zsh - Zsh integration for sandbox CLI
# Add to ~/.zshrc: source /path/to/sx.zsh

# Sandbox prompt indicator
_sx_prompt_indicator() {
    if [[ -n "$SANDBOX_MODE" ]]; then
        local color
        case "$SANDBOX_MODE" in
            offline) color="%F{red}" ;;
            localhost) color="%F{yellow}" ;;
            online) color="%F{green}" ;;
            *) color="%F{blue}" ;;
        esac
        echo "${color}[sx:${SANDBOX_MODE}]%f "
    fi
}

# Prepend to existing PROMPT
if [[ -z "$_sx_PROMPT_INITIALIZED" ]]; then
    PROMPT='$(_sx_prompt_indicator)'"$PROMPT"
    _sx_PROMPT_INITIALIZED=1
fi

# Completions
_sx() {
    local -a profiles
    profiles=(
        'base:Minimal sandbox (always included)'
        'online:Full network access'
        'localhost:Localhost network only'
        'rust:Rust/Cargo toolchain'
        'claude:Claude Code support'
        'gpg:GPG signing support'
    )

    local -a options
    options=(
        '--help:Show help'
        '--version:Show version'
        '--verbose:Verbose output'
        '--debug:Debug mode'
        '--dry-run:Show profile without executing'
        '--explain:Show what would be allowed/denied'
        '--init:Initialize .sandbox.toml'
        '--offline:Block all network'
        '--online:Allow all network'
        '--localhost:Allow localhost only'
    )

    _arguments \
        '(-h --help)'{-h,--help}'[Show help]' \
        '(-V --version)'{-V,--version}'[Show version]' \
        '(-v --verbose)'{-v,--verbose}'[Verbose output]' \
        '(-d --debug)'{-d,--debug}'[Debug mode]' \
        '(-n --dry-run)'{-n,--dry-run}'[Show profile]' \
        '--explain[Show permissions]' \
        '--init[Initialize config]' \
        '--offline[Block network]' \
        '--online[Allow network]' \
        '--localhost[Localhost only]' \
        '*:profile:_describe "profile" profiles' \
        '-- :command:_command_names'
}

compdef _sx sx

# Aliases for common patterns
alias sxo='sx online'
alias sxl='sx localhost'
alias sxr='sx online rust'
alias sxc='sx online claude'
alias sxb='sx online bun'
"#;

const BASH_INTEGRATION: &str = r#"# sx.bash - Bash integration for sandbox CLI
# Add to ~/.bashrc: source /path/to/sx.bash

# Sandbox prompt indicator
_sx_prompt_indicator() {
    if [[ -n "$SANDBOX_MODE" ]]; then
        local color reset
        reset='\[\033[0m\]'
        case "$SANDBOX_MODE" in
            offline) color='\[\033[0;31m\]' ;;   # Red
            localhost) color='\[\033[0;33m\]' ;; # Yellow
            online) color='\[\033[0;32m\]' ;;    # Green
            *) color='\[\033[0;34m\]' ;;         # Blue
        esac
        echo -e "${color}[sx:${SANDBOX_MODE}]${reset} "
    fi
}

# Prepend to existing PS1
if [[ -z "$_sx_PROMPT_INITIALIZED" ]]; then
    PROMPT_COMMAND='PS1="$(_sx_prompt_indicator)${_sx_ORIGINAL_PS1:-$PS1}"'
    _sx_ORIGINAL_PS1="$PS1"
    _sx_PROMPT_INITIALIZED=1
fi

# Completions
_sx_completions() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local profiles="base online localhost rust claude gpg"
    local options="--help --version --verbose --debug --dry-run --explain --init --offline --online --localhost"

    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "$options" -- "$cur"))
    else
        COMPREPLY=($(compgen -W "$profiles" -- "$cur"))
    fi
}

complete -F _sx_completions sx

# Aliases
alias sxo='sx online'
alias sxl='sx localhost'
alias sxr='sx online rust'
alias sxc='sx online claude'
alias sxb='sx online bun'
"#;

const FISH_INTEGRATION: &str = r#"# sx.fish - Fish integration for sandbox CLI
# Add to ~/.config/fish/conf.d/sx.fish

# Sandbox prompt indicator
function _sx_prompt_indicator
    if set -q SANDBOX_MODE
        switch $SANDBOX_MODE
            case offline
                set_color red
            case localhost
                set_color yellow
            case online
                set_color green
            case '*'
                set_color blue
        end
        echo -n "[sx:$SANDBOX_MODE] "
        set_color normal
    end
end

# Add to fish_prompt if not already added
if not functions -q _sx_original_fish_prompt
    functions -c fish_prompt _sx_original_fish_prompt
    function fish_prompt
        _sx_prompt_indicator
        _sx_original_fish_prompt
    end
end

# Completions
complete -c sx -s h -l help -d 'Show help'
complete -c sx -s V -l version -d 'Show version'
complete -c sx -s v -l verbose -d 'Verbose output'
complete -c sx -s d -l debug -d 'Debug mode'
complete -c sx -s n -l dry-run -d 'Show profile without executing'
complete -c sx -l explain -d 'Show what would be allowed/denied'
complete -c sx -l init -d 'Initialize .sandbox.toml'
complete -c sx -l offline -d 'Block all network'
complete -c sx -l online -d 'Allow all network'
complete -c sx -l localhost -d 'Allow localhost only'

# Profile completions
complete -c sx -a 'base' -d 'Minimal sandbox'
complete -c sx -a 'online' -d 'Full network access'
complete -c sx -a 'localhost' -d 'Localhost network only'
complete -c sx -a 'rust' -d 'Rust/Cargo toolchain'
complete -c sx -a 'claude' -d 'Claude Code support'
complete -c sx -a 'gpg' -d 'GPG signing support'

# Aliases
alias sxo 'sx online'
alias sxl 'sx localhost'
alias sxr 'sx online rust'
alias sxc 'sx online claude'
alias sxb 'sx online bun'
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_detection() {
        assert_eq!(ShellType::from_path("/bin/zsh"), ShellType::Zsh);
        assert_eq!(ShellType::from_path("/bin/bash"), ShellType::Bash);
        assert_eq!(ShellType::from_path("/usr/local/bin/fish"), ShellType::Fish);
        assert_eq!(ShellType::from_path("/bin/sh"), ShellType::Unknown);
    }

    #[test]
    fn test_zsh_integration_content() {
        let script = generate_zsh_integration();
        assert!(script.contains("SANDBOX_MODE"));
        assert!(script.contains("compdef"));
    }
}
