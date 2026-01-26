# Security Model

`sx` uses macOS Seatbelt (sandbox-exec) to create a secure execution environment that restricts filesystem and network access.

## Threat Model

`sx` protects against:

1. **Malicious npm packages** - Prevents untrusted code from accessing sensitive files
2. **Supply chain attacks** - Limits damage from compromised dependencies
3. **Data exfiltration** - Network restrictions prevent unauthorized data transmission
4. **Credential theft** - Denies access to SSH keys, cloud credentials, and secrets

## Security Layers

### Deny by Default

All access is denied by default. Only explicitly allowed operations succeed:

```
(version 1)
(deny default)
```

### Filesystem Isolation

| Category | Default |
|----------|---------|
| Working directory | Full access |
| System binaries | Read-only |
| Temp directories | Read/Write |
| Sensitive directories | Denied |

**Denied by default:**
- `~/.ssh` - SSH keys
- `~/.aws` - AWS credentials
- `~/.gnupg` - GPG keys
- `~/.config/gh` - GitHub CLI tokens
- `~/.netrc` - Network credentials
- `~/.docker/config.json` - Docker credentials
- `~/Documents`, `~/Desktop`, `~/Downloads` - Personal files

### Network Isolation

Three modes control network access:

| Mode | Allowed |
|------|---------|
| `offline` | No network (default) |
| `localhost` | 127.0.0.1 only |
| `online` | All network |

### Environment Sanitization

Sensitive environment variables are blocked by default:
- `AWS_*` - AWS credentials
- `*_SECRET*` - Secrets
- `*_PASSWORD*` - Passwords
- `*_KEY` - API keys

## Seatbelt Profile

The generated Seatbelt profile includes:

```scheme
(version 1)
(deny default)

; Process operations
(allow process-fork)
(allow process-exec)
(allow signal (target self))

; System read access
(allow sysctl-read)
(allow file-read-metadata)

; Working directory (full access)
(allow file* (subpath "/path/to/project"))

; Denied paths (override allows)
(deny file-read* (subpath "/Users/me/.ssh"))
(deny file-read* (subpath "/Users/me/.aws"))

; System paths
(allow file-read* (subpath "/usr"))
(allow file-read* (subpath "/bin"))

; Network (based on mode)
; offline: (nothing)
; localhost: (allow network-outbound (to ip "localhost:*"))
; online: (allow network*)
```

## Security Verification

Run the security test suite:

```bash
./scripts/test-security.sh
```

Tests verify:
1. Sensitive directories are denied
2. Default network is offline
3. Deny rules take precedence
4. Profile composition is secure
5. Environment sanitization works

## Limitations

1. **Root bypass** - Sandbox can be bypassed with root privileges
2. **Kernel vulnerabilities** - Sandbox depends on kernel security
3. **Covert channels** - Side-channel attacks are not prevented
4. **Existing processes** - Only affects new processes, not existing ones

## Best Practices

1. **Always use offline mode** unless network is required
2. **Use localhost mode** for local development servers
3. **Audit custom profiles** before use
4. **Review violations** in log file
5. **Keep sx updated** for security fixes

## Reporting Vulnerabilities

Report security issues privately. Do not open public issues for vulnerabilities.
