#!/bin/bash
# Spotify theming with Spicetify
# https://spicetify.app/

MODE=$1
COLORS_JSON=$2

if ! command -v spicetify &> /dev/null; then
    echo "Spicetify not installed"
    exit 1
fi

# Parse colors
PRIMARY=$(jq -r '.primary' "$COLORS_JSON")
SURFACE=$(jq -r '.surface' "$COLORS_JSON")
ON_SURFACE=$(jq -r '.on_surface' "$COLORS_JSON")
ERROR=$(jq -r '.error' "$COLORS_JSON")

# Set Spicetify colors
spicetify config color_scheme "lmtt-$MODE"
spicetify config current_theme "lmtt"

# Create color scheme
mkdir -p ~/.config/spicetify/Themes/lmtt
cat > ~/.config/spicetify/Themes/lmtt/color.ini << EOF
[lmtt-$MODE]
text               = $ON_SURFACE
subtext            = $ON_SURFACE
sidebar-text       = $ON_SURFACE
main               = $PRIMARY
sidebar            = $SURFACE
player             = $SURFACE
card               = $SURFACE
shadow             = #00000040
selected-row       = ${PRIMARY}40
button             = $PRIMARY
button-active      = $PRIMARY
button-disabled    = ${ON_SURFACE}40
tab-active         = $PRIMARY
notification       = $PRIMARY
notification-error = $ERROR
EOF

# Apply changes
spicetify apply
