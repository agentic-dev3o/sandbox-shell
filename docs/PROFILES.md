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
allow_exec_sugid = ["/bin/ps"]  # allow specific setuid/setgid binaries

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

### Raw Seatbelt Rules

For advanced use cases (IOKit, Mach services, app bundles), custom profiles support raw seatbelt rules:

```toml
# ~/.config/sx/profiles/playwright.toml
network_mode = "online"

[seatbelt]
raw = """
(allow iokit-open-user-client
  (iokit-user-client-class "RootDomainUserClient")
  (iokit-user-client-class "AGXDeviceUserClient")
  (iokit-user-client-class "IOSurfaceRootUserClient"))
(allow iokit-get-properties)
(allow file-issue-extension)
"""

[filesystem]
allow_read = ["~/Library/Caches/ms-playwright/"]
allow_write = ["~/Library/Caches/ms-playwright/"]
```

Raw rules are appended verbatim to the generated seatbelt profile. Use `sx --dry-run myprofile` to verify the output.

### Profile Resolution Order

When you specify a profile name, `sx` searches in this order:

1. **Built-in profiles** (embedded in the binary)
2. **Project custom directory** (if configured)
3. **`~/.config/sx/profiles/{name}.toml`**
4. **Fallback** to `online` with a warning if not found

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
4. **Exec sugid:** path lists are unioned; mixing paths and booleans → last wins
5. **Seatbelt raw rules:** concatenated from all profiles in order
