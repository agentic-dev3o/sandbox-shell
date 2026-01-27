# sx.fish - Fish integration for sandbox CLI
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
complete -c sx -a 'node' -d 'Node.js/npm toolchain'
complete -c sx -a 'python' -d 'Python toolchain'
complete -c sx -a 'rust' -d 'Rust/Cargo toolchain'
complete -c sx -a 'go' -d 'Go toolchain'
complete -c sx -a 'claude' -d 'Claude Code support'
complete -c sx -a 'gpg' -d 'GPG signing support'
complete -c sx -a 'git' -d 'Git with signing'

# Aliases
alias sxo 'sx online'
alias sxl 'sx localhost'
alias sxn 'sx online node'
alias sxp 'sx online python'
alias sxr 'sx online rust'
alias sxc 'sx online claude'
