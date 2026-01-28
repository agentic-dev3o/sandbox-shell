# Homebrew Tap Repository Specification

**Project:** `homebrew-sx`
**Purpose:** Homebrew formula distribution for the `sx` (sandbox-shell) CLI tool
**Target Platform:** macOS only
**Status:** To Be Created

---

## Table of Contents

1. [Overview](#1-overview)
2. [Repository Structure](#2-repository-structure)
3. [Initial Setup](#3-initial-setup)
4. [Formula Specification](#4-formula-specification)
5. [CI/CD Configuration](#5-cicd-configuration)
6. [Automated Updates](#6-automated-updates)
7. [Testing Requirements](#7-testing-requirements)
8. [Maintenance Procedures](#8-maintenance-procedures)
9. [Security Considerations](#9-security-considerations)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Overview

### 1.1 Purpose

The `homebrew-sx` repository hosts the Homebrew formula for distributing the `sx` CLI tool. This repository is automatically updated by the main `sx` repository's release workflow.

### 1.2 User Installation Flow

```bash
# First-time setup (adds tap)
brew tap agentic-dev3o/sx

# Install sx
brew install sx

# Future upgrades
brew upgrade sx
```

### 1.3 Dependencies

| Dependency | Type | Purpose |
|------------|------|---------|
| Rust toolchain | Build-time | Compiles sx from source |
| macOS | Runtime | Required (Seatbelt is macOS-only) |

### 1.4 Integration with Main Repository

```
┌────────────────────────────────┐
│ sx repository                  │
│ (main application)             │
│                                │
│ .github/workflows/release.yml  │
│   ├── Bumps version            │
│   ├── Creates GitHub release   │
│   ├── Calculates SHA256        │
│   └── Updates homebrew-sx ─────┼──────┐
└────────────────────────────────┘      │
                                        ▼
                          ┌─────────────────────────────┐
                          │ homebrew-sx repository      │
                          │                             │
                          │ Formula/sx.rb               │
                          │   ├── version: X.Y.Z        │
                          │   ├── sha256: <checksum>    │
                          │   └── url: release tarball  │
                          └─────────────────────────────┘
```

---

## 2. Repository Structure

```
homebrew-sx/
├── README.md                    # Installation instructions
├── LICENSE                      # Same license as main sx repo
├── Formula/
│   └── sx.rb                    # Main formula file
└── .github/
    └── workflows/
        └── audit.yml            # Formula validation on PR/push
```

### 2.1 File Descriptions

| File | Purpose | Update Frequency |
|------|---------|------------------|
| `README.md` | User-facing installation docs | Rarely |
| `LICENSE` | Legal terms (match main repo) | Never |
| `Formula/sx.rb` | Homebrew formula definition | Every release |
| `.github/workflows/audit.yml` | CI validation | Rarely |

---

## 3. Initial Setup

### 3.1 Create Repository

1. Create new GitHub repository: `homebrew-sx`
   - Visibility: **Public** (required for Homebrew taps)
   - Initialize with README: **No** (we'll create our own)
   - License: Match main `sx` repository

2. Clone locally:
   ```bash
   git clone https://github.com/agentic-dev3o/homebrew-sx.git
   cd homebrew-sx
   ```

### 3.2 Create Directory Structure

```bash
mkdir -p Formula .github/workflows
```

### 3.3 Create README.md

```markdown
# Homebrew Tap for sx

This is a [Homebrew](https://brew.sh) tap for [sx](https://github.com/agentic-dev3o/sx), a CLI that wraps shell sessions in macOS Seatbelt sandboxes.

## Requirements

- macOS (sx uses macOS Seatbelt, which is not available on Linux)
- Homebrew

## Installation

```bash
brew tap agentic-dev3o/sx
brew install sx
```

## Upgrading

```bash
brew upgrade sx
```

## Uninstalling

```bash
brew uninstall sx
brew untap agentic-dev3o/sx
```

## Troubleshooting

### Build fails

Ensure you have Xcode Command Line Tools installed:

```bash
xcode-select --install
```

### Issues

Report issues at: https://github.com/agentic-dev3o/sx/issues
```

### 3.4 Create LICENSE

Copy the exact same license file from the main `sx` repository.

### 3.5 Initial Commit

```bash
git add .
git commit -m "Initial tap setup"
git push origin main
```

---

## 4. Formula Specification

### 4.1 Formula File: `Formula/sx.rb`

```ruby
# typed: false
# frozen_string_literal: true

class Sx < Formula
  desc "Sandbox shell sessions with macOS Seatbelt"
  homepage "https://github.com/agentic-dev3o/sx"
  url "https://github.com/agentic-dev3o/sx/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256_WILL_BE_UPDATED_BY_RELEASE_WORKFLOW"
  license "MIT"
  head "https://github.com/agentic-dev3o/sx.git", branch: "main"

  # macOS only - Seatbelt is not available on Linux
  depends_on :macos

  # Rust toolchain for building from source
  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  def caveats
    <<~EOS
      sx restricts filesystem and network access using macOS Seatbelt.

      Quick start:
        sx echo "sandboxed command"     # Run single command
        sx --profile rust cargo build   # Use rust profile
        sx --dry-run online node        # Preview sandbox rules

      Initialize shell integration:
        sx init bash >> ~/.bashrc
        sx init zsh >> ~/.zshrc

      Configuration:
        ~/.config/sx/config.toml        # Global config
        .sandbox.toml                   # Project config

      Documentation: https://github.com/agentic-dev3o/sx
    EOS
  end

  test do
    # Verify binary runs and shows version
    assert_match version.to_s, shell_output("#{bin}/sx --version")

    # Verify help output
    assert_match "sandbox", shell_output("#{bin}/sx --help")

    # Verify dry-run works (doesn't require actual sandboxing)
    output = shell_output("#{bin}/sx --dry-run echo test")
    assert_match "file-read", output  # Seatbelt profile contains file-read rules
  end
end
```

### 4.2 Formula Fields Reference

| Field | Description | Auto-Updated |
|-------|-------------|--------------|
| `desc` | Short description (< 80 chars) | No |
| `homepage` | Project homepage URL | No |
| `url` | Source tarball URL (GitHub release) | **Yes** |
| `sha256` | SHA256 checksum of tarball | **Yes** |
| `license` | SPDX license identifier | No |
| `head` | Git URL for `--HEAD` installs | No |
| `depends_on` | Build/runtime dependencies | No |
| `install` | Build instructions | No |
| `caveats` | Post-install message to user | Rarely |
| `test` | Verification commands | Rarely |

### 4.3 Version Interpolation (Alternative)

For cleaner updates, use version interpolation:

```ruby
url "https://github.com/agentic-dev3o/sx/archive/refs/tags/v#{version}.tar.gz"
```

**Note:** This requires updating only the `version` line during releases, but the current sed-based update in the release workflow handles explicit URLs fine.

---

## 5. CI/CD Configuration

### 5.1 Formula Audit Workflow

Create `.github/workflows/audit.yml`:

```yaml
name: Audit Formula

on:
  push:
    branches: [main]
    paths:
      - 'Formula/**'
  pull_request:
    branches: [main]
    paths:
      - 'Formula/**'

jobs:
  audit:
    runs-on: macos-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Homebrew
        uses: Homebrew/actions/setup-homebrew@master

      - name: Tap local repository
        run: |
          mkdir -p $(brew --repository)/Library/Taps/agentic-dev3o
          ln -s $GITHUB_WORKSPACE $(brew --repository)/Library/Taps/agentic-dev3o/homebrew-sx

      - name: Run brew audit
        run: brew audit --strict Formula/sx.rb

      - name: Run brew style
        run: brew style Formula/sx.rb

      - name: Test formula installation
        run: |
          brew install --build-from-source Formula/sx.rb
          brew test sx
```

### 5.2 Workflow Triggers

| Trigger | Condition | Actions |
|---------|-----------|---------|
| Push to main | Formula files changed | Audit + Style check |
| Pull request | Formula files changed | Audit + Style check + Build test |
| Release workflow (from sx repo) | New release | Direct commit (bypasses PR) |

---

## 6. Automated Updates

### 6.1 How Updates Work

The main `sx` repository's release workflow updates this tap:

1. Release workflow calculates SHA256 of source tarball
2. Clones `homebrew-sx` using `TAP_REPO_TOKEN`
3. Updates `url` and `sha256` in `Formula/sx.rb` via sed
4. Commits and pushes changes

### 6.2 Update Script (in sx repo release workflow)

```bash
# Clone tap repo
git clone "https://x-access-token:$TAP_REPO_TOKEN@github.com/agentic-dev3o/homebrew-sx.git" tap
cd tap

# Update formula
sed -i '' "s|url \".*\"|url \"https://github.com/agentic-dev3o/sx/archive/refs/tags/v$VERSION.tar.gz\"|" Formula/sx.rb
sed -i '' "s|sha256 \".*\"|sha256 \"$SHA256\"|" Formula/sx.rb

# Commit and push
git config user.name "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"
git add Formula/sx.rb
git commit -m "sx $VERSION"
git push
```

### 6.3 Required Secrets (in sx repository)

| Secret | Purpose | Scope |
|--------|---------|-------|
| `TAP_REPO_TOKEN` | Fine-grained PAT for pushing to homebrew-sx | Contents: Read/Write on homebrew-sx only |

### 6.4 Token Setup Instructions

1. Go to GitHub → Settings → Developer settings → Personal access tokens → Fine-grained tokens
2. Click "Generate new token"
3. Configure:
   - **Token name:** `sx-tap-update`
   - **Expiration:** 1 year (set reminder to rotate)
   - **Repository access:** Only select repositories → `homebrew-sx`
   - **Permissions:**
     - Contents: Read and write
     - Metadata: Read (auto-selected)
4. Generate token and copy immediately
5. Go to `sx` repository → Settings → Secrets → Actions
6. Add new secret: `TAP_REPO_TOKEN` with the token value

---

## 7. Testing Requirements

### 7.1 Local Testing Before Initial Publish

```bash
# From homebrew-sx directory
cd /path/to/homebrew-sx

# Audit formula
brew audit --strict Formula/sx.rb

# Check Ruby style
brew style Formula/sx.rb

# Test installation from source
brew install --build-from-source Formula/sx.rb

# Run formula tests
brew test sx

# Verify installation
sx --version
sx --help
sx --dry-run echo "test"

# Cleanup
brew uninstall sx
```

### 7.2 Post-Release Verification

After each release, verify:

```bash
# Update tap
brew update

# Check new version is available
brew info sx

# Upgrade
brew upgrade sx

# Verify new version
sx --version
```

### 7.3 Test Matrix

| Test | Command | Expected Result |
|------|---------|-----------------|
| Version output | `sx --version` | Displays `sx X.Y.Z` |
| Help output | `sx --help` | Contains "sandbox" |
| Dry-run | `sx --dry-run echo test` | Shows Seatbelt profile |
| Basic execution | `sx echo "hello"` | Outputs "hello" |

---

## 8. Maintenance Procedures

### 8.1 Routine Tasks

| Task | Frequency | Procedure |
|------|-----------|-----------|
| Rotate TAP_REPO_TOKEN | Annually | Regenerate fine-grained PAT, update secret |
| Audit formula | Per release (automated) | CI runs automatically |
| Review caveats | Major releases | Update usage instructions if needed |
| Check Homebrew compatibility | Quarterly | Test with latest Homebrew |

### 8.2 Manual Formula Update (Emergency)

If automated update fails:

```bash
# Clone tap
git clone https://github.com/agentic-dev3o/homebrew-sx.git
cd homebrew-sx

# Get SHA256 of release tarball
curl -sL "https://github.com/agentic-dev3o/sx/archive/refs/tags/vX.Y.Z.tar.gz" | shasum -a 256

# Edit Formula/sx.rb with new url and sha256

# Test locally
brew audit --strict Formula/sx.rb
brew install --build-from-source Formula/sx.rb
brew test sx

# Commit and push
git add Formula/sx.rb
git commit -m "sx X.Y.Z"
git push
```

### 8.3 Deprecating a Version

If a version has critical bugs:

```bash
# Add deprecation notice to formula
# Edit Formula/sx.rb, add after license line:
deprecate! date: "YYYY-MM-DD", because: "critical security issue in vX.Y.Z"
```

### 8.4 Adding Pre-built Bottles (Optional Future Enhancement)

For faster installation, add pre-built binaries:

```ruby
bottle do
  sha256 cellar: :any_skip_relocation, arm64_sonoma: "SHA256_HERE"
  sha256 cellar: :any_skip_relocation, arm64_ventura: "SHA256_HERE"
  sha256 cellar: :any_skip_relocation, sonoma: "SHA256_HERE"
  sha256 cellar: :any_skip_relocation, ventura: "SHA256_HERE"
end
```

**Note:** Bottles require additional CI setup with `brew test-bot`. Defer this to a future enhancement.

---

## 9. Security Considerations

### 9.1 Repository Access

| Entity | Access Level | Purpose |
|--------|--------------|---------|
| Repository owner | Admin | Full control |
| github-actions[bot] | Write (via PAT) | Automated formula updates |
| Public | Read | Homebrew tap access |

### 9.2 Token Security

- **Scope:** Fine-grained PAT with minimal permissions (Contents: R/W on homebrew-sx only)
- **Storage:** GitHub Actions secret (encrypted)
- **Rotation:** Annual rotation required
- **Audit:** Review token usage in GitHub security log

### 9.3 Formula Security

- **Source URL:** Always use `https://github.com/` (verified domain)
- **SHA256:** Automatically verified by Homebrew during install
- **No external resources:** Formula doesn't fetch additional files

### 9.4 Branch Protection (Recommended)

Configure on `main` branch:
- Require pull request reviews: **Disabled** (allows automated updates)
- Require status checks: **Enabled** (audit workflow must pass)
- Allow force pushes: **Disabled**
- Allow deletions: **Disabled**

---

## 10. Troubleshooting

### 10.1 Common Issues

#### Formula audit fails

```
Error: sx: sha256 mismatch
```

**Cause:** SHA256 doesn't match actual tarball
**Fix:** Recalculate SHA256:
```bash
curl -sL "https://github.com/agentic-dev3o/sx/archive/refs/tags/vX.Y.Z.tar.gz" | shasum -a 256
```

#### Build fails on user machine

```
Error: No available formula with the name "rust"
```

**Cause:** Old Homebrew version
**Fix:** User should run:
```bash
brew update
brew install sx
```

#### Token expired

```
remote: Permission to agentic-dev3o/homebrew-sx.git denied
```

**Cause:** TAP_REPO_TOKEN expired
**Fix:** Generate new PAT and update secret in sx repository

### 10.2 Debug Commands

```bash
# View formula info
brew info sx

# View formula source
brew cat sx

# Verbose installation
brew install --verbose --debug sx

# Check tap status
brew tap-info agentic-dev3o/sx
```

### 10.3 Support Channels

- **Issues:** https://github.com/agentic-dev3o/sx/issues
- **Formula-specific issues:** Tag with `homebrew` label

---

## Appendix A: Quick Reference

### Commands for Users

```bash
brew tap agentic-dev3o/sx    # Add tap
brew install sx                  # Install
brew upgrade sx                  # Upgrade
brew uninstall sx                # Remove
brew untap agentic-dev3o/sx  # Remove tap
```

### Commands for Maintainers

```bash
brew audit --strict Formula/sx.rb    # Validate formula
brew style Formula/sx.rb             # Check Ruby style
brew install --build-from-source Formula/sx.rb  # Test build
brew test sx                         # Run tests
```

### Key URLs

| Resource | URL |
|----------|-----|
| Main repository | `https://github.com/agentic-dev3o/sx` |
| Tap repository | `https://github.com/agentic-dev3o/homebrew-sx` |
| Release tarball | `https://github.com/agentic-dev3o/sx/archive/refs/tags/vX.Y.Z.tar.gz` |

---

## Appendix B: Checklist

### Initial Setup Checklist

- [ ] Create `homebrew-sx` repository on GitHub (public)
- [ ] Create `Formula/sx.rb` with placeholder SHA256
- [ ] Create `README.md` with installation instructions
- [ ] Create `LICENSE` (match main repo)
- [ ] Create `.github/workflows/audit.yml`
- [ ] Generate fine-grained PAT for tap updates
- [ ] Add `TAP_REPO_TOKEN` secret to `sx` repository
- [ ] Add release workflow to `sx` repository
- [ ] Test full release cycle with v0.1.0-alpha
- [ ] Delete test release
- [ ] Perform first real release

### Per-Release Checklist (Automated)

- [ ] Version bumped in `Cargo.toml`
- [ ] Changelog generated
- [ ] GitHub release created
- [ ] Formula `url` updated
- [ ] Formula `sha256` updated
- [ ] Audit workflow passes

---

*Document version: 1.0*
*Last updated: January 2025*
