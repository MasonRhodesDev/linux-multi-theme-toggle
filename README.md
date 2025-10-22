# LMTT - Linux Matugen Theme Toggle

**High-performance, async theme switching for Hyprland/Wayland desktops** using Material You color schemes.

## Features

- ⚡ **Blazing Fast**: Async Rust implementation, modules run in parallel
- 🎨 **Material You**: Generates color schemes from wallpapers using [matugen](https://github.com/InioX/matugen)
- 🔌 **Modular**: Supports 15+ applications (Waybar, Hyprland, VSCode, Wezterm, etc.)
- 🎯 **Auto-Detection**: Only applies themes to installed applications
- 🔧 **Easy Setup**: Auto-injects config includes with `lmtt setup`
- 🧹 **Clean Uninstall**: `lmtt cleanup` removes all injected config
- ⚙️ **Highly Configurable**: TOML config at `~/.config/lmtt/config.toml`
- 🔔 **Desktop Notifications**: Optional notifications for theme changes
- 📡 **Event System**: JSON event stream for custom integrations

## Installation

### Requirements

- Rust 1.70+ (`rustup` recommended)
- [matugen](https://github.com/InioX/matugen) for color generation
- GTK 3/4 applications (optional, for `gsettings` integration)

### Build from Source

```bash
git clone https://github.com/yourusername/linux-matugen-theme-toggle.git
cd linux-matugen-theme-toggle
make install
```

Or manually:

```bash
cargo build --release
sudo cp target/release/lmtt /usr/local/bin/
```

## Quick Start

1. **Initialize config**:
   ```bash
   lmtt init
   ```

2. **Edit config** at `~/.config/lmtt/config.toml`:
   ```toml
   [general]
   wallpaper = "~/Pictures/your-wallpaper.png"
   ```

3. **Run setup** (auto-configures app config files):
   ```bash
   lmtt setup
   ```

4. **Switch theme**:
   ```bash
   lmtt switch         # Toggle between light/dark
   lmtt switch dark    # Switch to dark mode
   lmtt switch light   # Switch to light mode
   ```

## Usage

### Commands

```bash
# Switch theme
lmtt switch                   # Toggle between light/dark
lmtt switch dark              # Switch to dark mode
lmtt switch light             # Switch to light mode
lmtt switch --no-notify       # Toggle without notifications

# Setup mode (configure app configs)
lmtt setup
lmtt setup --dry-run

# Cleanup mode (remove lmtt config injections)
lmtt cleanup
lmtt cleanup --module waybar  # Cleanup specific module
lmtt cleanup --dry-run

# Status and info
lmtt status
lmtt list
lmtt list --all
```

### Configuration

Config file: `~/.config/lmtt/config.toml`

```toml
[general]
wallpaper = "~/Pictures/forrest.png"
default_mode = "dark"
scheme_type = "scheme-expressive"

[notifications]
enabled = true
timeout = 5000

[modules.waybar]
enabled = true

[modules.hyprland]
enabled = true
```

**Key features**:
- **Modules enabled by default**: Apps are auto-detected and run if installed
- **Disable modules**: Set `enabled = false` to skip specific apps
- **Custom commands**: Add `command = "/path/to/script.sh"` for custom modules

## Supported Applications

| Module | Config File | Auto-Setup |
|--------|-------------|------------|
| GTK | gsettings | ✓ |
| Waybar | `style.css` | ✓ |
| Hyprland | `colors.conf` | ✓ |
| SwayNC | `style.css` | ✓ |
| Wezterm | `colors.toml` | ✓ |
| Tmux | `lmtt-colors.conf` | ✓ |
| Neovim | `lmtt-colors.lua` | ✓ |
| VSCode | `settings.json` | ✓ |
| Wofi | `style.css` | ✓ |
| Fish | `lmtt-colors.fish` | ✓ |

## Setup Mode

`lmtt setup` checks your installed applications and prompts to inject config includes:

```bash
$ lmtt setup
🔧 LMTT Setup Mode
================

✓ waybar detected
  📄 /home/user/.config/waybar/style.css
     Import lmtt colors into Waybar CSS
     ⚠ Include line missing:
     @import url('../matugen/lmtt-colors.css');

     Inject this line? [Y/n/q] y
     ✓ Injected successfully!
```

### What gets injected?

Each module adds a marked block at the top of your config:

```css
# >>> lmtt managed block - do not edit manually >>>
@import url('../matugen/lmtt-colors.css');
# <<< lmtt managed block <<<

/* Your existing config below */
```

This allows clean removal with `lmtt cleanup`.

## Cleanup/Uninstall

Remove all lmtt-injected config lines:

```bash
# Clean all modules
lmtt cleanup

# Clean specific module
lmtt cleanup --module waybar

# Dry run (see what would be removed)
lmtt cleanup --dry-run
```

This is **completely non-intrusive** - your original config files are restored.

## Architecture

```
lmtt/               # CLI binary
lmtt-core/          # Core types (Config, ColorScheme, Cache)
lmtt-modules/       # Theme modules (Waybar, Hyprland, etc.)
lmtt-platforms/     # Platform backends (GTK, XDG, Qt)
```

### Adding a Custom Module

See `lmtt-modules/src/waybar.rs` for a template. Key points:

1. Implement `ThemeModule` trait
2. Specify `binary_name()` for auto-detection
3. Implement `apply()` for theme switching
4. Implement `config_files()` for setup mode
5. Add to `ModuleRegistry` in `registry.rs`

## Performance

Typical theme switch: **~100-200ms**

- Modules run in parallel (Tokio async)
- No blocking I/O
- Wallpaper color caching (skips regeneration if unchanged)

Performance warnings for slow modules (>250ms default).

## Events & Notifications

LMTT emits 3 event types:

1. `switch_started` - Theme switch initiated
2. `switch_completed` - Theme applied successfully
3. `switch_failed` - Theme switch error

Desktop notifications are shown by default (disable with `--no-notify` or config).

## Troubleshooting

### Module not running?

```bash
# Check if app is installed
lmtt list --all

# Check config
lmtt status

# Verbose output
lmtt -v switch dark
```

### Config not applied?

```bash
# Re-run setup
lmtt setup

# Verify config file includes lmtt block
cat ~/.config/waybar/style.css
```

### Cleanup not working?

Config files must have the lmtt marker comments. If you manually edited them, you may need to manually remove the include lines.

## Migration from Bash Version

The Rust version is a drop-in replacement with improved performance:

1. Backup your current bash scripts
2. Install lmtt
3. Run `lmtt init` and `lmtt setup`
4. Test with `lmtt switch dark`

Your existing module configs should work without changes.

## Contributing

Contributions welcome! Areas needing help:

- [ ] Additional module support (Alacritty, Kitty, etc.)
- [ ] Platform backends (KDE Plasma integration)
- [ ] Documentation improvements
- [ ] Testing on different distros

## License

MIT

## Credits

- [matugen](https://github.com/InioX/matugen) - Material You color generation
- Original bash implementation by Mason
