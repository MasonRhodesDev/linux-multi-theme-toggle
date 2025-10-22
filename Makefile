# Makefile for linux-matugen-theme-toggle (lmtt)

# Project metadata
PROJECT = lmtt
VERSION = 0.1.0
CARGO = cargo
INSTALL = install
PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share

# Build configuration
RELEASE_FLAGS = --release
TARGET_DIR = target/release
DEBUG_TARGET_DIR = target/debug

# Colors for output
BOLD := $(shell tput bold)
GREEN := $(shell tput setaf 2)
YELLOW := $(shell tput setaf 3)
RED := $(shell tput setaf 1)
RESET := $(shell tput sgr0)

.PHONY: all build release debug test bench clean install uninstall check fmt lint help run watch docs dist

# Default target
all: release

# Check Rust toolchain
check-toolchain:
	@echo "$(BOLD)Checking Rust toolchain...$(RESET)"
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)Error: Rust not found. Install from https://rustup.rs$(RESET)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)Error: Cargo not found. Install from https://rustup.rs$(RESET)"; exit 1; }
	@echo "$(GREEN)✓ Rust toolchain: $$(rustc --version)$(RESET)"
	@echo "$(GREEN)✓ Cargo: $$(cargo --version)$(RESET)"

# Build debug binary
debug: check-toolchain
	@echo "$(BOLD)Building debug binary...$(RESET)"
	$(CARGO) build
	@echo "$(GREEN)✓ Debug binary: $(DEBUG_TARGET_DIR)/$(PROJECT)$(RESET)"
	@ls -lh $(DEBUG_TARGET_DIR)/$(PROJECT)

# Build release binary (optimized)
release: check-toolchain
	@echo "$(BOLD)Building release binary...$(RESET)"
	$(CARGO) build $(RELEASE_FLAGS)
	@echo "$(GREEN)✓ Release binary: $(TARGET_DIR)/$(PROJECT)$(RESET)"
	@ls -lh $(TARGET_DIR)/$(PROJECT)

# Alias for release
build: release

# Run tests
test: check-toolchain
	@echo "$(BOLD)Running tests...$(RESET)"
	$(CARGO) test --all
	@echo "$(GREEN)✓ All tests passed$(RESET)"

# Run benchmarks
bench: check-toolchain
	@echo "$(BOLD)Running benchmarks...$(RESET)"
	$(CARGO) bench

# Check code (fast compile check without codegen)
check: check-toolchain
	@echo "$(BOLD)Running cargo check...$(RESET)"
	$(CARGO) check --all-targets --all-features
	@echo "$(GREEN)✓ Check passed$(RESET)"

# Format code
fmt: check-toolchain
	@echo "$(BOLD)Formatting code...$(RESET)"
	$(CARGO) fmt --all
	@echo "$(GREEN)✓ Code formatted$(RESET)"

# Lint with clippy
lint: check-toolchain
	@echo "$(BOLD)Running clippy...$(RESET)"
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo "$(GREEN)✓ Clippy passed$(RESET)"

# Clean build artifacts
clean:
	@echo "$(BOLD)Cleaning build artifacts...$(RESET)"
	$(CARGO) clean
	rm -rf dist/
	@echo "$(GREEN)✓ Cleaned$(RESET)"

# Install binary and config example
install: release
	@echo "$(BOLD)Installing $(PROJECT)...$(RESET)"
	$(INSTALL) -Dm755 $(TARGET_DIR)/$(PROJECT) $(DESTDIR)$(BINDIR)/$(PROJECT)
	$(INSTALL) -Dm644 config-example.toml $(DESTDIR)$(DATADIR)/$(PROJECT)/config-example.toml
	$(INSTALL) -Dm644 README.md $(DESTDIR)$(DATADIR)/doc/$(PROJECT)/README.md
	@echo "$(GREEN)✓ Installed to $(DESTDIR)$(BINDIR)/$(PROJECT)$(RESET)"
	@echo ""
	@echo "$(BOLD)Next steps:$(RESET)"
	@echo "  1. Run: $(PROJECT) init"
	@echo "  2. Edit: ~/.config/lmtt/config.toml"
	@echo "  3. Run: $(PROJECT) setup"
	@echo "  4. Run: $(PROJECT) switch"

# Uninstall
uninstall:
	@echo "$(BOLD)Uninstalling $(PROJECT)...$(RESET)"
	rm -f $(DESTDIR)$(BINDIR)/$(PROJECT)
	rm -rf $(DESTDIR)$(DATADIR)/$(PROJECT)
	rm -rf $(DESTDIR)$(DATADIR)/doc/$(PROJECT)
	@echo "$(GREEN)✓ Uninstalled$(RESET)"
	@echo ""
	@echo "$(YELLOW)Note: User config at ~/.config/lmtt/ was not removed$(RESET)"
	@echo "      Run 'rm -rf ~/.config/lmtt' to remove it manually"

# Create distribution tarball
dist: release
	@echo "$(BOLD)Creating distribution tarball...$(RESET)"
	@mkdir -p dist
	@tar czf dist/$(PROJECT)-$(VERSION)-x86_64-linux.tar.gz \
		-C $(TARGET_DIR) $(PROJECT) \
		-C ../../ README.md LICENSE config-example.toml \
		--transform 's,^,$(PROJECT)-$(VERSION)/,'
	@echo "$(GREEN)✓ Created dist/$(PROJECT)-$(VERSION)-x86_64-linux.tar.gz$(RESET)"
	@ls -lh dist/$(PROJECT)-$(VERSION)-x86_64-linux.tar.gz

# Run development version
run: debug
	@echo "$(BOLD)Running $(PROJECT)...$(RESET)"
	@$(DEBUG_TARGET_DIR)/$(PROJECT) $(ARGS)

# Watch for changes and rebuild
watch:
	@echo "$(BOLD)Watching for changes...$(RESET)"
	$(CARGO) watch -x 'build' -x 'test --lib'

# Generate documentation
docs:
	@echo "$(BOLD)Generating documentation...$(RESET)"
	$(CARGO) doc --no-deps --open

# Performance profiling with perf
profile: release
	@echo "$(BOLD)Running performance profile...$(RESET)"
	@command -v perf >/dev/null 2>&1 || { echo "$(YELLOW)Warning: perf not found. Install with: sudo dnf install perf$(RESET)"; exit 1; }
	perf record -g $(TARGET_DIR)/$(PROJECT) switch dark
	perf report

# Memory check with valgrind
memcheck: debug
	@echo "$(BOLD)Running memory check...$(RESET)"
	@command -v valgrind >/dev/null 2>&1 || { echo "$(YELLOW)Warning: valgrind not found. Install with: sudo dnf install valgrind$(RESET)"; exit 1; }
	valgrind --leak-check=full \
		--show-leak-kinds=all \
		$(DEBUG_TARGET_DIR)/$(PROJECT) switch dark

# CI/CD target (runs all checks)
ci: check test lint
	@echo "$(GREEN)✓ All CI checks passed$(RESET)"

# Quick install to ~/.local/bin (no root needed)
install-user: release
	@echo "$(BOLD)Installing to ~/.local/bin...$(RESET)"
	@mkdir -p ~/.local/bin
	$(INSTALL) -m755 $(TARGET_DIR)/$(PROJECT) ~/.local/bin/$(PROJECT)
	@echo "$(GREEN)✓ Installed to ~/.local/bin/$(PROJECT)$(RESET)"
	@echo ""
	@echo "$(YELLOW)Make sure ~/.local/bin is in your PATH$(RESET)"
	@echo "Add this to your shell rc file if needed:"
	@echo '  export PATH="$$HOME/.local/bin:$$PATH"'

# Uninstall from ~/.local/bin
uninstall-user:
	@echo "$(BOLD)Uninstalling from ~/.local/bin...$(RESET)"
	rm -f ~/.local/bin/$(PROJECT)
	@echo "$(GREEN)✓ Uninstalled$(RESET)"

# Help target
help:
	@echo "$(BOLD)Linux Matugen Theme Toggle (lmtt) - Build System$(RESET)"
	@echo ""
	@echo "$(BOLD)Usage:$(RESET) make [target]"
	@echo ""
	@echo "$(BOLD)Build Targets:$(RESET)"
	@echo "  $(GREEN)all$(RESET)           Build optimized release binary (default)"
	@echo "  $(GREEN)release$(RESET)       Build optimized release binary"
	@echo "  $(GREEN)debug$(RESET)         Build debug binary"
	@echo "  $(GREEN)build$(RESET)         Alias for release"
	@echo ""
	@echo "$(BOLD)Development:$(RESET)"
	@echo "  $(GREEN)check$(RESET)         Fast compilation check"
	@echo "  $(GREEN)test$(RESET)          Run all tests"
	@echo "  $(GREEN)bench$(RESET)         Run benchmarks"
	@echo "  $(GREEN)fmt$(RESET)           Format code"
	@echo "  $(GREEN)lint$(RESET)          Run clippy linter"
	@echo "  $(GREEN)run$(RESET)           Build and run debug version (use ARGS='...')"
	@echo "  $(GREEN)watch$(RESET)         Watch for changes and rebuild"
	@echo "  $(GREEN)docs$(RESET)          Generate and open documentation"
	@echo ""
	@echo "$(BOLD)Installation:$(RESET)"
	@echo "  $(GREEN)install$(RESET)       Install to $(PREFIX)/bin (requires root)"
	@echo "  $(GREEN)uninstall$(RESET)     Remove from $(PREFIX)/bin (requires root)"
	@echo "  $(GREEN)install-user$(RESET)  Install to ~/.local/bin (no root)"
	@echo "  $(GREEN)uninstall-user$(RESET) Remove from ~/.local/bin"
	@echo ""
	@echo "$(BOLD)Distribution:$(RESET)"
	@echo "  $(GREEN)dist$(RESET)          Create distribution tarball"
	@echo "  $(GREEN)clean$(RESET)         Remove build artifacts"
	@echo ""
	@echo "$(BOLD)Quality Assurance:$(RESET)"
	@echo "  $(GREEN)ci$(RESET)            Run all CI checks (check, test, lint)"
	@echo "  $(GREEN)profile$(RESET)       Run performance profiling"
	@echo "  $(GREEN)memcheck$(RESET)      Run valgrind memory check"
	@echo ""
	@echo "$(BOLD)Variables:$(RESET)"
	@echo "  $(YELLOW)PREFIX$(RESET)        Installation prefix (default: /usr/local)"
	@echo "  $(YELLOW)DESTDIR$(RESET)       Staging directory for package builds"
	@echo "  $(YELLOW)ARGS$(RESET)          Arguments to pass to 'make run'"
	@echo ""
	@echo "$(BOLD)Examples:$(RESET)"
	@echo "  make                                    # Build release binary"
	@echo "  make install-user                       # Install to ~/.local/bin"
	@echo "  make run ARGS='switch dark'             # Run debug with args"
	@echo "  sudo make install PREFIX=/usr           # System install to /usr/bin"
	@echo "  make clean && make release              # Clean rebuild"
