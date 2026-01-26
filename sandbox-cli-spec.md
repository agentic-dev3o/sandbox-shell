# Sandbox CLI Specification

**Project Name:** `sx` (sandbox-shell)
**Version:** 0.1.0
**Author:** Pierre Tomasina
**Date:** January 2026

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Problem Statement](#2-problem-statement)
3. [Solution Overview](#3-solution-overview)
4. [Target Users & Use Cases](#4-target-users--use-cases)
5. [Security Model](#5-security-model)
6. [Command Line Interface](#6-command-line-interface)
7. [Configuration System](#7-configuration-system)
8. [Architecture](#8-architecture)
9. [Implementation Plan](#9-implementation-plan)
10. [Testing Strategy](#10-testing-strategy)
11. [Shell Integration](#11-shell-integration)
12. [Future Considerations](#12-future-considerations)

---

## 1. Executive Summary

### What

`sx` is a lightweight command-line tool that wraps shell sessions and commands in a macOS Seatbelt sandbox, restricting filesystem and network access to protect the user's system from malicious code execution.

### Why

Every `git clone && npm install` executes arbitrary code from thousands of unknown contributors. Modern development workflows routinely run untrusted code with full system access. This tool provides frictionless isolation for the security-conscious developer who doesn't want to spin up a full container for every quick experiment.

### How

By leveraging macOS's native Seatbelt (sandbox-exec) kernel-level enforcement, `sx` dynamically generates sandbox profiles that restrict processes to the current project directory while blocking access to sensitive files like SSH keys, cloud credentials, and other projects.

---

## 2. Problem Statement

### The Current Reality

```
Developer workflow:
$ git clone https://github.com/someone/cool-project
$ cd cool-project
$ npm install        # ← Executes postinstall scripts from 1,847 packages
$ npm run dev        # ← Full access to ~/.ssh, ~/.aws, ~/Documents, everything
```

**What actually happens during `npm install`:**
- Arbitrary JavaScript executes with your full user permissions
- Can read `~/.ssh/id_rsa`, `~/.aws/credentials`, `~/.gnupg/`
- Can write to `~/.bashrc`, `~/.zshrc`, `~/.gitconfig`
- Can exfiltrate data to any server
- Can install persistent backdoors

### Why Existing Solutions Fall Short

| Solution | Problem |
|----------|---------|
| **Docker/Dev Containers** | Heavy, slow startup, IDE-bound, overkill for quick experiments |
| **VMs** | Even heavier, not practical for trying npm packages |
| **Claude Code sandbox** | Only protects writes, reads everything, Claude-specific |
| **sandboxtron** | Unmaintained, hardcoded paths, no config system |
| **Firejail** | Linux only |
| **Nothing** | Current state for 99% of developers |

### The Gap

There is no lightweight, configurable, well-maintained tool for macOS that lets you:

```bash
$ cd ~/projects/untrusted-repo
$ sx                    # Enter sandboxed shell
$ npm install           # Runs isolated - can't read ~/.ssh
$ npm run dev           # Network restricted to localhost + allowlist
$ exit                  # Back to normal shell
```

---

## 3. Solution Overview

### Core Principles

1. **Deny by default** — Block everything, allow only what's explicitly needed
2. **Project-scoped** — Protect everything outside the current directory
3. **Zero friction** — Single command to enter sandbox, works with any shell
4. **Transparent** — Clear indication when sandboxed, loud failures on violations
5. **Configurable** — Global defaults, per-project overrides, composable profiles

### What It Protects

```
PROTECTED (outside project directory):
├── ~/.ssh/              → Private keys
├── ~/.aws/              → Cloud credentials
├── ~/.gnupg/            → GPG keys
├── ~/.claude/           → Claude Code config (unless allowed)
├── ~/.config/           → Application configs
├── ~/Documents/         → Personal files
├── ~/Desktop/           → Personal files
├── ~/other-projects/    → Other codebases
├── /etc/                → System configs
└── Network              → Arbitrary outbound connections
```

### What It Allows

```
ALLOWED (configurable):
├── Current directory    → Read/write (the project)
├── /usr, /bin, /opt     → System binaries (read-only)
├── /tmp                 → Temporary files
├── ~/.gitconfig         → Git configuration (read-only)
├── ~/.cargo, ~/.rustup  → Toolchain caches (configurable)
├── localhost network    → Local dev servers
└── Allowlisted domains  → npm registry, GitHub, etc.
```

---

## 4. Target Users & Use Cases

### Primary User Profile

**Security-conscious developers who:**
- Regularly clone and test open source projects
- Know the risks but find containers too heavy for quick experiments
- Use macOS as their primary development machine
- Want protection without changing their workflow

### Use Cases

#### UC1: Trying a New npm Package

```bash
$ mkdir test-package && cd test-package
$ npm init -y
$ sx                           # Enter sandbox
$ npm install some-new-package # Isolated execution
$ node -e "require('some-new-package')"
$ exit
```

**Protection:** Package's postinstall can't read ~/.ssh or exfiltrate data.

#### UC2: Cloning and Running Unknown Repository

```bash
$ git clone https://github.com/unknown/project
$ cd project
$ sx online                    # Sandbox with network for dependencies
$ npm install
$ sx                           # Switch to offline for running
$ npm run dev
```

**Protection:** Build scripts can't phone home after install.

#### UC3: Running Claude Code on Untrusted Codebase

```bash
$ cd ~/projects/client-codebase
$ sx claude online             # Sandbox with Claude config access
$ claude "analyze this codebase for security issues"
```

**Protection:** Claude can't access other projects or sensitive configs.

#### UC4: Quick Script Execution

```bash
$ sx -- python untrusted_script.py
$ sx -- ./configure && make
$ sx -- cargo build
```

**Protection:** One-liner sandbox for any command.

---

## 5. Security Model

### Threat Model

**In Scope (Protected Against):**
- Supply chain attacks reading sensitive files
- Malicious scripts writing outside project
- Data exfiltration via network
- Persistent backdoors in shell configs
- Cross-project contamination

**Out of Scope (Not Protected Against):**
- Reading files within project directory (by design)
- Environment variable access (OS limitation)
- Kernel exploits / Seatbelt 0-days
- Resource exhaustion (CPU/memory)
- Physical access attacks

### Sandbox Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                     HOST SYSTEM                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                 SANDBOX BOUNDARY                          │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │              PROJECT DIRECTORY                      │  │  │
│  │  │                                                     │  │  │
│  │  │  ✅ Read/Write: ./src, ./node_modules, ./dist      │  │  │
│  │  │  ✅ Execute: ./node_modules/.bin/*                 │  │  │
│  │  │                                                     │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  │                                                           │  │
│  │  ✅ Read-only: /usr, /bin, /opt (system binaries)        │  │
│  │  ✅ Read-only: Allowed toolchain paths                   │  │
│  │  ✅ Write: /tmp, /var/folders (temp files)               │  │
│  │  ✅ Network: localhost + allowlisted domains             │  │
│  │                                                           │  │
│  │  ❌ DENIED: Everything else                              │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  ❌ ~/.ssh, ~/.aws, ~/.gnupg (credentials)                     │
│  ❌ ~/Documents, ~/Desktop (personal files)                    │
│  ❌ ~/other-projects (cross-contamination)                     │
│  ❌ Arbitrary network connections                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Defense Layers

| Layer | Mechanism | Bypass Difficulty |
|-------|-----------|-------------------|
| Filesystem deny | Seatbelt kernel enforcement | Requires 0-day |
| Network deny | Seatbelt kernel enforcement | Requires 0-day |
| Process inheritance | Child processes inherit sandbox | Automatic |
| Profile composition | Minimal permissions by default | User must explicitly allow |

---

## 6. Command Line Interface

### Command Structure

```
sx [OPTIONS] [PROFILES...] [-- COMMAND [ARGS...]]
```

### Basic Usage

```bash
# Enter sandboxed shell (interactive)
sx

# Enter sandboxed shell with network
sx online

# Run single command in sandbox
sx -- npm install

# Run command with specific profiles
sx online node -- npm install

# Run command with network
sx online -- curl https://api.github.com
```

### Options

```
OPTIONS:
    -h, --help                 Print help information
    -V, --version              Print version information
    -v, --verbose              Enable verbose output (show sandbox config)
    -d, --debug                Enable debug mode (log all denials)
    -n, --dry-run              Print generated sandbox profile without executing
    -c, --config <PATH>        Use specific config file
        --no-config            Ignore all config files
        --init                 Initialize .sandbox.toml in current directory
        --explain              Show what would be allowed/denied

NETWORK MODES:
        --offline              Block all network (default)
        --online               Allow all network
        --localhost            Allow localhost only
        --allow-domain <DOMAIN>  Allow specific domain (can repeat)

FILESYSTEM:
        --allow-read <PATH>    Allow read access to path
        --allow-write <PATH>   Allow write access to path
        --deny-read <PATH>     Deny read access to path (override allows)

PROFILES:
    Profiles are loaded from:
    1. ~/.config/sx/profiles/
    2. .sandbox/profiles/ (project-local)

    Built-in profiles:
        base        Minimal sandbox (always included)
        online      Full network access
        localhost   Localhost network only
        node        Node.js/npm toolchain
        python      Python toolchain
        rust        Rust/Cargo toolchain
        go          Go toolchain
        claude      Claude Code (~/.claude access)
        gpg         GPG signing support
        git         Git with signing support

ENVIRONMENT:
    sx_CONFIG_DIR      Override config directory
    sx_PROFILE_DIR     Override profile directory
    sx_DEBUG           Enable debug mode
    SANDBOX_MODE       Set to 'online' or 'offline' inside sandbox
```

### Examples

```bash
# Basic sandboxed shell
$ sx
[sx:offline] ~/project $

# With network for package installation
$ sx online
[sx:online] ~/project $

# Quick command execution
$ sx -- make build
$ sx online -- npm install
$ sx -- python script.py

# Multiple profiles
$ sx online node gpg -- npm install
$ sx claude online -- claude "explain this code"

# Debugging
$ sx --explain                    # Show what would be allowed
$ sx --dry-run                    # Show generated Seatbelt profile
$ sx --debug -- npm install       # Log all sandbox violations

# Project initialization
$ sx --init                       # Create .sandbox.toml with defaults

```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 126 | Command not executable |
| 127 | Command not found |
| 130 | Interrupted (Ctrl+C) |
| 137 | Sandbox violation (SIGKILL from Seatbelt) |

---

## 7. Configuration System

### Configuration Hierarchy (lowest to highest priority)

```
1. Built-in defaults
2. ~/.config/sx/config.toml          (user global)
3. .sandbox.toml                      (project root)
4. Command-line arguments             (highest priority)
```

### Global Configuration

**Location:** `~/.config/sx/config.toml`

```toml
# ~/.config/sx/config.toml
# Global sandbox configuration

[sandbox]
# Default network mode: "offline" | "online" | "localhost"
default_network = "offline"

# Always include these profiles
default_profiles = ["base"]

# Shell to use inside sandbox (default: $SHELL)
shell = "/bin/zsh"

# Show sandbox indicator in shell prompt
prompt_indicator = true

# Log file for sandbox violations
log_file = "~/.local/share/sx/violations.log"

[filesystem]
# Paths to always allow reading (toolchains, etc.)
allow_read = [
    "~/.gitconfig",
    "~/.config/git",
    "~/.cargo",
    "~/.rustup",
    "~/.npm",
    "~/.node-gyp",
    "~/.pyenv",
]

# Paths to always deny (override any allows)
deny_read = [
    "~/.ssh",
    "~/.aws",
    "~/.gnupg",
    "~/.config/gh",
    "~/.netrc",
    "~/.docker/config.json",
    "~/Documents",
    "~/Desktop",
    "~/Downloads",
]

# Paths to always allow writing (beyond project dir)
allow_write = [
    "~/.npm/_cacache",
    "~/.cache",
]

[network]
# Domains always allowed when online
allow_domains = [
    "registry.npmjs.org",
    "github.com",
    "api.github.com",
    "pypi.org",
    "files.pythonhosted.org",
    "crates.io",
    "static.crates.io",
]

# Domains always blocked (even when online)
deny_domains = [
    # Add known malicious domains here
]

[shell]
# Environment variables to pass through to sandbox
pass_env = [
    "TERM",
    "COLORTERM",
    "LANG",
    "LC_ALL",
    "EDITOR",
    "VISUAL",
    "PAGER",
    "HOME",
    "USER",
    "SHELL",
    "PATH",
]

# Environment variables to NEVER pass (secrets)
deny_env = [
    "AWS_*",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GITHUB_TOKEN",
    "NPM_TOKEN",
    "*_SECRET*",
    "*_PASSWORD*",
    "*_KEY",
]

# Set these variables inside the sandbox
set_env = { SANDBOX_MODE = "active" }

[profiles]
# Auto-detect project type and apply profiles
auto_detect = true

# Profile detection rules
[profiles.detect]
node = ["package.json"]
python = ["requirements.txt", "pyproject.toml", "setup.py"]
rust = ["Cargo.toml"]
go = ["go.mod"]
```

### Project Configuration

**Location:** `.sandbox.toml` (project root)

```toml
# .sandbox.toml
# Project-specific sandbox configuration

[sandbox]
# Inherit from global config (default: true)
inherit_global = true

# Profiles to use for this project
profiles = ["node", "claude"]

# Default network mode for this project
network = "localhost"

[filesystem]
# Additional paths this project needs to read
allow_read = [
    "~/.claude",           # Claude Code config
]

# Additional paths this project needs to write
allow_write = [
    "~/.claude/projects",  # Claude project cache
]

# Paths to deny even if globally allowed
deny_read = [
    # This project shouldn't access cargo
    "~/.cargo",
]

[network]
# Additional domains for this project
allow_domains = [
    "api.anthropic.com",
    "api.openai.com",
]

[shell]
# Additional env vars to pass for this project
pass_env = [
    "ANTHROPIC_API_KEY",   # Explicitly allow for this project
]
```

### Profile Files

**Location:** `~/.config/sx/profiles/` or `.sandbox/profiles/`

**Example: `claude.profile.toml`**

```toml
# Claude Code profile
[filesystem]
allow_read = ["~/.claude"]
allow_write = ["~/.claude"]

[network]
allow_domains = ["api.anthropic.com"]

[shell]
pass_env = ["ANTHROPIC_API_KEY"]
```

**Example: `gpg.profile.toml`**

```toml
# GPG signing profile
[filesystem]
allow_read = ["~/.gnupg"]
allow_write = ["~/.gnupg"]

[seatbelt]
# Raw Seatbelt rules for advanced config
raw = """
(allow network-outbound
       (to unix-socket (subpath (string-append (param "home") "/.gnupg"))))
(allow mach-lookup
       (global-name-regex "^org\\.gpgtools\\..*")
       (global-name "com.apple.SecurityServer"))
"""
```

---

## 8. Architecture

### Directory Structure

```
sx/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── LICENSE
├── SECURITY.md
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── src/
│   ├── main.rs                 # Entry point, CLI parsing
│   ├── lib.rs                  # Library root
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs             # Argument parsing (clap)
│   │   └── commands.rs         # Subcommands (init, explain, etc.)
│   ├── config/
│   │   ├── mod.rs
│   │   ├── global.rs           # Global config loading
│   │   ├── project.rs          # Project config loading
│   │   ├── profile.rs          # Profile loading and composition
│   │   ├── merge.rs            # Config merging logic
│   │   └── schema.rs           # Config structs (serde)
│   ├── sandbox/
│   │   ├── mod.rs
│   │   ├── seatbelt.rs         # Seatbelt profile generation
│   │   ├── executor.rs         # sandbox-exec invocation
│   │   └── violations.rs       # Violation logging/parsing
│   ├── detection/
│   │   ├── mod.rs
│   │   └── project_type.rs     # Auto-detect project type
│   ├── shell/
│   │   ├── mod.rs
│   │   ├── prompt.rs           # Prompt modification
│   │   └── integration.rs      # Shell integration scripts
│   └── utils/
│       ├── mod.rs
│       ├── paths.rs            # Path expansion, normalization
│       └── logging.rs          # Structured logging
├── profiles/
│   ├── base.sx                 # Base Seatbelt profile
│   ├── online.sx               # Network access
│   ├── localhost.sx            # Localhost only
│   ├── node.toml               # Node.js profile
│   ├── python.toml             # Python profile
│   ├── rust.toml               # Rust profile
│   ├── go.toml                 # Go profile
│   ├── claude.toml             # Claude Code profile
│   └── gpg.toml                # GPG profile
├── shell/
│   ├── sx.zsh                  # Zsh integration
│   ├── sx.bash                 # Bash integration
│   └── sx.fish                 # Fish integration
├── tests/
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── filesystem_test.rs  # Filesystem restriction tests
│   │   ├── network_test.rs     # Network restriction tests
│   │   ├── config_test.rs      # Config loading tests
│   │   └── profile_test.rs     # Profile composition tests
│   ├── unit/
│   │   ├── mod.rs
│   │   ├── seatbelt_test.rs    # Seatbelt generation tests
│   │   └── paths_test.rs       # Path handling tests
│   └── fixtures/
│       ├── configs/
│       └── projects/
├── docs/
│   ├── CONFIGURATION.md
│   ├── PROFILES.md
│   ├── SECURITY.md
│   └── SHELL_INTEGRATION.md
└── scripts/
    ├── install.sh
    └── test-security.sh        # Security verification tests
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                           CLI Layer                                 │
│                         (src/cli/)                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │   args.rs   │  │ commands.rs │  │   main.rs   │                 │
│  │   (clap)    │  │ (init,etc)  │  │  (entry)    │                 │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                 │
└─────────┼────────────────┼────────────────┼─────────────────────────┘
          │                │                │
          ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        Config Layer                                 │
│                       (src/config/)                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │  global.rs  │  │ project.rs  │  │  merge.rs   │                 │
│  │             │  │             │  │             │                 │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘                 │
│         │                │                │                         │
│         └────────────────┼────────────────┘                         │
│                          ▼                                          │
│                   ┌─────────────┐                                   │
│                   │ profile.rs  │  ← Compose profiles               │
│                   └──────┬──────┘                                   │
└──────────────────────────┼──────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Sandbox Layer                                 │
│                      (src/sandbox/)                                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                      seatbelt.rs                            │   │
│  │  ┌─────────────────────────────────────────────────────┐   │   │
│  │  │  generate_profile(config) -> String                 │   │   │
│  │  │                                                     │   │   │
│  │  │  (version 1)                                        │   │   │
│  │  │  (deny default)                                     │   │   │
│  │  │  (allow file-read* (subpath "/usr"))               │   │   │
│  │  │  (allow file* (subpath "{working_dir}"))           │   │   │
│  │  │  ...                                                │   │   │
│  │  └─────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                          │                                          │
│                          ▼                                          │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     executor.rs                             │   │
│  │                                                             │   │
│  │  sandbox-exec -f /tmp/sx-{hash}.sx \                       │   │
│  │    -D home="$HOME" \                                       │   │
│  │    -D working_dir="$(pwd)" \                               │   │
│  │    /bin/zsh                                                │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     macOS Kernel                                    │
│                 (Seatbelt/sandbox-exec)                             │
│                                                                     │
│     Enforces restrictions at kernel level                           │
│     Child processes inherit sandbox                                 │
│     Violations logged to system log                                 │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User runs: sx online node -- npm install
                    │
                    ▼
┌──────────────────────────────────────┐
│ 1. Parse CLI arguments               │
│    profiles = ["online", "node"]     │
│    command = ["npm", "install"]      │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 2. Load configurations               │
│    ~/.config/sx/config.toml          │
│    ./.sandbox.toml (if exists)       │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 3. Merge configs + profiles          │
│    base + global + project + CLI     │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 4. Generate Seatbelt profile         │
│    /tmp/sx-a1b2c3.sx                 │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 5. Execute via sandbox-exec          │
│    sandbox-exec -f /tmp/sx-a1b2c3.sx │
│      -D home="/Users/pierre"         │
│      -D working_dir="/path/to/proj"  │
│      npm install                     │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 6. Kernel enforces restrictions      │
│    npm runs sandboxed                │
│    Violations logged                 │
└──────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────┐
│ 7. Cleanup                           │
│    Remove temp profile               │
│    Return exit code                  │
└──────────────────────────────────────┘
```

---

## 9. Implementation Plan

### Phase 1: Foundation (Week 1-2)

#### Milestone 1.1: Project Bootstrap

- [ ] Initialize Cargo project with workspace
- [ ] Set up CI/CD (GitHub Actions)
- [ ] Configure linting (clippy) and formatting (rustfmt)
- [ ] Set up pre-commit hooks
- [ ] Create initial README and documentation structure

**Deliverable:** Empty project that builds and passes CI

#### Milestone 1.2: CLI Skeleton

- [ ] Implement argument parsing with clap
- [ ] Define all CLI options and flags
- [ ] Implement `--help` and `--version`
- [ ] Add basic subcommands (`init`, `explain`)
- [ ] Set up logging infrastructure (tracing)

**Dependencies:**
```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

**Deliverable:** `sx --help` shows all options

#### Milestone 1.3: Configuration Loading

- [ ] Define config structs with serde
- [ ] Implement global config loading (`~/.config/sx/config.toml`)
- [ ] Implement project config loading (`.sandbox.toml`)
- [ ] Implement config merging logic
- [ ] Add path expansion (`~` → `/Users/...`)
- [ ] Write unit tests for config parsing

**Dependencies:**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"
shellexpand = "3"
```

**Deliverable:** Configs load and merge correctly

### Phase 2: Sandbox Core (Week 3-4)

#### Milestone 2.1: Seatbelt Profile Generation

- [ ] Create base Seatbelt profile template
- [ ] Implement filesystem allow/deny rule generation
- [ ] Implement network rule generation
- [ ] Implement profile composition (multiple profiles)
- [ ] Add parameter substitution (`{home}`, `{working_dir}`)
- [ ] Write unit tests for profile generation

**Deliverable:** Generate valid Seatbelt profiles from config

#### Milestone 2.2: Sandbox Execution

- [ ] Implement sandbox-exec invocation
- [ ] Handle profile file creation/cleanup
- [ ] Implement process spawning and waiting
- [ ] Forward signals to sandboxed process
- [ ] Capture and forward exit codes
- [ ] Handle interactive shell mode

**Deliverable:** `sx -- ls` runs sandboxed

#### Milestone 2.3: Built-in Profiles

- [ ] Create `base.sx` (minimal sandbox)
- [ ] Create `online.sx` (network access)
- [ ] Create `localhost.sx` (localhost only)
- [ ] Create toolchain profiles (node, python, rust, go)
- [ ] Create `claude.toml` profile
- [ ] Create `gpg.toml` profile

**Deliverable:** All standard profiles work

### Phase 3: User Experience (Week 5-6)

#### Milestone 3.1: Project Detection

- [ ] Implement project type detection (package.json, Cargo.toml, etc.)
- [ ] Auto-apply relevant profiles based on detection
- [ ] Add `--no-detect` flag to disable

**Deliverable:** `sx` in Node project auto-applies node profile

#### Milestone 3.2: Shell Integration

- [ ] Create zsh integration script
- [ ] Create bash integration script
- [ ] Implement prompt indicator
- [ ] Export `SANDBOX_MODE` environment variable
- [ ] Document shell setup

**Deliverable:** Shell prompt shows sandbox status

#### Milestone 3.3: Debugging and Diagnostics

- [ ] Implement `--explain` (show what would be allowed/denied)
- [ ] Implement `--dry-run` (show generated profile)
- [ ] Implement `--debug` (log violations)
- [ ] Parse sandbox violations from system log
- [ ] Create violation log file
- [ ] Add helpful error messages

**Deliverable:** Users can debug sandbox issues

### Phase 4: Polish and Release (Week 7-8)

#### Milestone 4.1: Testing

- [ ] Write integration tests for filesystem restrictions
- [ ] Write integration tests for network restrictions
- [ ] Write security verification tests
- [ ] Test on multiple macOS versions
- [ ] Performance benchmarking

**Deliverable:** Comprehensive test suite

#### Milestone 4.2: Documentation

- [ ] Complete README with examples
- [ ] Write CONFIGURATION.md
- [ ] Write PROFILES.md
- [ ] Write SECURITY.md (threat model, limitations)
- [ ] Create man page
- [ ] Add inline documentation

**Deliverable:** Full documentation

#### Milestone 4.3: Distribution

- [ ] Set up Homebrew formula
- [ ] Create GitHub releases with binaries
- [ ] Add installation script
- [ ] Create demo GIFs/screenshots

**Deliverable:** Easy installation via Homebrew

---

## 10. Testing Strategy

### Unit Tests

**Location:** `tests/unit/`

```rust
// tests/unit/seatbelt_test.rs

#[test]
fn test_generate_deny_default() {
    let config = SandboxConfig::default();
    let profile = generate_seatbelt_profile(&config);
    assert!(profile.contains("(deny default)"));
}

#[test]
fn test_allow_working_directory() {
    let config = SandboxConfig {
        working_dir: PathBuf::from("/Users/test/project"),
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&config);
    assert!(profile.contains(r#"(subpath "/Users/test/project")"#));
}

#[test]
fn test_deny_ssh_directory() {
    let config = SandboxConfig {
        deny_read: vec!["~/.ssh".into()],
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&config);
    assert!(profile.contains(r#"(deny file-read* (subpath "/Users/test/.ssh"))"#));
}

#[test]
fn test_network_offline() {
    let config = SandboxConfig {
        network_mode: NetworkMode::Offline,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&config);
    assert!(!profile.contains("(allow network"));
}

#[test]
fn test_network_localhost() {
    let config = SandboxConfig {
        network_mode: NetworkMode::Localhost,
        ..Default::default()
    };
    let profile = generate_seatbelt_profile(&config);
    assert!(profile.contains(r#"(local ip "localhost:*")"#));
}
```

### Integration Tests

**Location:** `tests/integration/`

```rust
// tests/integration/filesystem_test.rs

use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cannot_read_ssh_keys() {
    let temp_dir = TempDir::new().unwrap();

    // Create a test file in ~/.ssh
    let ssh_dir = dirs::home_dir().unwrap().join(".ssh");

    // Run command that tries to read SSH key
    let output = Command::new("sx")
        .args(["--", "cat", ssh_dir.join("id_rsa").to_str().unwrap()])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    // Should fail with permission denied
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Operation not permitted") ||
            stderr.contains("Permission denied"));
}

#[test]
fn test_can_read_project_files() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file in project directory
    std::fs::write(temp_dir.path().join("test.txt"), "hello").unwrap();

    let output = Command::new("sx")
        .args(["--", "cat", "test.txt"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello");
}

#[test]
fn test_cannot_write_outside_project() {
    let temp_dir = TempDir::new().unwrap();
    let outside_file = "/tmp/sx-test-outside.txt";

    let output = Command::new("sx")
        .args(["--", "touch", outside_file])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    assert!(!output.status.success());
    assert!(!std::path::Path::new(outside_file).exists());
}

// tests/integration/network_test.rs

#[test]
fn test_network_blocked_offline() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("sx")
        .args(["--", "curl", "-s", "-o", "/dev/null", "-w", "%{http_code}",
               "https://httpbin.org/get"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    // Should fail - network blocked
    assert!(!output.status.success());
}

#[test]
fn test_network_allowed_online() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("sx")
        .args(["online", "--", "curl", "-s", "-o", "/dev/null", "-w", "%{http_code}",
               "https://httpbin.org/get"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "200");
}

#[test]
fn test_localhost_allowed() {
    let temp_dir = TempDir::new().unwrap();

    // Start a local server in background
    // ...

    let output = Command::new("sx")
        .args(["--", "curl", "-s", "http://localhost:8080"])
        .current_dir(&temp_dir)
        .output()
        .expect("Failed to execute sx");

    assert!(output.status.success());
}
```

### Security Verification Tests

**Location:** `scripts/test-security.sh`

```bash
#!/bin/bash
# Security verification tests - run manually to verify sandbox effectiveness

set -e

echo "=== Security Verification Tests ==="

TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

# Test 1: Cannot read SSH keys
echo "Test 1: SSH key protection"
if sx -- cat ~/.ssh/id_rsa 2>/dev/null; then
    echo "FAIL: Could read SSH key!"
    exit 1
else
    echo "PASS: SSH key protected"
fi

# Test 2: Cannot read AWS credentials
echo "Test 2: AWS credential protection"
if sx -- cat ~/.aws/credentials 2>/dev/null; then
    echo "FAIL: Could read AWS credentials!"
    exit 1
else
    echo "PASS: AWS credentials protected"
fi

# Test 3: Cannot write to home directory
echo "Test 3: Home directory write protection"
if sx -- touch ~/.sx-test-file 2>/dev/null; then
    rm -f ~/.sx-test-file
    echo "FAIL: Could write to home directory!"
    exit 1
else
    echo "PASS: Home directory write protected"
fi

# Test 4: Cannot modify shell config
echo "Test 4: Shell config protection"
if sx -- sh -c 'echo "malicious" >> ~/.zshrc' 2>/dev/null; then
    echo "FAIL: Could modify .zshrc!"
    exit 1
else
    echo "PASS: Shell config protected"
fi

# Test 5: Network blocked in offline mode
echo "Test 5: Network isolation (offline)"
if sx -- curl -s --max-time 5 https://httpbin.org/get >/dev/null 2>&1; then
    echo "FAIL: Network not blocked!"
    exit 1
else
    echo "PASS: Network blocked"
fi

# Test 6: Can write to project directory
echo "Test 6: Project directory writable"
if sx -- touch ./test-file; then
    rm -f ./test-file
    echo "PASS: Project directory writable"
else
    echo "FAIL: Cannot write to project directory!"
    exit 1
fi

# Test 7: Child processes inherit sandbox
echo "Test 7: Child process inheritance"
if sx -- sh -c 'cat ~/.ssh/id_rsa' 2>/dev/null; then
    echo "FAIL: Child process not sandboxed!"
    exit 1
else
    echo "PASS: Child processes inherit sandbox"
fi

# Cleanup
rm -rf "$TEMP_DIR"

echo ""
echo "=== All security tests passed ==="
```

### Test Matrix

| Test Category | Test Cases | Priority |
|--------------|------------|----------|
| Filesystem Read | SSH, AWS, GPG, other projects | Critical |
| Filesystem Write | Home dir, system dirs, shell configs | Critical |
| Network | Offline, localhost, allowlist | High |
| Config Loading | Global, project, merge, profiles | High |
| Profile Generation | Seatbelt syntax, parameters | High |
| Shell Integration | Prompt, env vars | Medium |
| Edge Cases | Symlinks, special chars, large files | Medium |
| Performance | Startup time, overhead | Low |

---

## 11. Shell Integration

### Zsh Integration

**File:** `shell/sx.zsh`

```zsh
# sx.zsh - Zsh integration for sandbox CLI
# Add to ~/.zshrc: source /path/to/sx.zsh

# Sandbox prompt indicator
_sx_prompt_indicator() {
    if [[ -n "$SANDBOX_MODE" ]]; then
        local color
        case "$SANDBOX_MODE" in
            offline) color="%F{red}" ;;
            localhost) color="%F{yellow}" ;;
            online) color="%F{green}" ;;
            *) color="%F{blue}" ;;
        esac
        echo "${color}[sx:${SANDBOX_MODE}]%f "
    fi
}

# Prepend to existing PROMPT
if [[ -z "$_sx_PROMPT_INITIALIZED" ]]; then
    PROMPT='$(_sx_prompt_indicator)'"$PROMPT"
    _sx_PROMPT_INITIALIZED=1
fi

# Completions
_sx() {
    local -a profiles
    profiles=(
        'base:Minimal sandbox (always included)'
        'online:Full network access'
        'localhost:Localhost network only'
        'node:Node.js/npm toolchain'
        'python:Python toolchain'
        'rust:Rust/Cargo toolchain'
        'go:Go toolchain'
        'claude:Claude Code support'
        'gpg:GPG signing support'
        'git:Git with signing'
    )

    local -a options
    options=(
        '--help:Show help'
        '--version:Show version'
        '--verbose:Verbose output'
        '--debug:Debug mode'
        '--dry-run:Show profile without executing'
        '--explain:Show what would be allowed/denied'
        '--init:Initialize .sandbox.toml'
        '--offline:Block all network'
        '--online:Allow all network'
        '--localhost:Allow localhost only'
    )

    _arguments \
        '(-h --help)'{-h,--help}'[Show help]' \
        '(-V --version)'{-V,--version}'[Show version]' \
        '(-v --verbose)'{-v,--verbose}'[Verbose output]' \
        '(-d --debug)'{-d,--debug}'[Debug mode]' \
        '(-n --dry-run)'{-n,--dry-run}'[Show profile]' \
        '--explain[Show permissions]' \
        '--init[Initialize config]' \
        '--offline[Block network]' \
        '--online[Allow network]' \
        '--localhost[Localhost only]' \
        '*:profile:_describe "profile" profiles' \
        '-- :command:_command_names'
}

compdef _sx sx

# Aliases for common patterns
alias sxo='sx online'
alias sxl='sx localhost'
alias sxn='sx online node'
alias sxp='sx online python'
alias sxr='sx online rust'
alias sxc='sx online claude'
```

### Bash Integration

**File:** `shell/sx.bash`

```bash
# sx.bash - Bash integration for sandbox CLI
# Add to ~/.bashrc: source /path/to/sx.bash

# Sandbox prompt indicator
_sx_prompt_indicator() {
    if [[ -n "$SANDBOX_MODE" ]]; then
        local color reset
        reset='\[\033[0m\]'
        case "$SANDBOX_MODE" in
            offline) color='\[\033[0;31m\]' ;;   # Red
            localhost) color='\[\033[0;33m\]' ;; # Yellow
            online) color='\[\033[0;32m\]' ;;    # Green
            *) color='\[\033[0;34m\]' ;;         # Blue
        esac
        echo -e "${color}[sx:${SANDBOX_MODE}]${reset} "
    fi
}

# Prepend to existing PS1
if [[ -z "$_sx_PROMPT_INITIALIZED" ]]; then
    PROMPT_COMMAND='PS1="$(_sx_prompt_indicator)${_sx_ORIGINAL_PS1:-$PS1}"'
    _sx_ORIGINAL_PS1="$PS1"
    _sx_PROMPT_INITIALIZED=1
fi

# Completions
_sx_completions() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local profiles="base online localhost node python rust go claude gpg git"
    local options="--help --version --verbose --debug --dry-run --explain --init --offline --online --localhost"

    if [[ "$cur" == -* ]]; then
        COMPREPLY=($(compgen -W "$options" -- "$cur"))
    else
        COMPREPLY=($(compgen -W "$profiles" -- "$cur"))
    fi
}

complete -F _sx_completions sx

# Aliases
alias sxo='sx online'
alias sxl='sx localhost'
alias sxn='sx online node'
alias sxp='sx online python'
alias sxr='sx online rust'
alias sxc='sx online claude'
```

### Fish Integration

**File:** `shell/sx.fish`

```fish
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
```

### Ghostty Integration

For Ghostty terminal, you can configure visual indicators:

**File:** `~/.config/ghostty/config`

```
# Show sandbox status in tab title
# The SANDBOX_MODE env var will appear in the title when set

# Optional: Different color scheme for sandboxed shells
# (Requires Ghostty 1.1+ with conditional config)
```

---

## 12. Future Considerations

### Short-term (v0.2)

- [ ] Profile sharing/marketplace
- [ ] Violation notifications (macOS notifications)
- [ ] Stats/analytics (how many violations blocked)
- [ ] Config validation command
- [ ] Profile linting

### Medium-term (v0.3)

- [ ] Endpoint Security integration (optional, for monitoring)
- [ ] DNS filtering (block DNS exfiltration)
- [ ] Environment variable filtering
- [ ] Resource limits (if macOS adds support)

### Long-term (v1.0+)

- [ ] Migration to Apple Containerization (when macOS 26+ is mainstream)
- [ ] Linux support (via Landlock/bubblewrap)
- [ ] VS Code extension
- [ ] GUI configuration app

### Potential Risks

| Risk | Mitigation |
|------|------------|
| Apple removes sandbox-exec | Abstract execution layer for future migration |
| Seatbelt vulnerabilities | Defense in depth, keep dependencies updated |
| User confusion | Clear documentation, loud warnings |
| Performance issues | Minimal overhead by design, profiling |

---

## Appendix A: Seatbelt Profile Reference

### Basic Structure

```scheme
(version 1)
(deny default)

; Parameters
(define home-path (param "home"))
(define working-dir (param "working_dir"))

; Helper functions
(define (home-subpath path)
  (subpath (string-append home-path path)))

; Rules
(allow file-read* ...)
(allow file-write* ...)
(allow network* ...)
```

### Common Operations

| Operation | Seatbelt Rule |
|-----------|---------------|
| Read file | `(allow file-read* (literal "/path/to/file"))` |
| Read directory tree | `(allow file-read* (subpath "/path/to/dir"))` |
| Read by regex | `(allow file-read* (regex "^/path/.*\\.txt$"))` |
| Write file | `(allow file-write* (literal "/path/to/file"))` |
| Write directory tree | `(allow file-write* (subpath "/path/to/dir"))` |
| Full file access | `(allow file* (subpath "/path/to/dir"))` |
| Network all | `(allow network*)` |
| Network localhost | `(allow network* (local ip "localhost:*"))` |
| Process fork | `(allow process-fork)` |
| Process exec | `(allow process-exec)` |

---

## Appendix B: Dependencies

```toml
[package]
name = "sx"
version = "0.1.0"
edition = "2021"
authors = ["Pierre Tomasina"]
description = "Lightweight sandbox for macOS development"
license = "MIT"
repository = "https://github.com/pierozi/sx"
keywords = ["sandbox", "security", "macos", "seatbelt"]
categories = ["command-line-utilities", "development-tools"]

[dependencies]
# CLI
clap = { version = "4", features = ["derive", "env", "wrap_help"] }

# Configuration
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Paths and directories
dirs = "5"
shellexpand = "3"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Process management
nix = { version = "0.28", features = ["process", "signal"] }

# Utilities
thiserror = "1"
anyhow = "1"
tempfile = "3"
uuid = { version = "1", features = ["v4"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"

[profile.release]
lto = true
strip = true
codegen-units = 1
```

---

**Document Version:** 1.0
**Last Updated:** January 2026
**Status:** Draft Specification
