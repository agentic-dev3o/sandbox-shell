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
        'rust:Rust/Cargo toolchain'
        'bun:Bun runtime'
        'claude:Claude Code support'
        'gpg:GPG signing support'
    )

    _arguments -s \
        '(-h --help)'{-h,--help}'[Show help]' \
        '(-V --version)'{-V,--version}'[Show version]' \
        '(-v --verbose)'{-v,--verbose}'[Verbose output]' \
        '(-d --debug)'{-d,--debug}'[Debug mode]' \
        '(-t --trace)'{-t,--trace}'[Trace violations]' \
        '--trace-file=[Write trace to file]:file:_files' \
        '(-n --dry-run)'{-n,--dry-run}'[Show profile]' \
        '(-c --config)'{-c,--config}'=[Use config file]:file:_files' \
        '--no-config[Ignore all config files]' \
        '--explain[Show permissions]' \
        '--init[Initialize config]' \
        '--offline[Block network]' \
        '--online[Allow network]' \
        '--localhost[Localhost only]' \
        '*--allow-read=[Allow read access]:path:_files' \
        '*--allow-write=[Allow write access]:path:_files' \
        '*--deny-read=[Deny read access]:path:_files' \
        '*:: :->args'

    case $state in
        args)
            if [[ ${words[CURRENT]} == -* ]]; then
                return
            fi
            # Check if we're past -- (command mode)
            local i
            for (( i=1; i < CURRENT; i++ )); do
                if [[ ${words[i]} == "--" ]]; then
                    _command_names
                    return
                fi
            done
            _describe -t profiles 'profile' profiles
            ;;
    esac
}

compdef _sx sx

# Aliases for common patterns
alias sxo='sx online'              # Online network access
alias sxl='sx localhost'           # Localhost only
alias sxb='sx bun online'          # Bun with network
alias sxr='sx online rust'         # Rust with network
alias sxc='sx online gpg claude'   # Claude Code with GPG signing
