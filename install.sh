#!/bin/bash
set -e

echo "=== LMTT Installation Script ==="
echo ""

# Build release
echo "Building release version..."
cargo build --release

# Install to ~/.local/bin
echo "Installing to ~/.local/bin..."
mkdir -p ~/.local/bin
cp target/release/lmtt ~/.local/bin/lmtt
chmod +x ~/.local/bin/lmtt

echo ""
echo "✓ LMTT installed successfully!"
echo ""
echo "Commands:"
echo "  lmtt config   - Open configuration TUI"
echo "  lmtt status   - Show current theme"
echo "  lmtt switch   - Toggle theme"
echo "  lmtt --help   - Show all commands"
