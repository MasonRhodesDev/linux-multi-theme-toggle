use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(SwayNCModule);

pub struct SwayNCModule;

impl SwayNCModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for SwayNCModule {
    fn name(&self) -> &'static str {
        "swaync"
    }
    
    fn binary_name(&self) -> &'static str {
        "swaync"
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        // Write colors to shared matugen directory (same as waybar)
        let css_path = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("matugen")
            .join("lmtt-colors.css");

        // Ensure directory exists
        if let Some(parent) = css_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Generate CSS file with @define-color declarations (same as waybar)
        let css_content = scheme.to_gtk_css();
        tokio::fs::write(&css_path, css_content).await?;

        tracing::info!("[SwayNC] Updated colors at {}", css_path.display());

        // Reload SwayNC CSS
        tokio::process::Command::new("swaync-client")
            .arg("--reload-css")
            .output()
            .await
            .ok();

        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        let config_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?;

        let style_css = config_dir.join("swaync").join("style.css");

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
            description: "Import lmtt colors into SwayNC CSS".to_string(),
            already_included,
        }])
    }
}
