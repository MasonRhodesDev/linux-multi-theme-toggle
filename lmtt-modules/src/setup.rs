use crate::ModuleRegistry;
use lmtt_core::{Config, Result};
use std::io::{self, Write};

pub struct SetupManager {
    registry: ModuleRegistry,
}

impl SetupManager {
    pub fn new(registry: ModuleRegistry) -> Self {
        Self { registry }
    }
    
    /// Run setup mode - check all installed apps and prompt for config injection
    pub async fn run(&self, _config: &Config) -> Result<()> {
        println!("🔧 LMTT Setup Mode");
        println!("================\n");
        
        println!("Checking installed applications and their config files...\n");
        
        let mut total_modules = 0;
        let mut total_configs = 0;
        let mut injected = 0;
        let mut skipped = 0;
        
        for module in &self.registry.modules {
            if !module.is_installed() {
                continue;
            }
            
            total_modules += 1;
            
            println!("✓ {} detected", module.name());
            
            let config_files = module.config_files().await?;
            
            if config_files.is_empty() {
                println!("  → No config injection needed\n");
                continue;
            }
            
            for config_file in config_files {
                total_configs += 1;
                
                println!("  📄 {}", config_file.path.display());
                println!("     {}", config_file.description);
                
                if config_file.already_included {
                    println!("     ✓ Already configured\n");
                    continue;
                }
                
                println!("     ⚠ Include line missing:");
                println!("     {}\n", config_file.include_line);
                
                // Prompt user
                print!("     Inject this line? [Y/n/q] ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let choice = input.trim().to_lowercase();
                
                match choice.as_str() {
                    "q" => {
                        println!("\nSetup cancelled.\n");
                        return Ok(());
                    }
                    "n" => {
                        println!("     Skipped. You'll need to add this manually.\n");
                        skipped += 1;
                    }
                    "" | "y" => {
                        // Inject
                        match module.inject_config(&config_file).await {
                            Ok(_) => {
                                println!("     ✓ Injected successfully!\n");
                                injected += 1;
                            }
                            Err(e) => {
                                println!("     ✗ Failed to inject: {}\n", e);
                                skipped += 1;
                            }
                        }
                    }
                    _ => {
                        println!("     Invalid choice, skipping.\n");
                        skipped += 1;
                    }
                }
            }
        }
        
        // Summary
        println!("================");
        println!("Setup Summary:");
        println!("  Modules detected: {}", total_modules);
        println!("  Config files found: {}", total_configs);
        println!("  Injected: {}", injected);
        println!("  Skipped: {}", skipped);
        
        if skipped > 0 {
            println!("\n⚠ {} config file(s) were skipped.", skipped);
            println!("You'll need to add the include lines manually.");
            println!("Run 'lmtt setup --dry-run' to see what needs to be added.");
        } else if injected > 0 {
            println!("\n✓ All config files updated successfully!");
            println!("You can now run 'lmtt switch dark' or 'lmtt switch light'");
        } else {
            println!("\n✓ Everything is already configured!");
        }
        
        Ok(())
    }
    
    /// Run dry-run mode - show what would be changed without prompting
    pub async fn dry_run(&self) -> Result<()> {
        println!("🔍 LMTT Setup (Dry Run)");
        println!("======================\n");
        
        for module in &self.registry.modules {
            if !module.is_installed() {
                continue;
            }
            
            let config_files = module.config_files().await?;
            
            if config_files.is_empty() {
                continue;
            }
            
            println!("Module: {}", module.name());
            
            for config_file in config_files {
                println!("  File: {}", config_file.path.display());
                println!("  Description: {}", config_file.description);
                
                if config_file.already_included {
                    println!("  Status: ✓ Already configured");
                } else {
                    println!("  Status: ⚠ Needs injection");
                    println!("  Include line: {}", config_file.include_line);
                }
                println!();
            }
        }
        
        Ok(())
    }
}
