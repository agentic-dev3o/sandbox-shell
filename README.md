# sx - macOS Sandbox CLI for Secure Development

[![QA](https://github.com/agentic-dev3o/sandbox-shell/actions/workflows/QA.yaml/badge.svg)](https://github.com/agentic-dev3o/sandbox-shell/actions/workflows/QA.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://developer.apple.com/documentation/security/app_sandbox)

> **TL;DR:** Run untrusted code without risking supply chain attacks. That sketchy npm package can't steal your SSH keys or AWS credentials. Uses macOS Seatbelt (`sandbox-exec`) with deny-by-default permissions.

A lightweight Rust CLI that wraps shell commands in macOS Seatbelt sandboxes. That npm package you just installed? It can't read your `~/.ssh` keys or `~/.aws` credentials. Can't steal what you can't see.

Supply chain attacks are everywhere. A single compromised dependency tries to exfiltrate your secrets? It can't—filesystem is deny-by-default. Your credentials aren't readable, even with network enabled. No containers, no VMs, just native macOS sandboxing.

## Quick Start

```bash
brew tap agentic-dev3o/sx
brew install sx

# That's it. Now run untrusted code:
sx -- npm run build
sx -- cargo test
sx -- ./build.sh

# Or start an interactive sandboxed shell
sx
```

Your secrets stay secret. Malicious postinstall scripts get nothing.

## Profiles

Profiles stack. Combine them: `sx online rust -- cargo build`

| Profile | What it does |
|---------|--------------|
| `base` | Minimal sandbox (always included) |
| `online` | Full network access |
| `localhost` | 127.0.0.1 only |
| `rust` | Cargo/rustup paths |
| `bun` | `~/.bun` + parent directory listing for module resolution |
| `claude` | Claude Code paths (includes `online`) |
| `gpg` | GPG signing |

### Examples

```bash
# Bun
sx bun -- bun install           # Offline, from cache
sx bun online -- bun install    # Download deps

# Rust
sx rust -- cargo test           # Offline tests
sx rust online -- cargo build   # Download crates

# Claude Code - the whole point
sx claude -- claude --dangerously-skip-permissions --continue

# Interactive shell with network
sx online
```

## Claude Code Integration

Claude Code has a built-in sandbox mode. Sounds great, except it allows **read-only access to your entire filesystem**. A compromised dependency can still read your `~/.ssh` keys, `~/.aws` credentials, and exfiltrate them.

`sx` is deny-by-default. Sensitive paths are blocked from *reading*, not just writing. Malicious code can't steal what it can't see.

```bash
sx claude -- claude --dangerously-skip-permissions --continue
```

Claude runs agentic, no permission prompts. Supply chain attacks in dependencies? They get sandboxed too. That's the setup I use.

## Installation

### Homebrew

```bash
brew tap agentic-dev3o/sx
brew install sx
```

### From Source

```bash
git clone https://github.com/agentic-dev3o/sandbox-shell.git
cd sandbox-shell
cargo install --path .
```

Requires macOS and Rust 1.70+.

## Configuration

### Global Config (`~/.config/sx/config.toml`)

Your personal paths go here. Terminal, shell prompt, directory jumper…

```toml
[filesystem]
allow_read = [
    # Shell prompt
    "~/.config/starship.toml",
    "~/.cache/starship/",

    # zoxide
    "~/.local/share/zoxide/",

    # Ghostty users - you need this or terminal breaks in sandbox
    "/Applications/Ghostty.app/Contents/Resources/terminfo",

    # Claude Code plugins
    # "~/projects/my-plugins/",
]

allow_write = [
    "~/.local/share/zoxide/",
    "~/Library/Application Support/zoxide/",
    "~/.cache/",
]
```

**Ghostty users:** add that terminfo path or you'll get display issues. Ask me how I know.

### Project Config (`.sandbox.toml`)

Per-project overrides:

```bash
sx --init
```

```toml
[sandbox]
profiles = ["rust"]

[filesystem]
allow_write = ["/tmp/build"]

[shell]
pass_env = ["NODE_ENV", "DEBUG"]
```

Custom profiles go in `~/.config/sx/profiles/name.toml`.

## Usage

```
sx [OPTIONS] [PROFILES]... [-- <COMMAND>...]
```

```bash
# Offline (default)
sx -- npm run build
sx -- ./scripts/setup.sh

# Localhost only - for dev servers
sx localhost -- npm start

# Online
sx online rust -- cargo audit
sx bun online -- bun install

# Debug what's blocked
sx --trace -- cargo build       # Real-time violation log
sx --explain rust               # Show allowed/denied
sx --dry-run rust               # Preview seatbelt profile
```

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show sandbox configuration |
| `-d, --debug` | Log all denials |
| `-t, --trace` | Real-time violation stream |
| `--trace-file <PATH>` | Write trace to file |
| `-n, --dry-run` | Print profile, don't execute |
| `-c, --config <PATH>` | Use specific config |
| `--no-config` | Ignore all configs |
| `--explain` | Show what's allowed/denied |
| `--init` | Create `.sandbox.toml` |
| `--offline` | Block network (default) |
| `--online` | Allow network |
| `--localhost` | 127.0.0.1 only |
| `--allow-read <PATH>` | Allow read |
| `--allow-write <PATH>` | Allow write |
| `--deny-read <PATH>` | Deny read (overrides allows) |

| `--trace` shows violations from *all* sandboxed processes on the system, not just yours. macOS limitation.

## Security Model

### Always Denied (even if you allow `~`)

These paths are explicitly blocked. Even if your config allows the home directory, these stay protected:

| Path | What |
|------|------|
| `~/.ssh` | SSH keys |
| `~/.aws` | AWS credentials |
| `~/.docker/config.json` | Docker credentials |
| `~/Documents`, `~/Desktop`, `~/Downloads` | Personal files |

Everything else (`~/.config/gh`, `~/.netrc`, `~/.gnupg`…) is blocked by deny-by-default. Use profiles like `gpg` to allow specific paths when needed.

### Network Modes

| Mode | Flag | Effect |
|------|------|--------|
| Offline | (default) | All blocked |
| Localhost | `localhost` | 127.0.0.1 only |
| Online | `online` | Full access |

### How It Works

**Reads:** denied by default. Only `/usr`, `/bin`, `/Library`, `/System`.

**Writes:** denied by default. Only working directory and `/tmp`.

**Network:** blocked by default.

## Use Cases

Supply chain attacks are the main threat. That one compromised package in your dependency tree running a postinstall script, exfiltrating `~/.aws` to some random server. Or worse, dropping malware.

`sx` makes npm/bun/yarn safe. Also: untrusted repos, random build scripts, CI/CD isolation locally, Claude Code agentic loops, security research…

## Shell Integration

Prompt indicators, tab completion, aliases.

**Zsh** (`~/.zshrc`):
```bash
source $(brew --prefix)/share/sx/sx.zsh
```

**Bash** (`~/.bashrc`):
```bash
source $(brew --prefix)/share/sx/sx.bash
```

**Fish**:
```fish
cp $(brew --prefix)/share/sx/sx.fish ~/.config/fish/conf.d/
```

### Prompt Colors

- `[sx:offline]` red - network blocked
- `[sx:localhost]` yellow - localhost only
- `[sx:online]` green - full network

### Aliases

| Alias | Command |
|-------|---------|
| `sxo` | `sx online` |
| `sxl` | `sx localhost` |
| `sxb` | `sx bun online` |
| `sxr` | `sx online rust` |
| `sxc` | `sx online gpg claude` |

## Comparison

| Tool | Platform | Overhead | Credential Protection | Network Control |
|------|----------|----------|----------------------|-----------------|
| **sx** | macOS | None | ✅ Deny-by-default | ✅ Offline/localhost/online |
| Docker | Cross-platform | Container runtime | ⚠️ Manual | ⚠️ Manual |
| Firejail | Linux | Minimal | ✅ Profiles | ✅ Profiles |
| Claude sandbox | macOS | None | ❌ Read-only everywhere | ❌ None |
| VM | Cross-platform | Heavy | ✅ Full | ✅ Full |

## Development

```bash
cargo fmt
cargo test
cargo build
cargo run -- --help
```

## License

MIT

## Contributing

PRs welcome. Read the security model before touching sandbox behavior.
