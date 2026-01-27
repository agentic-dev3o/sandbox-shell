# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`sx` (sandbox-shell) is a Rust CLI that wraps shell sessions and commands in macOS Seatbelt sandboxes. It protects developers from malicious code in npm packages, untrusted repositories, and build scripts by restricting filesystem and network access.

## Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build (with LTO)

# Test
cargo test               # Run all tests
cargo test test_name     # Run specific test
cargo test --test cli_test  # Run specific integration test file

# Run from source
cargo run -- --help
cargo run -- echo "test"
cargo run -- --dry-run online node  # Preview seatbelt profile
```

## Architecture

The codebase follows a layered architecture with clear separation of concerns:

```
src/
├── main.rs         # Binary entry point: calls sx::run()
├── lib.rs          # Library entry point: parses args, dispatches to commands
├── cli/
│   ├── args.rs     # Clap argument definitions
│   └── commands.rs # Command implementations (init, explain, dry_run, execute)
├── config/
│   ├── schema.rs   # Core types: Config, NetworkMode, FilesystemConfig
│   ├── profile.rs  # Profile system: builtin profiles (base, node, python, etc.)
│   ├── global.rs   # Global config loading (~/.config/sx/config.toml)
│   ├── project.rs  # Project config loading (.sandbox.toml)
│   └── merge.rs    # Config/profile merging logic
├── sandbox/
│   ├── seatbelt.rs # Seatbelt profile generation (the core sandbox logic)
│   ├── executor.rs # sandbox-exec invocation
│   ├── trace.rs    # Real-time violation streaming via macOS `log stream`
│   └── violations.rs # Violation parsing and log file handling
├── detection/
│   └── project_type.rs  # Auto-detect project type from marker files
├── shell/
│   ├── integration.rs   # Shell prompt/completion support
│   └── prompt.rs        # Prompt indicator formatting with ANSI colors
└── utils/
    └── paths.rs    # Path expansion (~, environment variables)
```

### Key Data Flow

1. **CLI args** (`cli/args.rs`) → parsed by Clap
2. **Config loading** (`config/`) → global + project configs merged
3. **Profile composition** (`config/profile.rs`) → profiles stacked (base + node + online, etc.)
4. **Seatbelt generation** (`sandbox/seatbelt.rs`) → generates macOS sandbox-exec profile
5. **Execution** (`sandbox/executor.rs`) → spawns `sandbox-exec -f profile.sb command`

### Security Model

The sandbox uses a **deny-by-default** approach:
- **Reads**: Denied by default, only explicit `allow_read` paths are accessible (system paths like `/usr`, `/bin`, `/Library`, `/System`)
- **Writes**: Denied by default, allowed only for working directory + explicit `allow_write` paths
- **Network**: Configurable (offline/localhost/online)

**Critical Seatbelt Rules**:
1. Root literal `(allow file-read* (literal "/"))` is required for path traversal - processes need to read `/` to resolve paths
2. Seatbelt uses last-match-wins semantics when rules have matching filter types - deny rules after allow rules take precedence for nested paths (e.g., allow `/home` then deny `/home/.ssh`)
3. `(allow file-read-metadata)` must be global (no path filter) - required for `getaddrinfo()` DNS resolution to work. Without this, `curl`, Python, and other tools using the system resolver fail with "Could not resolve host" even when network is allowed. Commands like `host` and `nslookup` work without it because they use direct DNS UDP queries.

**Configuration Options**:
- `inherit_base = false` in `.sandbox.toml` skips the base profile for full custom control over allowed paths

### Profile System

Profiles are composable configurations that stack on top of each other. Order matters - later profiles override network mode but merge filesystem paths.

Built-in profiles: `base`, `online`, `localhost`, `node`, `python`, `rust`, `go`, `claude`, `gpg`

Custom profiles can be added to `~/.config/sx/profiles/name.toml`.

## Programming Rules

Produce code following **LEAN, CLEAN, MINIMAL but thoughtful KISS & YAGNI principle**.
Follow TDD paradigm `red-green-refactor` for unit test.
Follow Rust Book, Rust API Guidelines, and idiomatic Rust resources.
