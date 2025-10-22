use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

pub struct WofiModule;

impl WofiModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for WofiModule {
    fn name(&self) -> &'static str {
        "wofi"
    }
    
    fn binary_name(&self) -> &'static str {
        "wofi"
    }
    
    async fn apply(&self, _scheme: &ColorScheme, _config: &Config) -> Result<()> {
        // Wofi uses the same CSS colors as waybar
        tracing::info!("[Wofi] Using centralized lmtt-colors.css");
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        let config_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?;
        
        let style_css = config_dir.join("wofi").join("style.css");
        
        if !style_css.exists() {
            return Ok(vec![]);
        }
        
        let content = tokio::fs::read_to_string(&style_css).await?;
        let include_line = "@import url('../matugen/lmtt-colors.css');";
        let already_included = content.contains(include_line);
        
        Ok(vec![ConfigFileInfo {
            path: style_css,
            include_line: include_line.to_string(),
            description: "Import lmtt colors into Wofi CSS".to_string(),
            already_included,
        }])
    }
}
