# Configuration

`sx` uses a layered configuration system: global config, project config, CLI flags.

## Global Config (`~/.config/sx/config.toml`)

Your personal paths. Terminal, shell prompt, directory jumper…

```toml
[sandbox]
default_network = "offline"      # offline | online | localhost
default_profiles = ["base"]      # always include these
shell = "/bin/zsh"               # shell inside sandbox (defaults to $SHELL,
                                 # then /bin/zsh on macOS, /bin/bash on Linux)
prompt_indicator = true          # show [sx:mode] in prompt
inherit_base = true              # include base profile
# allow_exec_sugid = ["/bin/ps"] # allow specific setuid/setgid binaries

[filesystem]
allow_read = [
    # Shell prompt
    "~/.config/starship.toml",
    "~/.cache/starship/",

    # zoxide
    "~/.local/share/zoxide/",

    # Ghostty users - required or terminal breaks (macOS path shown)
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
# allow_exec_sugid = ["/bin/ps"] # allow specific setuid/setgid binaries

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

Custom profiles also support raw seatbelt rules for advanced sandbox operations (IOKit, Mach services, app bundles):

```toml
# ~/.config/sx/profiles/playwright.toml
network_mode = "online"

[seatbelt]
raw = """
(allow iokit-open-user-client
  (iokit-user-client-class "RootDomainUserClient"))
(allow iokit-get-properties)
"""
```

Raw rules from all active profiles are concatenated in order. See [PROFILES.md](PROFILES.md) for details.

## Setuid/Setgid Execution

Some binaries like `/bin/ps` are setuid/setgid. Seatbelt blocks these by default with `forbidden-exec-sugid`. Use `allow_exec_sugid` to opt in.

Three modes:

| Value | Effect |
|-------|--------|
| `false` (default) | Deny all setuid/setgid execution |
| `true` | Allow all setuid/setgid execution |
| `["/bin/ps"]` | Allow only listed binaries |

```toml
# .sandbox.toml
[sandbox]
allow_exec_sugid = ["/bin/ps", "/usr/bin/newgrp"]
```

CLI flag (repeatable, path-based only):

```bash
sx --allow-exec-sugid /bin/ps --allow-exec-sugid /usr/bin/newgrp -- ps aux
```

**Merge semantics:** when both global and project configs specify path lists, paths are unioned. Otherwise project overrides global.

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

## First Run: Prompt and Shell Tooling

`sx` is deny-by-default, and that includes the tools your shell starts. Cache
directories are readable but **not writable** on either platform, so anything
that logs or caches per session will complain the first time you run `sx` with
no global config:

```
Unable to open session log file "~/.cache/starship/session_....log": Permission denied
```

That is the sandbox working. Grant the specific paths your prompt and shell
hooks need in `~/.config/sx/config.toml`:

```toml
[filesystem]
allow_read = [
    "~/.config/starship.toml",   # prompt config
    "~/.config/mise",            # version manager config
    "~/.local/share/mise",       # installed toolchains + shims
    "~/.local/state/mise",
    "~/.local/share/zoxide",     # directory jumper
]
allow_write = [
    "~/.cache/starship",         # per-session prompt log
    "~/.cache/mise",
    "~/.local/state/mise",
    "~/.local/share/zoxide",
]
```

macOS equivalents live under `~/Library/Caches/` and
`~/Library/Application Support/` — see the example at the top of this file.

Keep these grants narrow. `allow_write = ["~/.cache"]` works, but a cache is a
place tools later execute from, so widening it hands a compromised dependency a
persistence foothold. Grant the individual directories instead.

If something breaks and the cause is not obvious, `sx --explain` prints every
resolved path, and `sx --dry-run` prints the policy itself.

## Per-OS Configuration

The config format is identical on macOS and Linux, but the paths are not. Custom
**profiles** support `[platform.macos]` / `[platform.linux]` sections for paths
that only exist on one OS — see [PROFILES.md](PROFILES.md#per-os-sections).

For machine-specific paths in your global config, the simplest approach is to
keep the config next to the machine it describes: `~/.config/sx/config.toml` is
not shared between your Mac and your Linux box.

Settings that behave differently per platform:

| Setting | Note |
|---------|------|
| `allow_exec_sugid` | macOS only; on Linux setuid binaries never elevate |
| `[seatbelt] raw` | macOS only; ignored on Linux |
| `allow_list_dirs` | macOS lists exactly the named directory; Linux also lists nested directories (names only) |
| `deny_read` | on Linux, denies file contents; names may remain listable via a readable parent |

Run `sx --explain` to see exactly what the current machine will enforce.
