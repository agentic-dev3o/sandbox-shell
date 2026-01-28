# sx - macOS Sandbox CLI for Secure Development

[![QA](https://github.com/agentic-dev3o/sandbox-shell/actions/workflows/QA.yaml/badge.svg)](https://github.com/agentic-dev3o/sandbox-shell/actions/workflows/QA.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/platform-macOS-lightgrey.svg)](https://developer.apple.com/documentation/security/app_sandbox)

> **TL;DR:** Run untrusted code without exposing your SSH keys, AWS credentials, or personal files. Uses macOS Seatbelt (`sandbox-exec`) with deny-by-default permissions.

A lightweight Rust CLI that wraps shell commands and terminals in macOS Seatbelt sandboxes. Protect your system from malicious npm packages, supply chain attacks, compromised dependencies, and untrusted build scripts.

**Key features:**
- **Credential protection** - Blocks access to `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/gh`
- **Network isolation** - Offline by default, localhost-only, or full access
- **Filesystem isolation** - Deny-by-default reads, scoped writes
- **Claude Code integration** - Safe agentic loops with `--dangerously-skip-permissions`
- **Zero overhead** - Native macOS sandbox, no containers or VMs

## Quick Start

```bash
# Install
cargo install --path .

# Run commands in an isolated sandbox (network blocked, credentials protected)
sx -- bun lint
sx -- cargo test
sx -- ./build.sh

# Start an interactive sandboxed shell
sx
```

That's it. Your credentials (`~/.ssh`, `~/.aws`, `~/.gnupg`) and personal files are protected from malicious scripts.

## Installation

### Using Cargo (Recommended)

```bash
git clone https://github.com/agentic-dev3o/sandbox-shell.git
cd sandbox-shell
cargo install --path .
```

### Manual Build

```bash
cargo build --release
sudo cp target/release/sx /usr/local/bin/
```

### Verify

```bash
sx --version
sx --help
```

**Requirements:** macOS (uses Apple's Seatbelt sandbox), Rust 1.70+

## Use Cases

- **npm/bun/yarn projects** - Protect against malicious postinstall scripts
- **Cloning untrusted repos** - Safely explore code without risk
- **Running build scripts** - Isolate `make`, `./configure`, custom scripts
- **CI/CD local testing** - Reproduce pipeline isolation locally
- **Claude Code / AI agents** - Safe agentic loops without exposing credentials
- **Security research** - Analyze suspicious code safely

## Why Use sx?

When you run `npm install` or clone an untrusted repo, malicious code can:
- Steal your SSH keys and AWS credentials
- Access your personal documents
- Exfiltrate data over the network

`sx` prevents this by running commands in a sandbox that blocks access to sensitive paths and network by default.

### Why sx Instead of Claude's Native Sandbox?

Claude Code offers a built-in sandbox mode, but it allows **read-only access to your entire filesystem** by default. This means a compromised npm package or malicious build script can still read your `~/.ssh` keys, `~/.aws` credentials, and other secrets.

`sx` takes a **deny-by-default** approach: sensitive paths are blocked from reading, not just writing.

This makes `sx` ideal for **agentic loops** where you want Claude to run autonomously:

```bash
# Run Claude Code with dangerous permissions inside sx sandbox
sx online claude -- claude --dangerously-skip-permissions

# Or start a sandboxed shell for an agentic session
sx online claude
```

With this setup:
- Claude can execute commands without permission prompts (agentic loop)
- Malicious code cannot read your SSH keys, AWS credentials, or personal files
- Network access is controlled (offline by default, or scoped with `online`/`localhost`)

This is the best of both worlds: full automation for trusted AI workflows, with protection against supply chain attacks in dependencies.

## Usage

```
sx [OPTIONS] [PROFILES]... [-- <COMMAND>...]
```

### Common Patterns

```bash
# Offline sandbox (default) - run untrusted code safely
sx -- npm run build         # Build with cached deps
sx -- ./scripts/setup.sh    # Run untrusted scripts
sx rust -- cargo test       # Run tests isolated

# Localhost only - for dev servers
sx localhost -- npm start   # Allows 127.0.0.1 only

# Online - when network is required
sx online rust -- cargo build   # Download crates

# Debug what's being blocked
sx --trace -- cargo build   # Real-time violation log
sx --explain rust           # Show what would be allowed
sx --dry-run rust           # Preview sandbox profile
```

### Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Show sandbox configuration |
| `-t, --trace` | Trace sandbox violations in real-time (see note below) |
| `--trace-file <PATH>` | Write trace output to file instead of stderr |
| `-n, --dry-run` | Print sandbox profile without executing |
| `--explain` | Show what would be allowed/denied |
| `--init` | Create `.sandbox.toml` in current directory |
| `--allow-read <PATH>` | Allow read access to path |
| `--allow-write <PATH>` | Allow write access to path |

> **Note on `--trace`:** The trace output shows sandbox violations from **all sandboxed processes** on the system, not just the current session. This is a limitation of macOS sandbox logging, which doesn't include session identifiers in denial logs. If you're running multiple `sx` sessions simultaneously, violations from all sessions will appear in each trace output.

## Profiles

Profiles are composable configurations that stack together:

| Profile | Description |
|---------|-------------|
| `base` | Minimal sandbox (always included) |
| `online` | Full network access |
| `localhost` | Localhost-only network |
| `rust` | Rust/Cargo toolchain |
| `claude` | Claude Code support |
| `gpg` | GPG signing support |

Combine profiles: `sx online rust -- cargo build`

## Configuration

### Project Config (`.sandbox.toml`)

Create a config in your project root:

```bash
sx --init
```

Example:

```toml
[sandbox]
profiles = ["rust"]
# network = "localhost"

[filesystem]
allow_write = ["/tmp/build"]

[network]
allow_domains = ["api.example.com"]

[shell]
pass_env = ["NODE_ENV", "DEBUG"]
```

### Global Config (`~/.config/sx/config.toml`)

System-wide defaults and custom profiles.

## Security Model

### What's Protected by Default

| Path | Description |
|------|-------------|
| `~/.ssh` | SSH keys |
| `~/.aws` | AWS credentials |
| `~/.gnupg` | GPG keys |
| `~/.config/gh` | GitHub CLI tokens |
| `~/.netrc` | Network credentials |
| `~/.docker/config.json` | Docker credentials |
| `~/Documents`, `~/Desktop`, `~/Downloads` | Personal files |

### Network Modes

| Mode | Flag | Description |
|------|------|-------------|
| Offline | (default) | All network blocked |
| Localhost | `localhost` | Only 127.0.0.1 allowed |
| Online | `online` | Full network access |

### How It Works

1. **Reads:** Denied by default. Only system paths (`/usr`, `/bin`, `/Library`, `/System`) allowed.
2. **Writes:** Denied by default. Only working directory and `/tmp` allowed.
3. **Network:** Blocked by default. Use `online` or `localhost` profiles.

## Shell Integration

Add to your shell config for prompt indicators and tab completion:

**Zsh** (`~/.zshrc`):
```bash
source /path/to/sandbox-shell/shell/sx.zsh
```

**Bash** (`~/.bashrc`):
```bash
source /path/to/sandbox-shell/shell/sx.bash
```

**Fish** (`~/.config/fish/conf.d/`):
```fish
cp shell/sx.fish ~/.config/fish/conf.d/
```

Provides: prompt indicator, tab completion, aliases (`sxo`, `sxl`, `sxr`).

## Development

```bash
cargo test              # Run tests
cargo build             # Debug build
cargo run -- --help     # Run from source
```

## Comparison with Alternatives

| Tool | Platform | Overhead | Credential Protection | Network Control |
|------|----------|----------|----------------------|-----------------|
| **sx** | macOS | None (native) | ✅ Deny-by-default | ✅ Offline/localhost/online |
| Docker | Cross-platform | Container runtime | ⚠️ Manual config | ⚠️ Manual config |
| Firejail | Linux | Minimal | ✅ Profiles | ✅ Profiles |
| Claude sandbox | macOS | None | ❌ Read-only everywhere | ❌ No control |
| VM (Parallels, etc.) | Cross-platform | Heavy | ✅ Full isolation | ✅ Full isolation |

**When to use sx:**
- You're on macOS and want native performance
- You need to protect credentials from untrusted code
- You want fine-grained network control (offline by default)
- You're running Claude Code in agentic mode

## License

MIT

## Contributing

Contributions welcome! Please read the security model before submitting PRs that modify sandbox behavior.

---

**Keywords:** macOS sandbox, seatbelt, sandbox-exec, secure shell, isolated terminal, credential protection, supply chain security, npm security, Claude Code sandbox, agentic AI security, developer security tools
