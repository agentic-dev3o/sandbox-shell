# Profiles

Profiles are composable sandbox configurations that can be combined to create the right security posture for your project.

## Built-in Profiles

### base

The foundational profile included by default. Provides:
- Read access to system directories (`/usr`, `/bin`, `/sbin`, `/opt`)
- Read access to temp directories (`/tmp`, `/var/folders`)
- Denies access to sensitive directories (`~/.ssh`, `~/.aws`, `~/.gnupg`)
- Basic environment variables (`TERM`, `PATH`, `HOME`, `USER`)

### online

Enables full network access.

```bash
sx --profile online
```

### localhost

Allows network connections only to localhost (127.0.0.1).

```bash
sx --profile localhost
```

### node

For Node.js/npm projects:
- Read access: `~/.npm`, `~/.nvm`, `~/.npmrc`
- Write access: `~/.npm/_cacache`
- Network domains: `registry.npmjs.org`

```bash
sx --profile node
```

### python

For Python projects:
- Read access: `~/.pyenv`, `~/.local/share/virtualenvs`
- Write access: `~/.cache/pip`
- Network domains: `pypi.org`, `files.pythonhosted.org`

```bash
sx --profile python
```

### rust

For Rust projects:
- Read access: `~/.cargo`, `~/.rustup`
- Write access: `~/.cargo/registry`
- Network domains: `crates.io`, `static.crates.io`

```bash
sx --profile rust
```

### go

For Go projects:
- Read access: `~/go`, `~/.config/go`
- Write access: `~/go/pkg`
- Network domains: `proxy.golang.org`, `sum.golang.org`

```bash
sx --profile go
```

### claude

For Claude Code projects:
- Read/Write access: `~/.claude`
- Network domains: `api.anthropic.com`
- Passes: `ANTHROPIC_API_KEY`

```bash
sx --profile claude
```

### gpg

For GPG signing:
- Read/Write access: `~/.gnupg`

```bash
sx --profile gpg
```

## Profile Composition

Profiles can be combined. The order matters for network mode (last one wins):

```bash
# Node project with full network access
sx --profile node --profile online

# Rust project with localhost only
sx --profile rust --profile localhost

# Multiple language project
sx --profile node --profile python --profile online
```

## Custom Profiles

Create custom profiles in `~/.config/sx/profiles/` or `./profiles/`:

```toml
# ~/.config/sx/profiles/mycompany.toml
network_mode = "online"

[filesystem]
allow_read = ["/opt/mycompany"]
allow_write = ["~/.mycompany/cache"]

[network]
allow_domains = ["api.mycompany.com", "*.internal.mycompany.com"]

[shell]
pass_env = ["MYCOMPANY_TOKEN"]
```

Use it:

```bash
sx --profile mycompany
```

## Profile Merging Rules

When multiple profiles are composed:

1. **Network mode**: Last profile with a network mode wins
2. **Filesystem paths**: Union of all paths (no duplicates)
3. **Network domains**: Union of all domains
4. **Environment variables**: Union of all pass/deny lists

## Project-Specific Profiles

Define profiles in `.sx.toml`:

```toml
[sandbox]
profiles = ["base", "node", "localhost"]
```

Auto-detect project type:

```toml
[profiles]
auto_detect = true
```
