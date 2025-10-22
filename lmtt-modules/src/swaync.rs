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
        let style_file = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("swaync")
            .join("style.css");
        
        if let Some(parent) = style_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        // Extract colors with defaults
        let default_surface = "#12131a".to_string();
        let default_on_surface = "#e3e1ec".to_string();
        let default_primary = "#9fd491".to_string();
        let default_error = "#ffb4ab".to_string();
        
        let surface = scheme.get("surface").unwrap_or(&default_surface);
        let on_surface = scheme.get("on_surface").unwrap_or(&default_on_surface);
        let on_surface_variant = scheme.get("on_surface_variant").unwrap_or(&default_on_surface);
        let primary = scheme.get("primary").unwrap_or(&default_primary);
        let error = scheme.get("error").unwrap_or(&default_error);
        
        // Generate SwayNC CSS
        let css = format!(r#"/* SwayNC Theme - Material You (lmtt) */

@define-color cc-bg {};
@define-color noti-bg {};
@define-color noti-bg-opaque {};
@define-color noti-border-color transparent;
@define-color noti-close-bg {};
@define-color noti-close-bg-hover {};
@define-color text-color {};
@define-color text-color-disabled {};
@define-color bg-selected {};

* {{
  font-family: "SF Pro Text", sans-serif;
  font-size: 14px;
}}

.notification-row {{
  outline: none;
  background: transparent;
}}

.notification {{
  background: @noti-bg;
  border-radius: 12px;
  padding: 12px;
  margin: 6px;
}}

.control-center {{
  background: @cc-bg;
  border-radius: 16px;
  padding: 16px;
}}

.notification-content {{
  color: @text-color;
}}

.close-button {{
  background: @noti-close-bg;
  border-radius: 6px;
}}

.close-button:hover {{
  background: @noti-close-bg-hover;
}}
"#, surface, surface, surface, error, error, on_surface, on_surface_variant, primary);
        
        tokio::fs::write(&style_file, css).await?;
        
        // Reload SwayNC
        tokio::process::Command::new("swaync-client")
            .arg("--reload-css")
            .output()
            .await
            .ok();
        
        tracing::info!("[SwayNC] Updated style at {}", style_file.display());
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // SwayNC style.css is fully managed by lmtt, no injection needed
        Ok(vec![])
    }
}
