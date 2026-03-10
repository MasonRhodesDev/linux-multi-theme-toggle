use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

crate::register_module!(XdgModule);

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
        "dbus-send"
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

        // XDG_CURRENT_DESKTOP and GTK_USE_PORTAL are session-level env vars
        // that don't change between light/dark switches. They should be set
        // once at login (e.g. in hyprland.conf), not on every theme switch.
        // Setting them here via dbus-update-activation-environment can
        // disrupt the portal and cause apps to miss theme signals.

        // Portal color-scheme values: 1 = dark, 2 = light
        let expected_value: u32 = match mode {
            ThemeMode::Dark => 1,
            ThemeMode::Light => 2,
        };

        // The GTK module (priority 10) sets gsettings before us. The portal
        // backend (xdg-desktop-portal-gtk) detects the gsettings change and
        // emits SettingChanged from the real portal bus name. We do NOT need
        // to emit a fake signal — apps only accept signals from the real
        // portal sender.
        //
        // Wait for dconf → portal propagation, then verify.
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        // Verify the portal reflects the correct value (logging only)
        match tokio::process::Command::new("dbus-send")
            .args(&[
                "--session",
                "--print-reply",
                "--dest=org.freedesktop.portal.Desktop",
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.portal.Settings.ReadOne",
                "string:org.freedesktop.appearance",
                "string:color-scheme",
            ])
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.lines().any(|line| line.contains(&format!("uint32 {}", expected_value))) {
                    tracing::info!("[XDG] Portal reports correct value ({}), signal emitted by portal", expected_value);
                } else {
                    tracing::warn!("[XDG] Portal still reports stale value — apps may not have received signal");
                    tracing::debug!("[XDG] Portal ReadOne output: {}", stdout.trim());
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("[XDG] Portal query failed: {}", stderr.trim());
            }
            Err(e) => {
                tracing::warn!("[XDG] Failed to query portal: {}", e);
            }
        }

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // XDG doesn't need config file injection
        Ok(vec![])
    }
}
