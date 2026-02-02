# Shell Integration

Prompt indicators, tab completion, aliases.

## Setup

### Zsh

Add to `~/.zshrc`:

```bash
source $(brew --prefix)/share/sx/sx.zsh
# Or from source:
# source /path/to/sandbox-shell/shell/sx.zsh
```

### Bash

Add to `~/.bashrc`:

```bash
source $(brew --prefix)/share/sx/sx.bash
```

### Fish

```fish
cp $(brew --prefix)/share/sx/sx.fish ~/.config/fish/conf.d/
```

## Features

### Prompt Indicator

Inside a sandbox, prompt shows the mode:

```
[sx:offline] ~/projects/myapp $
[sx:localhost] ~/projects/myapp $
[sx:online] ~/projects/myapp $
```

Color-coded:
- `[sx:offline]` red - network blocked
- `[sx:localhost]` yellow - localhost only
- `[sx:online]` green - full network

### Aliases

| Alias | Command | Use case |
|-------|---------|----------|
| `sxo` | `sx online` | Full network |
| `sxl` | `sx localhost` | Dev servers |
| `sxb` | `sx bun online` | Bun with network |
| `sxr` | `sx online rust` | Rust with network |
| `sxc` | `sx online gpg claude` | Claude Code with GPG |

### Tab Completion

Profiles, options, commands:

```bash
sx --<TAB>
--config    --dry-run   --explain   --help
--offline   --online    --localhost --trace

sx <TAB>
base      online    localhost   rust
bun       claude    gpg
```

## Environment Variables

Inside a sandbox, `SANDBOX_MODE` is set:

```bash
if [[ -n "$SANDBOX_MODE" ]]; then
    echo "Running in sandbox: $SANDBOX_MODE"
fi
```

Values: `offline`, `localhost`, `online`

## Troubleshooting

### Prompt not showing

1. Check shell integration is sourced
2. Verify `$SANDBOX_MODE` is set inside sandbox
3. Check prompt order (sx prepends to existing `$PROMPT`)

### Completions not working

1. Completions need `compdef` (zsh) or `complete` (bash)
2. Restart shell after adding integration
3. Check shell integration loaded: `type _sx`
