use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

crate::register_module!(XdgModule);

pub struct XdgModule;

impl Default for XdgModule {
    fn default() -> Self {
        Self::new()
    }
}

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

        // Ensure XDG portal service is running. systemd is optional here —
        // the module is gated on dbus-send, so a non-systemd session must
        // degrade gracefully rather than error out.
        if which::which("systemctl").is_ok() {
            let active = tokio::process::Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", "xdg-desktop-portal"])
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false);

            if !active {
                tracing::info!("[XDG] Starting xdg-desktop-portal service");
                if let Err(e) = tokio::process::Command::new("systemctl")
                    .args(["--user", "start", "xdg-desktop-portal"])
                    .output()
                    .await
                {
                    tracing::warn!("[XDG] Could not start portal service: {}", e);
                }
            } else {
                tracing::debug!("[XDG] Portal service already running");
            }
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
        // Poll for dconf → portal propagation instead of a fixed sleep: this
        // runs in the sequential platform phase, so every 100ms saved here is
        // felt on every switch. Verification is logging-only either way.
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(1000);
        let mut confirmed = false;
        loop {
            match tokio::process::Command::new("dbus-send")
                .args([
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
                        confirmed = true;
                        break;
                    }
                }
                Ok(output) => {
                    tracing::warn!(
                        "[XDG] Portal query failed: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                    break;
                }
                Err(e) => {
                    tracing::warn!("[XDG] Failed to query portal: {}", e);
                    break;
                }
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        if confirmed {
            tracing::info!("[XDG] Portal reports correct value ({}), signal emitted by portal", expected_value);
        } else {
            tracing::warn!("[XDG] Portal did not confirm color-scheme {} — apps may not have received the signal", expected_value);
        }

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // XDG doesn't need config file injection
        Ok(vec![])
    }
}
