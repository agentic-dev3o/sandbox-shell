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
complete -c sx -s t -l trace -d 'Trace violations'
complete -c sx -l trace-file -d 'Write trace to file'
complete -c sx -s n -l dry-run -d 'Show profile without executing'
complete -c sx -s c -l config -d 'Use config file'
complete -c sx -l no-config -d 'Ignore all config files'
complete -c sx -l explain -d 'Show what would be allowed/denied'
complete -c sx -l init -d 'Initialize .sandbox.toml'
complete -c sx -l offline -d 'Block all network'
complete -c sx -l online -d 'Allow all network'
complete -c sx -l localhost -d 'Allow localhost only'
complete -c sx -l allow-read -d 'Allow read access to path'
complete -c sx -l allow-write -d 'Allow write access to path'
complete -c sx -l deny-read -d 'Deny read access to path'

# Profile completions
complete -c sx -a 'base' -d 'Minimal sandbox'
complete -c sx -a 'online' -d 'Full network access'
complete -c sx -a 'localhost' -d 'Localhost network only'
complete -c sx -a 'rust' -d 'Rust/Cargo toolchain'
complete -c sx -a 'bun' -d 'Bun runtime'
complete -c sx -a 'claude' -d 'Claude Code support'
complete -c sx -a 'gpg' -d 'GPG signing support'

# Aliases
alias sxo 'sx online'              # Online network access
alias sxl 'sx localhost'           # Localhost only
alias sxb 'sx bun online'          # Bun with network
alias sxr 'sx online rust'         # Rust with network
alias sxc 'sx online gpg claude'   # Claude Code with GPG signing
