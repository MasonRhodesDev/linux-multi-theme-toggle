use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};
use serde_json::{Map, Value};

crate::register_module!(VSCodeModule);

pub struct VSCodeModule;

impl VSCodeModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for VSCodeModule {
    fn name(&self) -> &'static str {
        "vscode"
    }
    
    fn binary_name(&self) -> &'static str {
        "code"
    }
    
    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let home = dirs::home_dir()
            .ok_or(lmtt_core::Error::Config("No home dir".into()))?;
        
        let settings_paths = vec![
            home.join(".config/Code/User/settings.json"),
            home.join(".config/Cursor/User/settings.json"),
            home.join(".config/Code - OSS/User/settings.json"),
            home.join(".config/VSCodium/User/settings.json"),
        ];
        
        let is_light = scheme.get("mode").map(|m| m == "light").unwrap_or(false);
        
        let profile = if is_light {
            &config.theme_profiles.light
        } else {
            &config.theme_profiles.dark
        };
        
        let theme = profile.vscode_theme.as_deref().unwrap_or_else(|| {
            if is_light { "Default Light+" } else { "Default Dark+" }
        });
        
        let mut updated_count = 0;
        
        for path in settings_paths {
            if !path.exists() {
                continue;
            }
            
            let content = tokio::fs::read_to_string(&path).await?;
            let mut json: Map<String, Value> = serde_json::from_str(&content).unwrap_or_default();
            
            json.insert("workbench.colorTheme".to_string(), Value::String(theme.to_string()));
            
            let new_content = serde_json::to_string_pretty(&json)?;
            tokio::fs::write(&path, new_content).await?;
            
            updated_count += 1;
            tracing::info!("[VSCode] Updated {}", path.display());
        }
        
        if updated_count == 0 {
            tracing::debug!("[VSCode] No installations found");
        }
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
