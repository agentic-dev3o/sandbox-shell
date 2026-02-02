# Profiles

Profiles are composable sandbox configs. Stack them: `sx online rust -- cargo build`

## Built-in Profiles

### base

Always included (unless `inherit_base = false`). Provides:
- Read access to system directories (`/usr`, `/bin`, `/sbin`, `/Library`, `/System`)
- Read access to shell configs (`~/.zshrc`, `~/.bashrc`…)
- Write access to `/tmp` and session temp dir
- Basic env vars (`TERM`, `PATH`, `HOME`, `USER`, `SHELL`)

**Always denied** (even if you allow `~`):
- `~/.ssh`
- `~/.aws`
- `~/.docker/config.json`
- `~/Documents`, `~/Desktop`, `~/Downloads`

### online

Full network access.

```bash
sx online -- curl https://example.com
```

### localhost

127.0.0.1 only. For dev servers.

```bash
sx localhost -- npm start
```

### rust

Rust/Cargo toolchain:
- Read/write: `~/.cargo`, `~/.rustup`
- Env: `CARGO_HOME`, `RUSTUP_HOME`

```bash
sx rust online -- cargo build
```

### bun

Bun runtime:
- Read/write: `~/.bun`
- Parent directory listing for module resolution (`/Users`, `~`)
- Env: `BUN_INSTALL`, `NODE_ENV`

```bash
sx bun online -- bun install
```

### claude

Claude Code:
- Read/write: `~/.claude`, `~/.claude.json`
- Includes `online` network
- Env: `ANTHROPIC_API_KEY`

```bash
sx claude -- claude --dangerously-skip-permissions --continue
```

### gpg

GPG signing:
- Read/write: `~/.gnupg`

```bash
sx gpg -- git commit -S -m "signed"
```

## Combining Profiles

Order matters for network mode (last wins). Filesystem paths merge.

```bash
# Rust with network
sx rust online -- cargo build

# Rust offline (tests with cached deps)
sx rust -- cargo test

# Claude with GPG signing
sx claude gpg -- claude --dangerously-skip-permissions

# Bun with network
sx bun online -- bun install
```

## Custom Profiles

Create in `~/.config/sx/profiles/`:

```toml
# ~/.config/sx/profiles/mycompany.toml
network_mode = "online"

[filesystem]
allow_read = ["/opt/mycompany"]
allow_write = ["~/.mycompany/cache"]

[shell]
pass_env = ["MYCOMPANY_TOKEN"]
```

Use it:

```bash
sx mycompany -- ./run.sh
```

## Project Profiles

In `.sandbox.toml`:

```toml
[sandbox]
profiles = ["rust", "localhost"]
```

## Merging Rules

1. **Network mode:** last profile with a mode wins
2. **Filesystem paths:** union (no duplicates)
3. **Env vars:** union of pass/deny lists
