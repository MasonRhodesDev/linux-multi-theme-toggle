use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(WaybarModule);

pub struct WaybarModule;

impl Default for WaybarModule {
    fn default() -> Self {
        Self::new()
    }
}

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
    
    async fn apply(&self, _scheme: &ColorScheme, _config: &Config) -> Result<()> {
        // The shared palette (~/.config/matugen/lmtt-colors.css, including
        // the #tray rule) is written once, atomically, by the main switch
        // path before any module runs — this module must never write it too,
        // or the two writers race waybar's inotify reload.
        //
        // Waybar hot-reloads CSS via reload_style_on_change: true.
        // Symbolic icons in hicolor use currentColor and are recolored by CSS.
        tracing::debug!("[Waybar] Palette updated by core; waybar reloads via reload_style_on_change");
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
        let already_included = crate::is_included(&content, include_line);
        
        Ok(vec![ConfigFileInfo {
            path: style_css,
            include_line: include_line.to_string(),
            description: "Import lmtt colors into Waybar CSS".to_string(),
            already_included,
        }])
    }
}
