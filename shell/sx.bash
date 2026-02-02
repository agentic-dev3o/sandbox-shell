# sx.bash - Bash integration for sandbox CLI
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
    local profiles="base online localhost rust bun claude gpg"
    local options="--help --version --verbose --debug --trace --trace-file --dry-run --config --no-config --explain --init --offline --online --localhost --allow-read --allow-write --deny-read"

    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "$options" -- "$cur"))
    else
        COMPREPLY=($(compgen -W "$profiles" -- "$cur"))
    fi
}

complete -F _sx_completions sx

# Aliases
alias sxo='sx online'              # Online network access
alias sxl='sx localhost'           # Localhost only
alias sxb='sx bun online'          # Bun with network
alias sxr='sx online rust'         # Rust with network
alias sxc='sx online gpg claude'   # Claude Code with GPG signing
