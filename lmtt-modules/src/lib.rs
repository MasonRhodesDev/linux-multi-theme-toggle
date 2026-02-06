pub mod registry;
pub mod setup;
pub mod cleanup;
pub mod custom;
pub mod gtk;
pub mod xdg;
pub mod hyprland;
pub mod waybar;
pub mod wofi;
pub mod fuzzel;
pub mod tmux;
pub mod swaync;
pub mod wezterm;
pub mod vscode;
pub mod nvim;
pub mod fish;
pub mod qt;
pub mod hyprpanel;

use async_trait::async_trait;
use lmtt_core::{ColorScheme, Result, Config};
use std::path::PathBuf;
use std::sync::Arc;

/// Information about a config file that needs lmtt integration
#[derive(Debug, Clone)]
pub struct ConfigFileInfo {
    /// Path to the config file
    pub path: PathBuf,
    
    /// The import/include line that should be added
    pub include_line: String,
    
    /// Description of what this does
    pub description: String,
    
    /// Whether the include is already present
    pub already_included: bool,
}

/// Standard trait that all theme modules must implement
#[async_trait]
pub trait ThemeModule: Send + Sync {
    /// Module name (e.g., "Waybar", "Hyprland")
    fn name(&self) -> &'static str;
    
    /// Binary name to check for installation (e.g., "waybar", "hyprctl")
    fn binary_name(&self) -> &'static str;
    
    /// Check if the application is installed on the system
    fn is_installed(&self) -> bool {
        which::which(self.binary_name()).is_ok()
    }
    
    /// Apply theme (non-blocking, returns immediately)
    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()>;
    
    /// Get config file(s) that need lmtt integration (for setup mode)
    /// Returns None if this module doesn't need config injection
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
    
    /// Inject include line into config file
    async fn inject_config(&self, config_file: &ConfigFileInfo) -> Result<()> {
        let path = &config_file.path;
        
        if !path.exists() {
            return Err(lmtt_core::Error::Module(
                format!("Config file not found: {}", path.display())
            ));
        }
        
        // Read existing content
        let content = tokio::fs::read_to_string(path).await?;
        
        // Check if already included
        if content.contains(&config_file.include_line) {
            return Ok(());
        }
        
        // Add lmtt marker comments for easy removal
        let marker_start = "# >>> lmtt managed block - do not edit manually >>>";
        let marker_end = "# <<< lmtt managed block <<<";
        
        let inject_block = format!("{}\n{}\n{}", marker_start, config_file.include_line, marker_end);
        
        // Prepend include block at the top
        let new_content = format!("{}\n\n{}", inject_block, content);
        
        // Write back
        tokio::fs::write(path, new_content).await?;
        
        Ok(())
    }
    
    /// Remove lmtt-injected config from file (for cleanup)
    async fn remove_config(&self, config_file: &ConfigFileInfo) -> Result<()> {
        let path = &config_file.path;
        
        if !path.exists() {
            return Ok(()); // Already gone
        }
        
        let content = tokio::fs::read_to_string(path).await?;
        
        // Remove entire lmtt managed block
        let marker_start = "# >>> lmtt managed block - do not edit manually >>>";
        let marker_end = "# <<< lmtt managed block <<<";
        
        if let Some(start_idx) = content.find(marker_start) {
            if let Some(end_idx) = content[start_idx..].find(marker_end) {
                let end_idx = start_idx + end_idx + marker_end.len();
                
                // Remove the block and any trailing newlines
                let mut new_content = format!(
                    "{}{}",
                    &content[..start_idx],
                    &content[end_idx..]
                );
                
                // Clean up excessive newlines
                new_content = new_content.trim_start().to_string();
                
                tokio::fs::write(path, new_content).await?;
                return Ok(());
            }
        }
        
        // Fallback: if no markers, try to remove just the include line
        if content.contains(&config_file.include_line) {
            let new_content = content.replace(&format!("{}\n", config_file.include_line), "");
            tokio::fs::write(path, new_content).await?;
        }
        
        Ok(())
    }
    
    /// Optional: Module-specific health check
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
    
    /// Optional: Priority (lower = runs first, for dependencies)
    /// Platform modules (GTK, XDG, Qt) should have priority < 50
    /// Application modules should have priority >= 100
    fn priority(&self) -> u8 {
        100
    }
    
    /// Whether this module is enabled (checks config and installation)
    fn is_enabled(&self, config: &Config) -> bool {
        // Check config (defaults to true)
        if !config.is_module_enabled(self.name()) {
            return false;
        }
        
        // Check if installed
        if !self.is_installed() {
            tracing::debug!("[{}] Not installed, skipping", self.name());
            return false;
        }
        
        true
    }
}

pub use registry::ModuleRegistry;
pub use setup::SetupManager;
pub use cleanup::CleanupManager;

/// Constructor function type for module auto-registration
pub struct ModuleConstructor {
    pub constructor: fn() -> Arc<dyn ThemeModule>,
}

inventory::collect!(ModuleConstructor);

/// Macro to auto-register a module
#[macro_export]
macro_rules! register_module {
    ($module:ty) => {
        inventory::submit! {
            $crate::ModuleConstructor {
                constructor: || std::sync::Arc::new(<$module>::new())
            }
        }
    };
}
