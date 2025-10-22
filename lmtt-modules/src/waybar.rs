use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(WaybarModule);

pub struct WaybarModule;

impl WaybarModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for WaybarModule {
    fn name(&self) -> &'static str {
        "waybar"
    }
    
    fn binary_name(&self) -> &'static str {
        "waybar"
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let css_path = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("matugen")
            .join("lmtt-colors.css");
        
        // Ensure directory exists
        if let Some(parent) = css_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Generate CSS file with @define-color declarations
        let css_content = scheme.to_gtk_css();
        tokio::fs::write(&css_path, css_content).await?;
        
        tracing::info!("[Waybar] Updated colors at {}", css_path.display());
        
        // Waybar hot-reloads CSS automatically with reload_style_on_change: true
        // No restart needed!
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        let config_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?;
        
        let style_css = config_dir.join("waybar").join("style.css");
        
        if !style_css.exists() {
            return Ok(vec![]);
        }
        
        // Check if already included
        let content = tokio::fs::read_to_string(&style_css).await?;
        let include_line = "@import url('../matugen/lmtt-colors.css');";
        let already_included = content.contains(include_line);
        
        Ok(vec![ConfigFileInfo {
            path: style_css,
            include_line: include_line.to_string(),
            description: "Import lmtt colors into Waybar CSS".to_string(),
            already_included,
        }])
    }
}
