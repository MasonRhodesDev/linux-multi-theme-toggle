# Testing Guide for LMTT

## Pre-Testing Checklist

1. Ensure Rust is installed: `rustc --version`
2. Ensure matugen is installed: `matugen --version`
3. Have a wallpaper ready at known path

## Compilation Test

```bash
cd ~/linux-matugen-theme-toggle

# Quick check (no codegen)
cargo check

# Full build (debug)
cargo build

# Full build (release)
cargo build --release
```

Expected: No errors, binary at `target/release/lmtt`

## Basic Functionality Tests

### 1. Test Init Command

```bash
./target/release/lmtt init
```

Expected:
- Creates `~/.config/lmtt/config.toml`
- Shows next steps message

Verify:
```bash
cat ~/.config/lmtt/config.toml
```

### 2. Test List Command

```bash
./target/release/lmtt list
```

Expected:
- Shows installed modules with ✓
- Shows not installed with ✗

### 3. Test Status Command

```bash
./target/release/lmtt status
```

Expected:
- Shows current theme (defaults to "dark" if never run)
- Shows wallpaper path
- Shows scheme type

### 4. Test Setup (Dry Run)

```bash
./target/release/lmtt setup --dry-run
```

Expected:
- Lists all detected apps
- Shows which config files would be modified
- Doesn't actually modify anything

### 5. Test Cleanup (Dry Run)

```bash
./target/release/lmtt cleanup --dry-run
```

Expected:
- Shows what would be removed
- Doesn't actually remove anything

## Theme Switching Tests

### Test 1: Explicit Mode Switch

```bash
# Edit config to set your wallpaper
vim ~/.config/lmtt/config.toml

# Switch to dark mode
./target/release/lmtt switch dark
```

Expected:
- Shows "Switching to dark mode..."
- Lists each module with timing
- Shows success/failure count
- GTK theme changes to Adwaita-dark
- Waybar colors update (if waybar installed)

### Test 2: Toggle Mode (NEW!)

```bash
# Toggle from dark to light
./target/release/lmtt switch
```

Expected:
- Shows "Toggling from dark to light mode..."
- Switches to opposite of current theme
- All modules update

```bash
# Toggle back to dark
./target/release/lmtt switch
```

Expected:
- Shows "Toggling from light to dark mode..."
- Switches back to dark

### Test 3: Verify State Persistence

```bash
# Switch to light
./target/release/lmtt switch light

# Check status
./target/release/lmtt status
```

Expected:
- Status shows "Current theme: light"

```bash
# Toggle (should go to dark)
./target/release/lmtt switch

# Check status again
./target/release/lmtt status
```

Expected:
- Status shows "Current theme: dark"

## Setup/Cleanup Tests

### Test Setup with Waybar (if installed)

```bash
# Backup your waybar config
cp ~/.config/waybar/style.css ~/.config/waybar/style.css.backup

# Run setup (answer 'y' to waybar prompt)
./target/release/lmtt setup
```

Expected:
- Prompts "Inject this line? [Y/n/q]"
- After answering 'y': "✓ Injected successfully!"

Verify injection:
```bash
head -n 5 ~/.config/waybar/style.css
```

Expected to see:
```css
# >>> lmtt managed block - do not edit manually >>>
@import url('../matugen/lmtt-colors.css');
# <<< lmtt managed block <<<

/* Your original content */
```

### Test Cleanup

```bash
# Remove waybar config
./target/release/lmtt cleanup --module waybar
```

Expected:
- Shows "Removing from ~/.config/waybar/style.css..."
- Shows "✓ Removed: @import url(...)"

Verify removal:
```bash
head -n 5 ~/.config/waybar/style.css
```

Expected:
- Marker block is gone
- Original content restored

```bash
# Restore backup
cp ~/.config/waybar/style.css.backup ~/.config/waybar/style.css
```

## Performance Tests

### Test Parallel Execution

```bash
# Run with verbose output
./target/release/lmtt -v switch dark 2>&1 | tee /tmp/lmtt-test.log
```

Expected:
- Multiple modules complete in <250ms
- Total time <500ms (depends on installed modules)

### Test Slow Module Warning

```bash
# Set low threshold in config
vim ~/.config/lmtt/config.toml
# Change slow_module_threshold = 50

./target/release/lmtt switch light
```

Expected:
- Modules slower than 50ms show ⚠ icon

## Edge Cases

### Test with Missing Wallpaper

```bash
# Edit config with invalid wallpaper
vim ~/.config/lmtt/config.toml
# Set: wallpaper = "/nonexistent/file.png"

./target/release/lmtt switch dark
```

Expected:
- Error: "Wallpaper not found: /nonexistent/file.png"

### Test with Invalid Config

```bash
# Break config syntax
echo "invalid toml [[[" >> ~/.config/lmtt/config.toml

./target/release/lmtt switch dark
```

Expected:
- TOML parse error message

```bash
# Restore config
./target/release/lmtt init
```

### Test Cleanup on Non-Injected File

```bash
# Try to cleanup module that was never set up
./target/release/lmtt cleanup --module waybar
```

Expected:
- Shows "○ nothing to remove" or "Already gone"

## Integration Tests

### Test Full Workflow

```bash
# Clean slate
rm -rf ~/.config/lmtt
./target/release/lmtt init

# Edit wallpaper path
vim ~/.config/lmtt/config.toml

# Run setup
./target/release/lmtt setup --dry-run  # Preview
./target/release/lmtt setup            # Answer 'y' to all

# Test switching
./target/release/lmtt switch dark
./target/release/lmtt status           # Verify dark
./target/release/lmtt switch           # Toggle to light
./target/release/lmtt status           # Verify light
./target/release/lmtt switch           # Toggle to dark
./target/release/lmtt status           # Verify dark

# Verify modules
./target/release/lmtt list

# Clean up
./target/release/lmtt cleanup --dry-run  # Preview
./target/release/lmtt cleanup            # Confirm 'y'
```

## Troubleshooting

### Compilation Errors

If you get dependency errors:
```bash
cargo clean
cargo update
cargo build --release
```

### Runtime Errors

Enable debug logging:
```bash
RUST_LOG=debug ./target/release/lmtt switch dark
```

### Module Not Running

Check:
1. Is the app installed? `which waybar`
2. Is the module enabled in config?
3. Does the binary name match? Check `lib.rs`

## Success Criteria

- [ ] Compiles without errors
- [ ] `lmtt init` creates config
- [ ] `lmtt list` shows installed modules
- [ ] `lmtt setup` prompts for injection
- [ ] `lmtt switch dark` changes theme
- [ ] `lmtt switch` toggles theme
- [ ] `lmtt status` shows correct theme
- [ ] `lmtt cleanup` removes injections
- [ ] Dry-run modes preview without changing
- [ ] Performance <500ms total
- [ ] State persists between runs

## Next Steps After Testing

1. Port remaining bash modules
2. Add event system
3. Implement notifications
4. Create Makefile
5. Write unit tests
6. Benchmark performance
