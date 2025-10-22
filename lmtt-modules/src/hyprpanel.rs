use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};
use serde_json::{Map, Value};

crate::register_module!(HyprPanelModule);

pub struct HyprPanelModule;

impl HyprPanelModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for HyprPanelModule {
    fn name(&self) -> &'static str {
        "hyprpanel"
    }
    
    fn binary_name(&self) -> &'static str {
        "hyprpanel"
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        if which::which("swaync").is_ok() {
            tracing::debug!("[HyprPanel] SwayNC installed, giving it priority");
            
            if let Ok(output) = tokio::process::Command::new("pgrep")
                .arg("-x")
                .arg("hyprpanel")
                .output()
                .await
            {
                if output.status.success() {
                    tokio::process::Command::new("pkill")
                        .arg("hyprpanel")
                        .output()
                        .await
                        .ok();
                    tracing::info!("[HyprPanel] Stopped to prevent conflicts with SwayNC");
                }
            }
            return Ok(());
        }
        
        let config_file = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("hyprpanel")
            .join("config.json");
        
        if !config_file.exists() {
            return Ok(());
        }
        
        let is_light = scheme.get("mode").map(|m| m == "light").unwrap_or(false);
        let mode = if is_light { "light" } else { "dark" };
        
        let content = tokio::fs::read_to_string(&config_file).await?;
        let mut json: Map<String, Value> = serde_json::from_str(&content).unwrap_or_default();
        
        json.insert("theme.matugen_settings.mode".to_string(), Value::String(mode.to_string()));
        
        let new_content = serde_json::to_string_pretty(&json)?;
        tokio::fs::write(&config_file, new_content).await?;
        
        if let Ok(output) = tokio::process::Command::new("pgrep")
            .arg("-x")
            .arg("hyprpanel")
            .output()
            .await
        {
            if output.status.success() {
                tokio::process::Command::new("pkill")
                    .arg("hyprpanel")
                    .output()
                    .await
                    .ok();
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                
                tokio::process::Command::new("hyprpanel")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .ok();
                
                tracing::info!("[HyprPanel] Restarted with {} theme", mode);
            }
        }
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
