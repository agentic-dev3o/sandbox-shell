`sx` (sandbox-shell) is a Rust CLI that wraps shell sessions and commands in macOS Seatbelt sandboxes. It protects developers from malicious code in npm packages, untrusted repositories, and build scripts by restricting filesystem and network access.

**Critical Seatbelt Rules**:
1. Root literal `(allow file-read* (literal "/"))` is required for path traversal - processes need to read `/` to resolve paths
2. Seatbelt uses last-match-wins semantics when rules have matching filter types - deny rules after allow rules take precedence for nested paths (e.g., allow `/home` then deny `/home/.ssh`)
3. `(allow file-read-metadata)` must be global (no path filter) - required for `getaddrinfo()` DNS resolution to work. Without this, `curl`, Python, and other tools using the system resolver fail with "Could not resolve host" even when network is allowed. Commands like `host` and `nslookup` work without it because they use direct DNS UDP queries.

**Configuration Options**:
- `inherit_base = false` in `.sandbox.toml` skips the base profile for full custom control over allowed paths

## Programming Rules

Produce code following **LEAN, CLEAN, MINIMAL but thoughtful KISS & YAGNI principle**.
Follow TDD paradigm `red-green-refactor` for unit test.
Follow Rust Book, Rust API Guidelines, and idiomatic Rust resources.
