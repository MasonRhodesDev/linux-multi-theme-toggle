use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

pub struct GtkModule;

impl GtkModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for GtkModule {
    fn name(&self) -> &'static str {
        "gtk"
    }
    
    fn binary_name(&self) -> &'static str {
        "gsettings"
    }
    
    fn priority(&self) -> u8 {
        10  // Platform module - run first
    }
    
    async fn apply(&self, _scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let mode = _scheme.mode;
        
        // Set GTK color scheme preference
        let preference = match mode {
            ThemeMode::Light => "prefer-light",
            ThemeMode::Dark => "prefer-dark",
        };
        
        tokio::process::Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "color-scheme", preference])
            .output()
            .await?;
        
        // Set GTK theme
        let theme = match mode {
            ThemeMode::Light => "Adwaita",
            ThemeMode::Dark => "Adwaita-dark",
        };
        
        tokio::process::Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "gtk-theme", theme])
            .output()
            .await?;
        
        tracing::info!("[GTK] Set color-scheme to {}", preference);
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // GTK doesn't need config file injection - it uses gsettings
        Ok(vec![])
    }
}
