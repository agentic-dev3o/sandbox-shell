# Security Model

`sx` uses macOS Seatbelt (`sandbox-exec`) to isolate processes. Deny-by-default.

## Threat Model

Supply chain attacks. That one compromised npm package in your dependency tree running a postinstall script, trying to exfiltrate `~/.aws` or plant malware.

`sx` protects against:
- **Credential theft** - Can't read `~/.ssh`, `~/.aws`, `~/.docker/config.json`
- **Data exfiltration** - Filesystem is deny-by-default, network is offline by default
- **Malware drops** - Write access limited to working directory and `/tmp`

## Security Layers

### Deny by Default

Everything blocked unless explicitly allowed:

```scheme
(version 1)
(deny default)
```

### Filesystem Isolation

| Category | Access |
|----------|--------|
| Working directory | Read/write |
| System binaries (`/usr`, `/bin`) | Read-only |
| Temp (`/tmp`) | Read/write |
| Everything else | Denied |

**Always denied** (even if you allow `~`):

| Path | What |
|------|------|
| `~/.ssh` | SSH keys |
| `~/.aws` | AWS credentials |
| `~/.docker/config.json` | Docker credentials |
| `~/Documents`, `~/Desktop`, `~/Downloads` | Personal files |

Everything else (`~/.config/gh`, `~/.netrc`, `~/.gnupg`…) is blocked by deny-by-default. Use profiles like `gpg` to allow specific paths.

### Network Isolation

| Mode | Effect |
|------|--------|
| `offline` (default) | All blocked |
| `localhost` | 127.0.0.1 only |
| `online` | Full access |

Even with `online`, your credentials can't be read. Can't exfiltrate what you can't see.

### Environment Sanitization

Blocked by default:
- `AWS_*`
- `*_SECRET*`
- `*_PASSWORD*`
- `*_KEY`

## Generated Seatbelt Profile

```scheme
(version 1)
(deny default)

; Process operations
(allow process-fork)
(allow process-exec)
(allow signal (target self))

; Required for path resolution
(allow file-read* (literal "/"))
(allow file-read-metadata)  ; Required for DNS resolution

; Working directory
(allow file* (subpath "/path/to/project"))

; Denied paths (override allows)
(deny file-read* (subpath "/Users/me/.ssh"))
(deny file-read* (subpath "/Users/me/.aws"))

; System paths
(allow file-read* (subpath "/usr"))
(allow file-read* (subpath "/bin"))

; Network (based on mode)
; offline: nothing
; localhost: (allow network-outbound (to ip "localhost:*"))
; online: (allow network*)
```

## Limitations

1. **Root bypass** - Root can escape any sandbox
2. **Kernel bugs** - Sandbox depends on kernel security
3. **Side channels** - Timing attacks not prevented
4. **Existing processes** - Only affects new processes

## Best Practices

1. Default to `offline` unless network required
2. Use `localhost` for dev servers
3. Review custom profiles before trusting them
4. Use `--trace` to debug denials
