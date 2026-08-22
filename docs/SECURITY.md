# Security Model

`sx` isolates processes with the sandbox the kernel already provides — Seatbelt
(`sandbox-exec`) on macOS, Landlock on Linux. Deny-by-default on both.

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

#### The working directory is an exception

The working directory gets full read/write access, and that grant is applied
*after* the deny rules on both backends. A deny that lives inside it therefore
has no effect. This is deliberate — a project under `~/Documents` still has to
build — but it has a sharp edge:

```bash
cd ~ && sx        # working directory is $HOME, so every deny above is void
```

Run from `$HOME`, the sandbox no longer protects `~/.ssh` or `~/.aws` from the
code it runs. `sx` prints a warning when this happens, and `sx --explain` marks
the affected entries `(OVERRIDDEN by the working directory)`. Run `sx` from the
project you actually want to sandbox, not from your home directory.

#### `.sandbox.toml` is trusted input

Project config is read from the working directory, which the sandbox grants
write access to. Code running under `sx` can therefore rewrite `.sandbox.toml`
and widen its own policy on the *next* run — the same trust you already extend
to a `Makefile` or an npm `postinstall`. Review it like one, especially in
repositories you did not write.

### Network Isolation

| Mode | Effect |
|------|--------|
| `offline` (default) | All blocked |
| `localhost` | 127.0.0.1 only |
| `online` | Full access |

Even with `online`, your credentials can't be read. Can't exfiltrate what you can't see.

### Setuid/Setgid Execution

Setuid/setgid binaries (e.g., `/bin/ps`, `/usr/bin/newgrp`) are denied by default. Seatbelt raises `forbidden-exec-sugid` when a sandboxed process tries to execute one.

Opt in selectively via `allow_exec_sugid`:

| Mode | Seatbelt rule |
|------|---------------|
| Deny (default) | No rule — blocked by `(deny default)` |
| Allow all | `(allow process-exec* (with no-sandbox))` |
| Specific paths | `(allow process-exec (with no-sandbox) (literal "/bin/ps"))` |

Paths are validated against injection attacks (no quotes, newlines, or null bytes).

### Environment Sanitization

Blocked by default:
- `AWS_*`
- `*_SECRET*`
- `*_PASSWORD*`
- `*_KEY`

Dynamic-loader variables (`LD_*`, `DYLD_*`) are dropped unconditionally and
cannot be re-added through `set_env`. They inject code into a process before its
`main` runs — including the sandbox launcher itself, where a preloaded library
could stop the policy from being applied at all.

## Linux Enforcement

Linux has no single mechanism equivalent to Seatbelt, so `sx` composes two.

### Filesystem: Landlock

Landlock is an unprivileged, per-process LSM policy that survives `exec` and is
inherited by every descendant. Like Seatbelt — and unlike a mount namespace — it
does not change the filesystem *view*: denied paths still exist and return
`EACCES`, so error messages stay recognisable.

`sx` refuses to run if the kernel reports that Landlock was not enforced. A
sandbox that silently does not sandbox is worse than no sandbox.

Access rights are requested at ABI 3 (the v1 file rights plus `Refer` for
cross-directory renames and `Truncate`), on a best-effort basis so older kernels
still get everything they support. `IoctlDev` (ABI 5) is deliberately left
unhandled, which keeps device ioctls implicitly allowed and mirrors Seatbelt's
global `(allow file-ioctl)` — without it, terminal control breaks. Signal
scoping (ABI 6) is requested, matching Seatbelt's `(allow signal (target self))`.

### Handing the policy to the launcher

The launcher receives the policy through its environment, set by `sx` at
`execve` time. It is deliberately not a file: the sandbox grants write access to
`/tmp`, so a policy file there could be swapped by a concurrent sandboxed
process between the moment `sx` writes it and the moment the launcher reads it.
An environment set by the parent has no such window. The launcher clears the
variable before `exec`, so the sandboxed program never sees it.

### Emulating `deny_read`

Landlock is **allow-list only**: it has no deny rules and no last-match-wins
ordering. `sx` emulates denies by *subtraction* — when an allowed hierarchy
contains a denied path, the hierarchy is expanded into its siblings so the
denied subtree simply never receives a rule.

```
allow_read = ["~"], deny_read = ["~/.aws"]

  becomes rules for  ~/projects, ~/.bashrc, ~/dev, ...   (everything but ~/.aws)
  plus listing on    ~
```

A directory that cannot be enumerated contributes no rules — it fails closed.

Two details that matter for correctness:

- **Symlinks are resolved before a rule is written.** Landlock registers rules
  against the inode a path resolves to, so a link named outside a denied subtree
  would otherwise hand that subtree straight back.
- **A `deny_read` glob keeps its directory carved even when it matches nothing
  yet.** Patterns are resolved once, so a directory containing a deny pattern is
  always expanded entry by entry; a file created afterwards falls outside every
  rule instead of inheriting a broad grant.

A consequence of Landlock's design: a symlink *inside* an allowed directory that
points outside it is not reachable, because the target is not beneath any rule.
Allow the target path explicitly if you need it.

### Network: namespaces, or seccomp

Landlock only gained TCP controls in ABI 4, and they filter by port rather than
address, so they cannot express `sx`'s network modes. Network isolation uses
namespaces instead:

| Mode | Mechanism |
|------|-----------|
| `online` | no restriction |
| `offline` | empty network namespace; seccomp rejecting `AF_INET`/`AF_INET6`/`AF_PACKET` sockets where unprivileged user namespaces are blocked |
| `localhost` | network namespace with its own loopback interface |

If neither mechanism can be applied, `sx` refuses to run rather than execute an
"offline" command with live network access.

### Setuid on Linux

Unprivileged Landlock requires `no_new_privs`, so a setuid binary executed
inside the sandbox never elevates. This is stricter than the macOS default, and
it means `allow_exec_sugid` has no effect on Linux — `--dry-run` and `--explain`
say so when it is configured.

## Platform Differences

Same CLI, same config, same deny-by-default model. These behaviours differ:

| Behaviour | macOS | Linux |
|-----------|-------|-------|
| `deny_read` and directory listings | names and contents both denied | contents denied; **names may still be listable** when a parent grants listing, because Landlock rules are always hierarchical |
| `localhost` network | filters the *host* loopback, so a sandboxed server is reachable from the host | the sandbox gets its **own private loopback**; sandboxed processes reach each other, host-local services stay out of reach (strictly tighter, but a real difference) |
| Glob paths (`/tmp/foo*`) | matched at access time | resolved once when the policy is built; paths created later do not match |
| `allow_list_dirs` | exactly the named directory | the directory and everything beneath it (names only) |
| `allow_exec_sugid` | per-binary opt-in | no effect; setuid never elevates |
| Raw `[seatbelt]` rules | applied | ignored |
| Supplementary groups | unchanged | dropped in `offline`/`localhost`, because entering a user namespace maps only your own uid/gid |
| `--trace` | streams denials from the unified log | unavailable; Landlock denials only reach the kernel audit log, which needs privileges. Use `--dry-run` / `--explain` instead |

### `/proc` exposure on Linux

The Linux base profile grants read access to `/proc`, which most runtimes
require. That also exposes `/proc/<pid>/environ` and `/proc/<pid>/cmdline` for
**your own other processes**, so secrets exported into an unrelated shell are
readable from inside the sandbox.

This is comparable to macOS, where the profile grants `(allow sysctl-read)`.
Narrowing it needs a PID namespace with a private `/proc` mount, which is a
container-shaped change and is not done today. If it matters to you, avoid
exporting long-lived secrets into your interactive shell environment, or drop
`/proc` from `allow_read` and add back the specific files your tools need.

## Generated Seatbelt Profile

```scheme
(version 1)
(deny default)

; Process operations
(allow process-fork)
(allow process-exec)
(allow signal (target self))

; Setuid/setgid execution denied (default)
; Or, if allow_exec_sugid = ["/bin/ps"]:
; (allow process-exec (with no-sandbox) (literal "/bin/ps"))

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

## Generated Landlock Policy

`sx --dry-run` prints the resolved rule set, the network plan for this machine,
and the paths that were denied:

```
# sx sandbox policy (landlock)
# kernel Landlock ABI: 6
# network: offline -> private network namespace
# setuid/setgid execution: never elevates (no_new_privs is always set)

# denied for reading (no rule is emitted for these paths)
# deny /home/me/.aws

# r = read files + execute, l = list directory, w = create/modify/delete
rl- /usr
rl- /proc
rlw /home/me/project
r-w /dev/null
```

## Limitations

1. **Root bypass** - Root can escape any sandbox
2. **Kernel bugs** - Sandbox depends on kernel security
3. **Side channels** - Timing attacks not prevented
4. **Existing processes** - Only affects new processes
5. **Same-user process introspection** (Linux) - see [`/proc` exposure](#proc-exposure-on-linux)

## Best Practices

1. Default to `offline` unless network required
2. Use `localhost` for dev servers
3. Review custom profiles before trusting them
4. Use `--trace` to debug denials (macOS), or `--dry-run` / `--explain` (both)
5. On Linux, check `sx --explain` reports a Landlock ABI — if it does not, the kernel cannot enforce the sandbox
