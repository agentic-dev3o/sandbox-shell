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
    local profiles="base online localhost node python rust go claude gpg git"
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
alias sxn='sx online node'
alias sxp='sx online python'
alias sxr='sx online rust'
alias sxc='sx online claude'
