# LMTT Installation

## Quick Install

```bash
./install.sh
```

## Manual Install

```bash
cargo build --release
cp target/release/lmtt ~/.local/bin/lmtt
```

## Usage

### Configuration TUI
```bash
lmtt config
```
Opens an interactive TUI to configure:
- Theme profiles (light/dark)
- Wallpaper path (supports `$WALLPAPER` env vars)
- Matugen settings
- Module settings
- Notifications, cache, logging

Changes are auto-saved to `~/.config/lmtt/config.toml`

### Theme Switching
```bash
lmtt switch         # Toggle between light/dark
lmtt switch light   # Switch to light theme
lmtt switch dark    # Switch to dark theme
```

### Other Commands
```bash
lmtt status         # Show current theme
lmtt list           # List available modules
lmtt init           # Create default config
lmtt setup          # Configure application configs
lmtt cleanup        # Remove lmtt injections
```

## Configuration

Config file: `~/.config/lmtt/config.toml`

You can use environment variables in the config:
```toml
[general]
wallpaper = "$WALLPAPER"  # Expanded at runtime
```

## Schema-Driven TUI Features

- ✅ Auto-generated widgets from JSON schema
- ✅ Dynamic dropdowns (GTK themes, fonts, etc.)
- ✅ Terminal theme support (respects your colors)
- ✅ Subsections for organized settings
- ✅ Auto-save on every change
- ✅ Searchable dropdowns
- ✅ External editor support (press 'e' on path fields)
