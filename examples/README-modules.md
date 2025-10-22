# LMTT Custom Modules

This directory contains custom module definitions that extend LMTT's theme support to additional applications.

## Quick Start

1. Copy a module file from `examples/modules/` to `~/.config/lmtt/modules/`:
   ```bash
   cp examples/modules/alacritty.toml ~/.config/lmtt/modules/
   ```

2. LMTT will automatically discover and load it on next run:
   ```bash
   lmtt list --all  # Shows your custom module!
   lmtt switch      # Applies theme to custom modules too
   ```

## Creating Your Own Module

### Method 1: Template-Based (Declarative)

Best for: Simple config files with color substitution

```toml
name = "myapp"
binary = "myapp"           # Binary name to check if installed
priority = 100             # Lower = runs earlier

[output]
path = "~/.config/myapp/colors.conf"
format = "conf"            # yaml, json, ini, css, toml, conf

[template]
content = """
background={{surface}}
foreground={{on_surface}}
primary={{primary}}
"""

[reload]                   # Optional
command = "pkill -USR1 myapp"
timeout = 5000

[setup]                    # Optional
config_file = "~/.config/myapp/config.conf"
include_line = "source ~/.config/myapp/colors.conf"
description = "Include LMTT colors in myapp config"
```

### Method 2: Script-Based

Best for: Complex logic, multiple files, API calls

```toml
name = "myapp"
binary = "myapp"
priority = 100

[script]
path = "~/.config/lmtt/scripts/myapp.sh"
timeout = 10000
pass_as_env = false        # If true, passes colors as env vars
```

Create the script at `~/.config/lmtt/scripts/myapp.sh`:

```bash
#!/bin/bash
MODE=$1           # "light" or "dark"
COLORS_JSON=$2    # Path to JSON file with all colors

# Parse colors
PRIMARY=$(jq -r '.primary' "$COLORS_JSON")
SURFACE=$(jq -r '.surface' "$COLORS_JSON")

# Your custom logic here
echo "Setting $MODE theme with primary=$PRIMARY"
```

Don't forget to make it executable:
```bash
chmod +x ~/.config/lmtt/scripts/myapp.sh
```

## Available Template Variables

All Material You colors are available in templates:

### Core Colors
- `{{surface}}`, `{{on_surface}}`
- `{{primary}}`, `{{on_primary}}`
- `{{secondary}}`, `{{on_secondary}}`
- `{{tertiary}}`, `{{on_tertiary}}`
- `{{error}}`, `{{on_error}}`

### Surface Variants
- `{{surface_container}}`
- `{{surface_container_high}}`
- `{{surface_container_highest}}`
- `{{surface_variant}}`, `{{on_surface_variant}}`

### Container Colors
- `{{primary_container}}`, `{{on_primary_container}}`
- `{{secondary_container}}`, `{{on_secondary_container}}`
- `{{tertiary_container}}`, `{{on_tertiary_container}}`
- `{{error_container}}`, `{{on_error_container}}`

### Other
- `{{outline}}`, `{{outline_variant}}`
- `{{background}}`, `{{on_background}}`
- `{{mode}}` - "light" or "dark"

## Module Priority

Priority determines execution order (lower runs first):
- **1-50**: Platform modules (GTK, XDG, Qt)
- **100** (default): Application modules
- **200+**: Modules that depend on others

## Examples Included

- `alacritty.toml` - Alacritty terminal (declarative)
- `kitty.toml` - Kitty terminal (declarative)
- `discord-betterdiscord.toml` - Discord with BetterDiscord (declarative)
- `spotify.toml` - Spotify with Spicetify (script-based)

## Tips

- **Test with dry-run**: Check if your module loads correctly
  ```bash
  lmtt list --all
  ```

- **Debug output**: Use verbose mode to see what's happening
  ```bash
  lmtt switch --verbose
  ```

- **Module not loading?**
  - Check file is `.toml` extension
  - Verify TOML syntax is valid
  - Check logs for errors

## Troubleshooting

### Module not appearing in `lmtt list`
- Ensure file ends with `.toml`
- Check TOML syntax: `toml-test your-module.toml`
- Check LMTT logs for parsing errors

### Template errors
- Verify all `{{variables}}` match available colors
- Use `{{mode}}` to get "light" or "dark"
- Handlebars syntax required (not `$variable`)

### Script not executing
- Ensure script is executable: `chmod +x script.sh`
- Test script manually: `./script.sh dark /tmp/colors.json`
- Check timeout value (default 10s)


