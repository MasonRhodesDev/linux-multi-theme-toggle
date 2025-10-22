use crate::ModuleRegistry;
use lmtt_core::Result;
use std::io::{self, Write};

pub struct CleanupManager {
    registry: ModuleRegistry,
}

impl CleanupManager {
    pub fn new(registry: ModuleRegistry) -> Self {
        Self { registry }
    }
    
    /// Run cleanup - remove all lmtt-injected config lines
    pub async fn run_all(&self) -> Result<()> {
        println!("🧹 LMTT Cleanup");
        println!("==============\n");
        
        println!("This will remove all lmtt-injected config lines from your application configs.\n");
        print!("Continue? [y/N] ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
        
        let mut total_removed = 0;
        let mut total_errors = 0;
        
        for module in &self.registry.modules {
            let config_files = module.config_files().await?;
            
            if config_files.is_empty() {
                continue;
            }
            
            println!("\n{}", module.name());
            
            for config_file in config_files {
                if !config_file.already_included {
                    println!("  ○ {} - nothing to remove", config_file.path.display());
                    continue;
                }
                
                print!("  Removing from {}... ", config_file.path.display());
                io::stdout().flush()?;
                
                match module.remove_config(&config_file).await {
                    Ok(_) => {
                        println!("✓");
                        total_removed += 1;
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                        total_errors += 1;
                    }
                }
            }
        }
        
        println!("\n==============");
        println!("Cleanup Summary:");
        println!("  Removed: {}", total_removed);
        println!("  Errors: {}", total_errors);
        
        if total_removed > 0 {
            println!("\n✓ Cleanup complete. Your config files have been restored.");
        }
        
        Ok(())
    }
    
    /// Cleanup a specific module
    pub async fn run_module(&self, module_name: &str) -> Result<()> {
        println!("🧹 LMTT Cleanup: {}", module_name);
        println!("==============\n");
        
        let module = self.registry.modules
            .iter()
            .find(|m| m.name().eq_ignore_ascii_case(module_name))
            .ok_or_else(|| lmtt_core::Error::Module(
                format!("Module not found: {}", module_name)
            ))?;
        
        let config_files = module.config_files().await?;
        
        if config_files.is_empty() {
            println!("No config files to clean for {}", module_name);
            return Ok(());
        }
        
        let mut removed = 0;
        let mut errors = 0;
        
        for config_file in config_files {
            if !config_file.already_included {
                println!("○ {} - nothing to remove", config_file.path.display());
                continue;
            }
            
            println!("Removing from {}...", config_file.path.display());
            
            match module.remove_config(&config_file).await {
                Ok(_) => {
                    println!("  ✓ Removed: {}", config_file.include_line);
                    removed += 1;
                }
                Err(e) => {
                    println!("  ✗ Error: {}", e);
                    errors += 1;
                }
            }
        }
        
        println!("\nRemoved {} config line(s)", removed);
        if errors > 0 {
            println!("Failed: {}", errors);
        }
        
        Ok(())
    }
    
    /// Dry run - show what would be removed
    pub async fn dry_run(&self) -> Result<()> {
        println!("🔍 LMTT Cleanup (Dry Run)");
        println!("=========================\n");
        
        for module in &self.registry.modules {
            let config_files = module.config_files().await?;
            
            if config_files.is_empty() {
                continue;
            }
            
            println!("Module: {}", module.name());
            
            for config_file in config_files {
                println!("  File: {}", config_file.path.display());
                
                if config_file.already_included {
                    println!("  Status: ⚠ Would remove: {}", config_file.include_line);
                } else {
                    println!("  Status: ○ Nothing to remove");
                }
                println!();
            }
        }
        
        Ok(())
    }
}
