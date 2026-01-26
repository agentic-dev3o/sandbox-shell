# sx.zsh - Zsh integration for sandbox CLI
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
        'node:Node.js/npm toolchain'
        'python:Python toolchain'
        'rust:Rust/Cargo toolchain'
        'go:Go toolchain'
        'claude:Claude Code support'
        'gpg:GPG signing support'
        'git:Git with signing'
    )

    local -a options
    options=(
        '--help:Show help'
        '--version:Show version'
        '--verbose:Verbose output'
        '--debug:Debug mode'
        '--trace:Trace sandbox violations'
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
        '(-t --trace)'{-t,--trace}'[Trace violations]' \
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
alias sxn='sx online node'
alias sxp='sx online python'
alias sxr='sx online rust'
alias sxc='sx online claude'
