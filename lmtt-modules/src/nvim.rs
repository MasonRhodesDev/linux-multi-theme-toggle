use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(NvimModule);

pub struct NvimModule;

impl NvimModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for NvimModule {
    fn name(&self) -> &'static str {
        "nvim"
    }
    
    fn binary_name(&self) -> &'static str {
        "nvim"
    }
    
    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let is_light = scheme.get("mode").map(|m| m == "light").unwrap_or(false);
        
        let profile = if is_light {
            &config.theme_profiles.light
        } else {
            &config.theme_profiles.dark
        };
        
        let colorscheme = profile.neovim_colorscheme.as_deref();
        let mode = if is_light { "light" } else { "dark" };
        
        let mut updated = 0;
        
        if let Ok(entries) = tokio::fs::read_dir("/tmp").await {
            let mut entries = entries;
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("nvim") {
                    let socket = path.join("0");
                    if socket.exists() {
                        let cmd = if let Some(cs) = colorscheme {
                            format!("_G.set_nvim_theme('{}', '{}')", mode, cs)
                        } else {
                            format!("_G.set_nvim_theme('{}')", mode)
                        };
                        
                        let result = tokio::process::Command::new("nvim")
                            .arg("--server")
                            .arg(&socket)
                            .arg("--remote-expr")
                            .arg(&cmd)
                            .output()
                            .await;
                        
                        if result.is_ok() {
                            updated += 1;
                        }
                    }
                }
            }
        }
        
        if updated > 0 {
            tracing::info!("[Nvim] Updated {} instance(s)", updated);
        } else {
            tracing::debug!("[Nvim] No running instances found");
        }
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
