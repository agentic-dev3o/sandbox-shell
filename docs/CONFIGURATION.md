# Configuration

`sx` uses a layered configuration system with global, project, and CLI options.

## Configuration Files

### Global Configuration

Location: `~/.config/sx/config.toml`

```toml
[sandbox]
default_network = "offline"      # offline | online | localhost
default_profiles = ["base"]      # profiles to always include
shell = "/bin/zsh"               # shell to use inside sandbox
prompt_indicator = true          # show sandbox indicator in prompt
log_file = "~/.sx/violations.log"

[filesystem]
allow_read = ["/usr", "/bin"]    # paths to allow reading
deny_read = ["~/.ssh", "~/.aws"] # paths to deny reading
allow_write = ["/tmp"]           # paths to allow writing

[network]
allow_domains = []               # domains to allow when online
deny_domains = []                # domains to block even when online

[shell]
pass_env = ["TERM", "PATH"]      # env vars to pass through
deny_env = ["AWS_*", "*_SECRET*"] # env vars to block
```

### Project Configuration

Location: `.sandbox.toml` in project root

```toml
[sandbox]
inherit_global = true            # inherit from global config
profiles = ["rust", "online"]    # additional profiles for this project
network = "localhost"            # override network mode

[filesystem]
allow_read = ["./target"]
deny_read = ["./secrets"]
allow_write = ["./target", "./build"]

[shell]
pass_env = ["RUST_LOG"]
set_env = { CI = "true" }
```

## Configuration Precedence

1. CLI flags (highest priority)
2. Project config (`.sx.toml`)
3. Global config (`~/.config/sx/config.toml`)
4. Built-in defaults (lowest priority)

## Network Modes

| Mode | Description |
|------|-------------|
| `offline` | Block all network access (default) |
| `online` | Allow all network access |
| `localhost` | Allow only localhost (127.0.0.1) connections |

## Filesystem Rules

- **allow_read**: Paths the sandbox can read from
- **deny_read**: Paths explicitly denied (overrides allows)
- **allow_write**: Paths the sandbox can write to (besides working directory)

The working directory always has full read/write access.

## Environment Variables

- **pass_env**: Environment variables passed into the sandbox
- **deny_env**: Environment variables blocked (supports wildcards)
- **set_env**: Environment variables to set inside the sandbox

### Wildcard Patterns

Environment variable patterns support wildcards:
- `AWS_*` - matches any variable starting with `AWS_`
- `*_SECRET*` - matches variables containing `_SECRET`
- `*_KEY` - matches variables ending with `_KEY`

## Custom Profiles

Create custom profiles in `~/.config/sx/profiles/`:

```toml
# ~/.config/sx/profiles/myproject.toml
network_mode = "online"

[filesystem]
allow_read = ["/opt/myproject"]
allow_write = ["~/.myproject/cache"]

[shell]
pass_env = ["MYPROJECT_TOKEN"]
```
