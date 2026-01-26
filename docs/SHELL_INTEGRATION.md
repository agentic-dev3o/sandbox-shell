# Shell Integration

`sx` provides shell integration for a seamless development experience with prompt indicators and convenient aliases.

## Quick Setup

### Zsh

Add to `~/.zshrc`:

```bash
eval "$(sx shell zsh)"
```

### Bash

Add to `~/.bashrc`:

```bash
eval "$(sx shell bash)"
```

### Fish

Add to `~/.config/fish/config.fish`:

```fish
sx shell fish | source
```

## Features

### Prompt Indicator

When inside a sandbox, your prompt shows an indicator:

```
[sx] ~/projects/myapp $
```

With colored output (default):
- 🔒 Yellow indicator for visual feedback
- Clear distinction between sandboxed and normal shells

### Aliases

The shell integration provides convenient aliases:

| Alias | Command | Description |
|-------|---------|-------------|
| `sxs` | `sx` | Start sandbox shell |
| `sxe` | `sx --explain` | Explain what sandbox would do |
| `sxd` | `sx --dry-run` | Show generated profile |
| `sxo` | `sx --profile online` | Start with network access |
| `sxl` | `sx --profile localhost` | Start with localhost access |

### Tab Completion

Completions for all `sx` commands and options:

```bash
sx --<TAB>
--config      --dry-run     --explain     --help
--network     --profile     --shell       --version
```

## Manual Integration

If you prefer manual setup:

### Prompt Function

```bash
# Check if inside sandbox
sx_prompt() {
    if [ -n "$SX_SANDBOX" ]; then
        echo "[sx] "
    fi
}

# Add to prompt
PS1='$(sx_prompt)'"$PS1"
```

### Environment Detection

Inside a sandbox, these environment variables are set:

| Variable | Value | Description |
|----------|-------|-------------|
| `SX_SANDBOX` | `1` | Indicates sandbox is active |
| `SX_PROFILE` | Profile name | Current profile(s) |
| `SX_NETWORK` | Mode | Network mode |

### Example Usage

```bash
# Check if in sandbox
if [ "$SX_SANDBOX" = "1" ]; then
    echo "Running in sandbox with profile: $SX_PROFILE"
fi

# Conditional behavior
if [ "$SX_NETWORK" = "offline" ]; then
    echo "Network is disabled"
fi
```

## Customization

### Disable Prompt Indicator

In config:

```toml
[sandbox]
prompt_indicator = false
```

Or via CLI:

```bash
sx --no-prompt-indicator
```

### Custom Prompt Style

```bash
# Plain text (no colors)
export SX_PROMPT_STYLE=plain

# Colored (default)
export SX_PROMPT_STYLE=colored
```

### Custom Indicator

Override the default indicator:

```bash
export SX_PROMPT_INDICATOR="🔒 "
```

## Troubleshooting

### Prompt not showing

1. Ensure shell integration is sourced
2. Check `$SX_SANDBOX` is set inside sandbox
3. Verify prompt modification order

### Completions not working

1. Ensure completion system is initialized
2. Check shell integration is loaded
3. Restart shell after changes

### Slow startup

Shell integration is minimal. If startup is slow:
1. Check for network calls in shell config
2. Profile with `time sx shell zsh`
