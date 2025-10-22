use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

pub struct XdgModule;

impl XdgModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for XdgModule {
    fn name(&self) -> &'static str {
        "xdg"
    }
    
    fn binary_name(&self) -> &'static str {
        "gdbus"
    }
    
    fn priority(&self) -> u8 {
        15 // Platform module - run after GTK but before apps
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let mode = scheme.mode;
        
        // Ensure XDG portal service is running
        let status = tokio::process::Command::new("systemctl")
            .args(&["--user", "is-active", "--quiet", "xdg-desktop-portal"])
            .status()
            .await?;
        
        if !status.success() {
            tracing::info!("[XDG] Starting xdg-desktop-portal service");
            tokio::process::Command::new("systemctl")
                .args(&["--user", "start", "xdg-desktop-portal"])
                .output()
                .await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        } else {
            tracing::debug!("[XDG] Portal service already running");
        }
        
        // Set XDG environment variables
        tokio::process::Command::new("systemctl")
            .args(&["--user", "set-environment", "XDG_CURRENT_DESKTOP=Hyprland"])
            .output()
            .await?;
        
        tokio::process::Command::new("systemctl")
            .args(&["--user", "set-environment", "GTK_USE_PORTAL=1"])
            .output()
            .await?;
        
        // Update D-Bus activation environment
        tokio::process::Command::new("dbus-update-activation-environment")
            .args(&["--systemd", "XDG_CURRENT_DESKTOP=Hyprland", "GTK_USE_PORTAL=1"])
            .output()
            .await?;
        
        // Set Hyprland environment
        if which::which("hyprctl").is_ok() {
            tokio::process::Command::new("hyprctl")
                .args(&["setenv", "XDG_CURRENT_DESKTOP", "Hyprland"])
                .output()
                .await
                .ok();
            
            tokio::process::Command::new("hyprctl")
                .args(&["setenv", "GTK_USE_PORTAL", "1"])
                .output()
                .await
                .ok();
        }
        
        // Emit portal signal for color-scheme change
        // Values: 1 = dark, 2 = light
        let value = match mode {
            ThemeMode::Dark => "1",
            ThemeMode::Light => "2",
        };
        
        tokio::process::Command::new("gdbus")
            .args(&[
                "emit",
                "--session",
                "--object-path", "/org/freedesktop/portal/desktop",
                "--signal", "org.freedesktop.portal.Settings.SettingChanged",
                "org.freedesktop.appearance",
                "color-scheme",
                &format!("<uint32 {}>", value),
            ])
            .output()
            .await
            .ok();
        
        tracing::info!("[XDG] Emitted portal signal for {} mode ({})", mode, value);
        
        Ok(())
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // XDG doesn't need config file injection
        Ok(vec![])
    }
}
