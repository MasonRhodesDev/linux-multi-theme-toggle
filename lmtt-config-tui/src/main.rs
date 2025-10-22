use anyhow::Result;
use lmtt_config_tui::register_all;
use schema_tui::SchemaTUIBuilder;

fn main() -> Result<()> {
    println!("LMTT Configuration TUI");
    println!("======================\n");
    
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
        println!("Installing config schema to {:?}", schema_path);
        let schema_content = include_str!("../assets/config-schema.json");
        std::fs::write(&schema_path, schema_content)?;
    }
    
    // Create default config if it doesn't exist
    if !config_path.exists() {
        println!("Creating default config at {:?}", config_path);
        let default_config = lmtt_core::Config::default();
        default_config.save()?;
    }
    
    println!("Loading schema from: {:?}", schema_path);
    println!("Loading config from: {:?}\n", config_path);
    
    // Build TUI with LMTT-specific option providers
    let builder = SchemaTUIBuilder::new()
        .schema_file(&schema_path)?
        .config_file(&config_path)?;
    
    // Register LMTT detectors
    let builder = register_all(builder);
    
    let _tui = builder.build()?;
    
    println!("✓ Schema and config loaded successfully");
    println!("✓ LMTT option detectors registered\n");
    
    // Note: TUI run() is not yet implemented
    println!("Note: Full TUI not yet implemented");
    println!("Config successfully validated and loaded!");
    
    // tui.run()?; // Will work once TUI app is implemented
    
    Ok(())
}
