mod matugen;

use anyhow::Result;
use appearance_profiles::{Background, Fit, OutputIdentity, Profile, Registry};
use clap::{Parser, Subcommand};
use lmtt_core::{Config, ThemeMode};
use lmtt_modules::{CleanupManager, ModuleRegistry, SetupManager};
use std::path::{Path, PathBuf};

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

    /// Manage the shared appearance-profile wallpaper registry
    Wallpaper {
        #[command(subcommand)]
        command: WallpaperCommand,
    },

    /// Print the current palette (JSON) or a single token
    Tokens {
        /// Token name; prints `#rrggbb`
        #[arg(long)]
        key: Option<String>,
        /// Read a greeter-readable published snapshot
        #[arg(long)]
        user: Option<String>,
    },
}

#[derive(Subcommand)]
enum WallpaperCommand {
    /// Set the current user's global or per-output wallpaper
    Set {
        path: PathBuf,
        #[arg(long)]
        output: Option<String>,
        #[arg(long, value_enum)]
        fit: Option<WallpaperFit>,
        /// Do not refresh the greeter-readable snapshot
        #[arg(long)]
        no_publish: bool,
    },
    /// Resolve the effective wallpaper for an output
    Resolve {
        #[arg(long, default_value = "default")]
        output: String,
        #[arg(long)]
        description: Option<String>,
        /// Resolve a greeter-readable snapshot for this user
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Refresh the greeter-readable snapshot for the current user
    Publish,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum WallpaperFit {
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
}

impl From<WallpaperFit> for Fit {
    fn from(value: WallpaperFit) -> Self {
        match value {
            WallpaperFit::Fill => Fit::Fill,
            WallpaperFit::Fit => Fit::Fit,
            WallpaperFit::Stretch => Fit::Stretch,
            WallpaperFit::Center => Fit::Center,
            WallpaperFit::Tile => Fit::Tile,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging: --verbose wins, then RUST_LOG, then config [logging].level
    let logging = Config::load().map(|c| c.logging).unwrap_or_default();
    let log_level = if cli.verbose {
        "debug".to_string()
    } else {
        logging.level.clone()
    };
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    use tracing_subscriber::prelude::*;
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_target(false));

    match open_log_file(&logging) {
        Some(file) => registry
            .with(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(file),
            )
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
        Commands::Wallpaper { command } => cmd_wallpaper(command)?,
        Commands::Tokens { key, user } => cmd_tokens(key, user)?,
    }

    Ok(())
}

fn cmd_wallpaper(command: WallpaperCommand) -> Result<()> {
    match command {
        WallpaperCommand::Set {
            path,
            output,
            fit,
            no_publish,
        } => {
            let path = std::fs::canonicalize(&path).map_err(|error| {
                anyhow::anyhow!("cannot use wallpaper {}: {error}", path.display())
            })?;
            let profile_path = appearance_profiles::user_profile_path()
                .ok_or_else(|| anyhow::anyhow!("cannot determine the user config directory"))?;
            let mut profile = Profile::load(&profile_path)?.unwrap_or_default();
            let rule = if let Some(output) = output {
                profile.output.entry(output).or_default()
            } else {
                &mut profile.background
            };
            rule.path = Some(path);
            if let Some(fit) = fit {
                rule.fit = Some(fit.into());
            }
            write_profile(&profile_path, &profile)?;
            println!("Updated {}", profile_path.display());
            if !no_publish {
                publish_current(&profile)?;
            }
        }
        WallpaperCommand::Resolve {
            output,
            description,
            user,
            json,
        } => {
            let registry = match user {
                Some(user) => Registry::load_published(&user)?,
                None => Registry::load_current_user()?,
            };
            let resolved = registry.resolve(&OutputIdentity::new(output, description), None);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "path": resolved.path,
                        "fit": format!("{:?}", resolved.fit).to_lowercase(),
                        "path_source": format!("{:?}", resolved.path_source),
                        "fit_source": format!("{:?}", resolved.fit_source),
                    }))?
                );
            } else if let Some(path) = resolved.path {
                println!("{}", path.display());
            } else {
                anyhow::bail!("no wallpaper is configured for this output");
            }
        }
        WallpaperCommand::Publish => {
            let path = appearance_profiles::user_profile_path()
                .ok_or_else(|| anyhow::anyhow!("cannot determine the user config directory"))?;
            let profile = Profile::load(&path)?.ok_or_else(|| {
                anyhow::anyhow!("no user appearance profile at {}", path.display())
            })?;
            publish_current(&profile)?;
        }
    }
    Ok(())
}

fn write_profile(path: &Path, profile: &Profile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, toml::to_string_pretty(profile)?)?;
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn publish_current(profile: &Profile) -> Result<()> {
    let user = std::env::var("USER").map_err(|_| anyhow::anyhow!("USER is not set"))?;
    let destination = appearance_profiles::published_profile_path(&user)?;
    let root = destination
        .parent()
        .expect("published profile has a parent");
    std::fs::create_dir_all(root).map_err(|error| {
        anyhow::anyhow!(
            "cannot create {}: {error}; provision it for this user first",
            root.display()
        )
    })?;
    let assets = root.join("assets");
    std::fs::create_dir_all(&assets)?;
    let mut snapshot = profile.clone();
    publish_rule(&mut snapshot.background, &assets, "background")?;
    for (selector, rule) in &mut snapshot.output {
        let safe: String = selector
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        publish_rule(rule, &assets, &format!("output-{safe}"))?;
    }
    write_profile(&destination, &snapshot)?;
    publish_tokens(root)?;
    println!("Published {}", destination.display());
    Ok(())
}

fn publish_tokens(root: &Path) -> Result<()> {
    let source = lmtt_core::tokens::user_tokens_path()?;
    if !source.is_file() {
        return Ok(());
    }
    let destination = root.join("tokens.json");
    std::fs::copy(&source, &destination).map_err(|error| {
        anyhow::anyhow!("cannot publish tokens {}: {error}", source.display())
    })?;
    Ok(())
}

fn cmd_tokens(key: Option<String>, user: Option<String>) -> Result<()> {
    let scheme = match user {
        Some(user) => lmtt_core::tokens::load_published(&user)?,
        None => match lmtt_core::tokens::load_current() {
            Ok(scheme) => scheme,
            Err(_) => lmtt_core::tokens::load_preferring(ThemeMode::Dark),
        },
    };
    if let Some(key) = key {
        let value = scheme
            .get(&key)
            .cloned()
            .unwrap_or_else(|| scheme.get_or_fallback(&key));
        println!("{value}");
        return Ok(());
    }
    println!("{}", serde_json::to_string_pretty(&scheme)?);
    Ok(())
}

fn publish_rule(rule: &mut Background, assets: &Path, stem: &str) -> Result<()> {
    let Some(source) = rule.path.as_ref() else {
        return Ok(());
    };
    let extension = source.extension().and_then(|v| v.to_str()).unwrap_or("img");
    let destination = assets.join(format!("{stem}.{extension}"));
    std::fs::copy(source, &destination).map_err(|error| {
        anyhow::anyhow!("cannot publish wallpaper {}: {error}", source.display())
    })?;
    rule.path = Some(destination);
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
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()
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
    let color_cache = if config.cache.enabled {
        Some(&cache)
    } else {
        None
    };
    let scheme = matugen::generate_colors(&config, mode, color_cache).await?;
    lmtt_core::tokens::write_current(&scheme)?;

    // Write shared lmtt-colors.css BEFORE modules run.
    // GTK3 apps (Thunar) re-read gtk.css when gsettings changes, which
    // @imports this file. It must have the new colors before the GTK
    // module updates gsettings, otherwise apps render with stale colors.
    // This is the ONLY writer of this file; modules just reload their app.
    let css_path = lmtt_core::paths::user_dirs()?
        .config_home()
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
            let status = if result.is_success() {
                "updated"
            } else {
                "FAILED"
            };
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
