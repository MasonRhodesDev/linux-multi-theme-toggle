use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(QtModule);

pub struct QtModule;

impl QtModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for QtModule {
    fn name(&self) -> &'static str {
        "qt"
    }
    
    fn binary_name(&self) -> &'static str {
        "qt6ct"
    }
    
    fn priority(&self) -> u8 {
        20
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let home = dirs::home_dir()
            .ok_or(lmtt_core::Error::Config("No home dir".into()))?;
        
        let is_light = scheme.get("mode").map(|m| m == "light").unwrap_or(false);
        let mode = if is_light { "light" } else { "dark" };
        
        let colorscheme_dir = home.join(".local/share/color-schemes");
        tokio::fs::create_dir_all(&colorscheme_dir).await?;
        
        let scheme_file = colorscheme_dir.join(format!("lmtt-{}.colors", mode));
        
        let default_fg = "#e3e1ec".to_string();
        let default_bg = "#12131a".to_string();
        let default_primary = "#9fd491".to_string();
        
        let fg = scheme.get("on_surface").unwrap_or(&default_fg);
        let bg = scheme.get("surface").unwrap_or(&default_bg);
        let primary = scheme.get("primary").unwrap_or(&default_primary);
        
        let kde_colors = format!(
            "[ColorScheme]\nName=lmtt-{}\n\n[Colors:Window]\nForegroundNormal={}\nBackgroundNormal={}\n\n[Colors:Button]\nBackgroundNormal={}\n\n[Colors:Selection]\nBackgroundNormal={}\n",
            mode, fg, bg, bg, primary
        );
        
        tokio::fs::write(&scheme_file, kde_colors).await?;
        
        let env_commands = vec![
            ("systemctl", vec!["--user", "set-environment", "QT_QPA_PLATFORMTHEME=qt6ct"]),
            ("dbus-update-activation-environment", vec!["--systemd", "QT_QPA_PLATFORMTHEME=qt6ct"]),
        ];
        
        for (cmd, args) in env_commands {
            tokio::process::Command::new(cmd)
                .args(&args)
                .output()
                .await
                .ok();
        }
        
        if which::which("hyprctl").is_ok() {
            tokio::process::Command::new("hyprctl")
                .args(&["setenv", "QT_QPA_PLATFORMTHEME", "qt6ct"])
                .output()
                .await
                .ok();
        }
        
        tracing::info!("[Qt] Updated colorscheme at {}", scheme_file.display());
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
