# Installation Guide

## Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs)
- **matugen** - For color generation: `cargo install matugen`
- **Git** - For cloning the repository

## Quick Install (Recommended)

```bash
# Clone repository
cd ~/repos
git clone https://github.com/yourusername/linux-matugen-theme-toggle.git
cd linux-matugen-theme-toggle

# Install to ~/.local/bin (no root needed)
make install-user
```

## System-Wide Install

```bash
cd ~/repos/linux-matugen-theme-toggle

# Install to /usr/local/bin (default)
sudo make install

# Or install to /usr/bin
sudo make install PREFIX=/usr
```

## Verify Installation

```bash
which lmtt
lmtt --help
```

## Post-Installation

1. **Create config file**:
   ```bash
   lmtt init
   ```

2. **Edit config** to set your wallpaper:
   ```bash
   vim ~/.config/lmtt/config.toml
   ```

3. **Run setup** to configure apps:
   ```bash
   lmtt setup
   ```

4. **Test theme switching**:
   ```bash
   lmtt switch dark
   lmtt switch light
   lmtt switch  # Toggle
   ```

## Build from Source Only

If you just want to build without installing:

```bash
cd ~/repos/linux-matugen-theme-toggle
make release

# Binary will be at:
./target/release/lmtt
```

## Uninstall

### User Installation
```bash
cd ~/repos/linux-matugen-theme-toggle
make uninstall-user
```

### System Installation
```bash
cd ~/repos/linux-matugen-theme-toggle
sudo make uninstall
```

### Remove Config (Optional)
```bash
# Remove config injections from apps
lmtt cleanup

# Remove lmtt config directory
rm -rf ~/.config/lmtt

# Remove cache
rm -rf ~/.cache/lmtt
```

## Distribution Package (Future)

### Arch Linux (AUR)
```bash
yay -S lmtt
```

### Fedora (COPR)
```bash
sudo dnf copr enable username/lmtt
sudo dnf install lmtt
```

### Manual .tar.gz
```bash
# Download release tarball
wget https://github.com/yourusername/lmtt/releases/download/v0.1.0/lmtt-0.1.0-x86_64-linux.tar.gz

# Extract
tar xzf lmtt-0.1.0-x86_64-linux.tar.gz
cd lmtt-0.1.0

# Install
sudo install -Dm755 lmtt /usr/local/bin/lmtt
```

## Troubleshooting

### "command not found: lmtt"

Make sure `~/.local/bin` is in your PATH:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### "error: failed to load manifest"

Make sure you're in the repository directory:
```bash
cd ~/repos/linux-matugen-theme-toggle
```

### Rust not installed

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### matugen not found

```bash
cargo install matugen
# Or on Arch: yay -S matugen
```

## Next Steps

After installation, see:
- `QUICKSTART.md` for quick start guide
- `README.md` for full documentation
- `lmtt --help` for command reference
