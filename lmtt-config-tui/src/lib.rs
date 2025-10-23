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
    
    // Copy schema from assets if it doesn't exist
    if !schema_path.exists() {
        let schema_content = include_str!("../assets/config-schema.json");
        std::fs::write(&schema_path, schema_content)?;
    }
    
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
    
    Ok(())
}
