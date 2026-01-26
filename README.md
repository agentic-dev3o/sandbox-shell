# sx - Sandbox Shell

A lightweight CLI that wraps shell sessions and commands in macOS Seatbelt sandboxes. Protect your system from malicious npm packages, untrusted repositories, and rogue build scripts.

## Features

- **Filesystem Isolation** - Restrict read/write access to specific paths
- **Network Control** - Block, allow, or limit network to localhost
- **Profile System** - Pre-configured profiles for Node.js, Python, Rust, Go
- **Auto-Detection** - Automatically applies relevant profiles based on project type
- **Credential Protection** - Blocks access to SSH keys, AWS credentials, GPG keys by default
- **Shell Integration** - Prompt indicators and completions for Zsh, Bash, Fish

## Requirements

- macOS (uses Apple's Seatbelt sandbox)
- Rust 1.70+ (for building)

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/agentic-dev3o/sandbox-shell.git
cd sandbox-shell

# Build release binary
cargo build --release

# Install to /usr/local/bin (requires sudo)
sudo cp target/release/sx /usr/local/bin/

# Or install to user bin directory
mkdir -p ~/.local/bin
cp target/release/sx ~/.local/bin/
# Add ~/.local/bin to PATH if not already
```

### Verify Installation

```bash
sx --version
sx --help
```

## Quick Start

```bash
# Run a command in offline sandbox (default)
sx -- npm install

# Start an interactive sandboxed shell
sx

# Allow network access
sx online -- npm install

# Allow only localhost (for local dev servers)
sx localhost -- npm start

# Use toolchain-specific profile
sx node -- npm install
sx python -- pip install requests
sx rust -- cargo build

# Combine profiles
sx online node -- npm install
```

## Usage

```
sx [OPTIONS] [PROFILES]... [-- <COMMAND>...]

Arguments:
  [PROFILES]...  Profiles to apply (e.g., online, node, claude)
  [COMMAND]...   Command to run in sandbox (after --)

Options:
  -v, --verbose      Show sandbox configuration
  -n, --dry-run      Print sandbox profile without executing
      --explain      Show what would be allowed/denied
      --init         Create .sandbox.toml in current directory
      --offline      Block all network (default)
      --online       Allow all network
      --localhost    Allow localhost only
      --allow-read   Allow read access to path
      --allow-write  Allow write access to path
      --deny-read    Deny read access to path
  -h, --help         Print help
```

## Profiles

| Profile | Description |
|---------|-------------|
| `base` | Minimal sandbox, always included |
| `online` | Full network access |
| `localhost` | Localhost-only network |
| `node` | Node.js/npm (allows npm registry) |
| `python` | Python/pip (allows PyPI) |
| `rust` | Rust/Cargo (allows crates.io) |
| `go` | Go modules (allows proxy.golang.org) |
| `claude` | Claude Code support |
| `gpg` | GPG signing support |

## Configuration

### Project Configuration

Create `.sandbox.toml` in your project root:

```bash
sx --init
```

Example configuration:

```toml
[sandbox]
inherit_global = true
profiles = ["node"]
# network = "localhost"

[filesystem]
allow_read = []
allow_write = ["/tmp/build"]
deny_read = []

[network]
allow_domains = ["api.example.com"]

[shell]
pass_env = ["NODE_ENV", "DEBUG"]
```

### Global Configuration

Create `~/.config/sx/config.toml` for system-wide defaults.

## Security Model

### Default Protections

The sandbox **blocks access** to sensitive paths by default:

- `~/.ssh` - SSH keys
- `~/.aws` - AWS credentials
- `~/.gnupg` - GPG keys
- `~/.config/gh` - GitHub CLI tokens
- `~/.netrc` - Network credentials
- `~/.docker/config.json` - Docker credentials
- `~/Documents`, `~/Desktop`, `~/Downloads` - Personal files

### Network Modes

| Mode | Description |
|------|-------------|
| `offline` | All network blocked (default) |
| `localhost` | Only 127.0.0.1 allowed |
| `online` | Full network access |

### How It Works

1. **Reads**: Allowed globally, specific sensitive paths denied
2. **Writes**: Denied by default, allowed for working directory and temp
3. **Network**: Controlled via Seatbelt network rules

This model ensures programs can execute (read system libraries, binaries) while protecting credentials and restricting where data can be written or sent.

## Shell Integration

Copy the appropriate shell integration script to your config directory:

### Zsh
```bash
# Copy to your zsh config directory
cp shell/sx.zsh ~/.config/zsh/  # or wherever you keep zsh scripts

# Add to ~/.zshrc
source ~/.config/zsh/sx.zsh
```

### Bash
```bash
# Add to ~/.bashrc
source /path/to/sandbox-shell/shell/sx.bash
```

### Fish
```fish
# Copy to fish functions
cp shell/sx.fish ~/.config/fish/conf.d/
```

### Features
- Prompt indicator showing sandbox mode (offline/online/localhost)
- Tab completion for profiles and options
- Aliases: `sxo` (online), `sxl` (localhost), `sxn` (node), etc.

## Examples

### Safe npm install
```bash
# Offline - package must be cached or bundled
sx -- npm install

# Online - allows npm registry
sx online node -- npm install
```

### Run untrusted code
```bash
# Clone and run in isolated sandbox
git clone https://github.com/someone/sketchy-repo.git
cd sketchy-repo
sx -- ./build.sh
```

### Development server
```bash
# Allow localhost for dev server, block external network
sx localhost node -- npm run dev
```

### Check what would be allowed
```bash
sx --explain online node
sx --dry-run online node
```

## Development

```bash
# Run tests
cargo test

# Run specific test
cargo test test_name

# Build debug
cargo build

# Run from source
cargo run -- --help
cargo run -- echo "test"
```

## License

MIT

## Contributing

Contributions welcome! Please read the security model before submitting PRs that modify sandbox behavior.
