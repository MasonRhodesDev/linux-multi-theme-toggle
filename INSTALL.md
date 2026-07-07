# LMTT Installation

## Arch Linux

Add the `[mason]` repo to `/etc/pacman.conf`, then install:

```ini
[mason]
SigLevel = Optional TrustAll
Server = https://masonrhodesdev.github.io/arch-repo/x86_64
```

```bash
sudo pacman -Sy lmtt
```

## Fedora

```bash
sudo dnf copr enable solaris765/lmtt
sudo dnf install lmtt
```

## Build from Source

```bash
git clone https://github.com/MasonRhodesDev/linux-multi-theme-toggle.git
cd linux-multi-theme-toggle
sudo make install PREFIX=/usr
```

## Usage

### Configuration TUI
```bash
lmtt config
```
Opens an interactive TUI with three sections:
- **General** — wallpaper path (supports env vars), default mode, scheme type,
  matugen settings, plus Notifications / Performance / Cache / Logging
- **Light Profile** and **Dark Profile** — per-mode GTK/icon/cursor themes,
  fonts, VSCode theme, opacity, blur

Modules aren't configured from the TUI — toggle one by editing
`[modules.<name>] enabled = false` in the config. Changes are auto-saved to
`~/.config/lmtt/config.toml`.

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
