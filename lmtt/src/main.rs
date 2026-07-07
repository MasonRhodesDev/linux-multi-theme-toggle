mod matugen;

use clap::{Parser, Subcommand};
use lmtt_core::{Config, ThemeMode};
use lmtt_modules::{ModuleRegistry, SetupManager, CleanupManager};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "lmtt")]
#[command(about = "Linux Multi-Theme Toggle - High-performance theme switching", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Switch to light or dark theme, or toggle between them
    Switch {
        /// Theme mode (light or dark). If omitted, toggles between current theme.
        mode: Option<ThemeMode>,
        
        /// Disable notifications
        #[arg(long)]
        no_notify: bool,
    },
    
    /// Setup mode - configure application config files
    Setup {
        /// Dry run - show what would be changed without prompting
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Cleanup - remove lmtt config injections
    Cleanup {
        /// Dry run - show what would be removed without prompting
        #[arg(long)]
        dry_run: bool,
        
        /// Cleanup specific module only
        #[arg(short, long)]
        module: Option<String>,
    },
    
    /// Show current theme status
    Status,
    
    /// List installed modules
    List {
        /// Show all modules (including not installed)
        #[arg(long)]
        all: bool,
    },
    
    /// Initialize config file
    Init,
    
    /// Interactive configuration manager
    Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging: --verbose wins, then RUST_LOG, then config [logging].level
    let logging = Config::load().map(|c| c.logging).unwrap_or_default();
    let log_level = if cli.verbose { "debug".to_string() } else { logging.level.clone() };
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    use tracing_subscriber::prelude::*;
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false));

    match open_log_file(&logging) {
        Some(file) => registry
            .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(file))
            .init(),
        None => registry.init(),
    }
    
    match cli.command {
        Commands::Switch { mode, no_notify } => {
            cmd_switch(mode, no_notify).await?;
        }
        
        Commands::Setup { dry_run } => {
            cmd_setup(dry_run).await?;
        }
        
        Commands::Cleanup { dry_run, module } => {
            cmd_cleanup(dry_run, module).await?;
        }
        
        Commands::Status => {
            cmd_status().await?;
        }
        
        Commands::List { all } => {
            cmd_list(all).await?;
        }
        
        Commands::Init => {
            cmd_init().await?;
        }
        
        Commands::Config => {
            return lmtt_config_tui::run_config_tui();
        }
    }
    
    Ok(())
}

/// Open the configured log file for appending, rotating it to `<file>.old`
/// once it exceeds max_log_size MB. Returns None (console-only logging) if
/// the file can't be opened.
fn open_log_file(logging: &lmtt_core::config::LoggingConfig) -> Option<std::fs::File> {
    let path = std::path::PathBuf::from(&logging.log_file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > logging.max_log_size.saturating_mul(1024 * 1024) {
            let _ = std::fs::rename(&path, path.with_extension("log.old"));
        }
    }
    std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()
}

/// Waybar-specific rule shipped in the shared palette file. It lives here
/// (not in the waybar module) so the palette file has exactly one writer;
/// other importers parse it as an unknown selector and ignore it.
const WAYBAR_TRAY_CSS: &str = "\n/* Tray icon theming: prefer symbolic icons recolored by foreground */\n#tray {\n    -gtk-icon-style: symbolic;\n    color: @foreground;\n}\n";

async fn cmd_switch(mode: Option<ThemeMode>, no_notify: bool) -> Result<()> {
    let config = Config::load()?;
    let cache = lmtt_core::cache::Cache::from_config(&config)?;

    // Serialize concurrent switches (e.g. a double-tapped toggle keybind):
    // without this both processes read the same state and toggle to the same
    // mode while interleaving writes to shared files. Blocking is correct —
    // the second invocation then sees the first one's saved state.
    let lock_path = std::path::PathBuf::from(&config.cache.dir).join("lmtt.lock");
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)?;
    lock_file.lock()?;

    let registry = ModuleRegistry::new();

    // Determine target mode (toggle if not specified)
    let mode = if let Some(m) = mode {
        m
    } else {
        let current = cache.get_theme_state(config.general.default_mode).await?;
        let toggled = match current {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        println!("Toggling from {} to {} mode...", current, toggled);
        toggled
    };

    println!("Switching to {} mode...", mode);

    // Generate color scheme
    let color_cache = if config.cache.enabled { Some(&cache) } else { None };
    let scheme = matugen::generate_colors(&config, mode, color_cache).await?;

    // Write shared lmtt-colors.css BEFORE modules run.
    // GTK3 apps (Thunar) re-read gtk.css when gsettings changes, which
    // @imports this file. It must have the new colors before the GTK
    // module updates gsettings, otherwise apps render with stale colors.
    // This is the ONLY writer of this file; modules just reload their app.
    let css_path = dirs::config_dir()
        .ok_or(anyhow::anyhow!("No config dir"))?
        .join("matugen")
        .join("lmtt-colors.css");
    if let Some(parent) = css_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut css = scheme.to_gtk_css();
    css.push_str(WAYBAR_TRAY_CSS);
    lmtt_core::fsutil::write_atomic(&css_path, css).await?;

    // Apply to all modules
    let results = registry.apply_all(&scheme, &config).await;

    // Print results
    let mut successes = 0;
    let mut failures = 0;

    let show_progress =
        config.notifications.enabled && !no_notify && config.notifications.show_module_progress;

    for result in results {
        if result.is_success() {
            successes += 1;
            let icon = if result.is_slow(config.performance.slow_module_threshold) {
                "⚠"
            } else {
                "✓"
            };
            println!("{} [{}] {}ms", icon, result.name, result.duration_ms);
        } else {
            failures += 1;
            if let Err(e) = &result.result {
                eprintln!("✗ [{}] {}", result.name, e);
            }
        }

        if show_progress {
            let status = if result.is_success() { "updated" } else { "FAILED" };
            let _ = tokio::process::Command::new("notify-send")
                .args([
                    "--app-name=lmtt",
                    "--expire-time=2000",
                    &format!("lmtt: {} {}", result.name, status),
                ])
                .status()
                .await;
        }
    }

    println!("\n{} successful, {} failed", successes, failures);

    if successes == 0 && failures > 0 {
        // Nothing switched: don't record a state we never reached, and let
        // scripts see the failure in the exit code.
        anyhow::bail!("theme switch failed: all {} modules failed", failures);
    }

    cache.set_theme_state(mode).await?;

    if failures == 0 {
        println!("Theme switched to {} mode!", mode);
    }

    if config.notifications.enabled && !no_notify {
        notify_switch(&config, mode, successes, failures).await;
    }

    Ok(())
}

/// Best-effort desktop notification; failures are logged, never fatal.
async fn notify_switch(config: &Config, mode: ThemeMode, successes: usize, failures: usize) {
    let summary = format!("Theme switched to {} mode", mode);
    let body = if failures > 0 {
        format!("{} modules updated, {} failed", successes, failures)
    } else {
        format!("{} modules updated", successes)
    };
    let result = tokio::process::Command::new("notify-send")
        .args([
            "--app-name=lmtt",
            &format!("--expire-time={}", config.notifications.timeout.max(0)),
            &summary,
            &body,
        ])
        .status()
        .await;
    if let Err(e) = result {
        tracing::debug!("notify-send unavailable: {}", e);
    }
}

async fn cmd_setup(dry_run: bool) -> Result<()> {
    let config = Config::load()?;
    let registry = ModuleRegistry::new();
    let setup = SetupManager::new(registry);
    
    if dry_run {
        setup.dry_run().await?;
    } else {
        setup.run(&config).await?;
    }
    
    Ok(())
}

async fn cmd_cleanup(dry_run: bool, module: Option<String>) -> Result<()> {
    let registry = ModuleRegistry::new();
    let cleanup = CleanupManager::new(registry);
    
    if dry_run {
        cleanup.dry_run().await?;
    } else if let Some(module_name) = module {
        cleanup.run_module(&module_name).await?;
    } else {
        cleanup.run_all().await?;
    }
    
    Ok(())
}

async fn cmd_status() -> Result<()> {
    let config = Config::load()?;
    let cache = lmtt_core::cache::Cache::from_config(&config)?;
    
    let current_mode = cache.get_theme_state(config.general.default_mode).await?;

    println!("Current theme: {}", current_mode);
    println!("Wallpaper: {}", config.general.wallpaper);
    println!("Scheme type: {}", config.general.scheme_type);
    
    Ok(())
}

async fn cmd_list(all: bool) -> Result<()> {
    let config = Config::load()?;
    let registry = ModuleRegistry::new();
    
    println!("Module Status:");
    println!("==============\n");
    
    for module in &registry.modules {
        let installed = module.is_installed();
        let enabled = module.is_enabled(&config);
        
        if !all && !enabled {
            continue;
        }
        
        let status = if enabled && installed {
            "✓ enabled"
        } else if installed {
            "○ disabled"
        } else {
            "✗ not installed"
        };
        
        println!("{:12} {}", module.name(), status);
    }
    
    Ok(())
}

async fn cmd_init() -> Result<()> {
    let config_path = Config::config_path()?;
    
    if config_path.exists() {
        println!("Config already exists at: {}", config_path.display());
        print!("Overwrite? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    let config = Config::default();
    config.save()?;
    
    println!("✓ Created config at: {}", config_path.display());
    println!("\nNext steps:");
    println!("1. Edit the config file to set your wallpaper path");
    println!("2. Run 'lmtt setup' to configure application config files");
    println!("3. Run 'lmtt switch dark' or 'lmtt switch light' to apply theme");
    
    Ok(())
}
