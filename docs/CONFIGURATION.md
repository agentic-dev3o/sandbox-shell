# Configuration

`sx` uses a layered configuration system: global config, project config, CLI flags.

## Global Config (`~/.config/sx/config.toml`)

Your personal paths. Terminal, shell prompt, directory jumper…

```toml
[sandbox]
default_network = "offline"      # offline | online | localhost
default_profiles = ["base"]      # always include these
shell = "/bin/zsh"               # shell inside sandbox
prompt_indicator = true          # show [sx:mode] in prompt
inherit_base = true              # include base profile

[filesystem]
allow_read = [
    # Shell prompt
    "~/.config/starship.toml",
    "~/.cache/starship/",

    # zoxide
    "~/.local/share/zoxide/",

    # Ghostty users - required or terminal breaks
    "/Applications/Ghostty.app/Contents/Resources/terminfo",
]
allow_write = [
    "~/.local/share/zoxide/",
    "~/Library/Application Support/zoxide/",
    "~/.cache/",
]
deny_read = []  # additional paths to block

[shell]
pass_env = ["CUSTOM_VAR"]        # env vars to pass through
deny_env = ["*_SECRET*"]         # env vars to block (wildcards)
set_env = { CI = "true" }        # env vars to set inside sandbox
```

## Project Config (`.sandbox.toml`)

Per-project overrides. Create with `sx --init`.

```toml
[sandbox]
profiles = ["rust"]              # profiles for this project
network = "localhost"            # override network mode
inherit_global = true            # inherit from global config
inherit_base = true              # include base profile (false for full custom)

[filesystem]
allow_read = ["./vendor"]
allow_write = ["./target", "/tmp/build"]
deny_read = ["./secrets"]

[shell]
pass_env = ["RUST_LOG", "NODE_ENV"]
set_env = { DEBUG = "1" }
```

## Custom Profiles

Create in `~/.config/sx/profiles/`:

```toml
# ~/.config/sx/profiles/myproject.toml
network_mode = "online"

[filesystem]
allow_read = ["/opt/myproject"]
allow_write = ["~/.myproject/cache"]

[shell]
pass_env = ["MYPROJECT_TOKEN"]
```

Use with `sx myproject -- command`.

## Precedence

1. CLI flags (highest)
2. Project config (`.sandbox.toml`)
3. Global config (`~/.config/sx/config.toml`)
4. Built-in defaults (lowest)

## Environment Wildcards

`deny_env` supports wildcards:
- `AWS_*` - matches `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`…
- `*_SECRET*` - matches `DATABASE_SECRET`, `MY_SECRET_KEY`…
- `*_KEY` - matches `API_KEY`, `SSH_KEY`…
