# LMTT Quick Start Guide

## Installation

```bash
cd ~/repos/linux-matugen-theme-toggle

# Install to ~/.local/bin (no root needed)
make install-user

# Or system-wide install (requires root)
sudo make install PREFIX=/usr
```

## First Run

```bash
# 1. Create config file
lmtt init

# 2. Edit wallpaper path
vim ~/.config/lmtt/config.toml
# Change: wallpaper = "~/Pictures/your-wallpaper.png"

# 3. Run setup (prompts to inject config includes)
lmtt setup

# 4. Switch theme!
lmtt switch         # Toggle between light/dark
lmtt switch dark    # Explicit dark mode
lmtt switch light   # Explicit light mode
```

## Daily Usage

```bash
# Toggle theme (most common)
lmtt switch

# Check current theme
lmtt status

# See what's installed
lmtt list
```

## Makefile Commands

```bash
# Build
make                    # Build optimized release binary
make debug              # Build debug binary
make clean              # Clean build artifacts

# Development
make check              # Fast compile check
make test               # Run tests
make fmt                # Format code
make lint               # Run clippy
make run ARGS='list'    # Run debug with arguments

# Install/Uninstall
make install-user       # Install to ~/.local/bin
make uninstall-user     # Remove from ~/.local/bin
sudo make install       # System install to /usr/local/bin
sudo make uninstall     # System uninstall

# Quality
make ci                 # Run all checks (check, test, lint)
make dist               # Create distribution tarball

# Help
make help               # Show all targets
```

## Common Tasks

### Change Wallpaper
```bash
vim ~/.config/lmtt/config.toml
# Update wallpaper path, then:
lmtt switch
```

### Add App Config Integration
```bash
# Check what needs setup
lmtt setup --dry-run

# Run interactive setup
lmtt setup
# Answer 'y' to inject config includes
```

### Remove App Config Integration
```bash
# Remove all lmtt config injections
lmtt cleanup

# Or remove specific app
lmtt cleanup --module waybar
```

### Disable a Module
```bash
vim ~/.config/lmtt/config.toml
# Add:
# [modules.waybar]
# enabled = false

lmtt switch  # Won't apply to waybar anymore
```

## Configuration File

Location: `~/.config/lmtt/config.toml`

```toml
[general]
wallpaper = "~/Pictures/forrest.png"
default_mode = "dark"
scheme_type = "scheme-expressive"

[notifications]
enabled = true
timeout = 5000

[performance]
timeout = 10
slow_module_threshold = 250

# Modules are enabled by default
# Disable specific modules:
[modules.vscode]
enabled = false
```

## Troubleshooting

### Binary not in PATH
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Module not running
```bash
# Check if app is installed
lmtt list

# Check if module is enabled
cat ~/.config/lmtt/config.toml
```

### Setup not working
```bash
# Preview what setup would do
lmtt setup --dry-run

# Make sure config file exists
lmtt status
```

## Uninstall

### Remove Binary
```bash
make uninstall-user
# Or: sudo make uninstall
```

### Remove Config Injections
```bash
lmtt cleanup
```

### Remove Config File (optional)
```bash
rm -rf ~/.config/lmtt
```

## Development

### Build from source
```bash
cd ~/repos/linux-matugen-theme-toggle
make release
./target/release/lmtt --help
```

### Run tests
```bash
make test
```

### Format and lint
```bash
make fmt
make lint
```

### Watch for changes
```bash
make watch
```

## Next Steps

- Read `README.md` for detailed documentation
- See `TESTING.md` for comprehensive testing guide
- Check `PROJECT_STATUS.md` for implementation status
- Read `SUMMARY.md` for architecture overview

## Quick Links

- Binary: `~/.local/bin/lmtt` (after install-user)
- Config: `~/.config/lmtt/config.toml`
- Cache: `~/.cache/lmtt/`
- Source: `~/repos/linux-matugen-theme-toggle`
