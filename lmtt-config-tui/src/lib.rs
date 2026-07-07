pub mod detectors;

pub use detectors::*;

use anyhow::Result;
use schema_tui::SchemaTUIBuilder;

/// Run the LMTT configuration TUI
pub fn run_config_tui() -> Result<()> {
    // Get config directory
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
        .join("lmtt");
    
    // Ensure config directory exists
    std::fs::create_dir_all(&config_dir)?;
    
    let schema_path = config_dir.join("config-schema.json");
    let config_path = config_dir.join("config.toml");
    
    // Always refresh the schema from the bundled asset. It's a generated UI
    // schema (not user-edited — the user's values live in config.toml), so
    // copying only-if-absent left upgraded installs on a stale schema with
    // outdated defaults/labels. Overwriting each run keeps it current.
    std::fs::write(&schema_path, include_str!("../assets/config-schema.json"))?;
    
    // Create default config if it doesn't exist
    if !config_path.exists() {
        let default_config = lmtt_core::Config::default();
        default_config.save()?;
    }
    
    // Build TUI with LMTT-specific option providers
    let builder = SchemaTUIBuilder::new()
        .schema_file(&schema_path)?
        .config_file(&config_path)?;
    
    // Register LMTT detectors
    let builder = register_all(builder);
    
    let mut tui = builder.build()?;
    
    // Register save handler
    tui.on_change(move |_key, _value| {
        // Changes are auto-saved by schema-tui
    });
    
    tui.run()?;
    
    // After exiting TUI, apply the current theme from config
    apply_theme_on_exit()?;
    
    Ok(())
}

fn apply_theme_on_exit() -> Result<()> {
    // Load the config to get the current default_mode
    let config = lmtt_core::Config::load()?;
    let mode = config.general.default_mode;
    
    println!("\nApplying {} theme with updated configuration...", mode);
    
    // Call lmtt switch command as subprocess
    let output = std::process::Command::new("lmtt")
        .arg("switch")
        .arg(mode.to_string().to_lowercase())
        .output()?;
    
    // Print output
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    
    if !output.status.success() {
        anyhow::bail!("Failed to apply theme");
    }
    
    Ok(())
}
