mod matugen;

use clap::{Parser, Subcommand};
use lmtt_core::{Config, ThemeMode};
use lmtt_modules::{ModuleRegistry, SetupManager, CleanupManager};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "lmtt")]
#[command(about = "Linux Matugen Theme Toggle - High-performance theme switching", long_about = None)]
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
        /// Show all modules (including disabled)
        #[arg(short, long)]
        all: bool,
    },
    
    /// Initialize config file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .init();
    
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
    }
    
    Ok(())
}

async fn cmd_switch(mode: Option<ThemeMode>, _no_notify: bool) -> Result<()> {
    let config = Config::load()?;
    let registry = ModuleRegistry::new();
    
    // Determine target mode (toggle if not specified)
    let mode = if let Some(m) = mode {
        m
    } else {
        // Toggle: get current theme and switch to opposite
        let cache_dir = std::path::PathBuf::from(
            config.cache.dir.replace("~", &dirs::home_dir().unwrap().display().to_string())
        );
        let cache = lmtt_core::cache::Cache::new(cache_dir)?;
        let current = cache.get_theme_state().await?;
        
        let toggled = match current.as_str() {
            "light" => ThemeMode::Dark,
            _ => ThemeMode::Light,
        };
        
        println!("Toggling from {} to {} mode...", current, toggled);
        toggled
    };
    
    println!("Switching to {} mode...", mode);
    
    // Generate color scheme
    let scheme = matugen::generate_colors(&config, mode).await?;
    
    // Apply to all modules
    let results = registry.apply_all(&scheme, &config).await;
    
    // Print results
    let mut successes = 0;
    let mut failures = 0;
    
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
    }
    
    println!("\n{} successful, {} failed", successes, failures);
    
    if failures == 0 {
        // Save theme state to cache
        let cache_dir = std::path::PathBuf::from(
            config.cache.dir.replace("~", &dirs::home_dir().unwrap().display().to_string())
        );
        let cache = lmtt_core::cache::Cache::new(cache_dir)?;
        cache.set_theme_state(&mode.to_string()).await?;
        
        println!("Theme switched to {} mode!", mode);
    }
    
    Ok(())
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
    let cache_dir = std::path::PathBuf::from(
        config.cache.dir.replace("~", &dirs::home_dir().unwrap().display().to_string())
    );
    let cache = lmtt_core::cache::Cache::new(cache_dir)?;
    
    let current_mode = cache.get_theme_state().await?;
    
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
