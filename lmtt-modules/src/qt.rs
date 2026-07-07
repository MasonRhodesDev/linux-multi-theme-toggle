use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(QtModule);

pub struct QtModule;

impl Default for QtModule {
    fn default() -> Self {
        Self::new()
    }
}

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
        // Qt light/dark switching rides the portal color-scheme signal (set by
        // the gtk/xdg modules); qt6ct supplies the widget style. All this
        // module does is make sure QT_QPA_PLATFORMTHEME points at qt6ct —
        // and only when the session doesn't already define a platform theme,
        // so qt5ct/kvantum users aren't stomped on every switch.
        let already_set = std::env::var("QT_QPA_PLATFORMTHEME").is_ok()
            || session_env_has_platformtheme().await;

        if already_set {
            tracing::debug!("[Qt] QT_QPA_PLATFORMTHEME already set, leaving session env alone");
        } else {
            let env_commands = vec![
                ("systemctl", vec!["--user", "set-environment", "QT_QPA_PLATFORMTHEME=qt6ct"]),
                ("dbus-update-activation-environment", vec!["--systemd", "QT_QPA_PLATFORMTHEME=qt6ct"]),
            ];

            for (cmd, args) in env_commands {
                if which::which(cmd).is_err() {
                    continue;
                }
                let result = tokio::process::Command::new(cmd).args(&args).output().await;
                match result {
                    Ok(output) if !output.status.success() => {
                        tracing::warn!(
                            "[Qt] {} failed: {}",
                            cmd,
                            String::from_utf8_lossy(&output.stderr).trim()
                        );
                    }
                    Err(e) => tracing::warn!("[Qt] {} failed to run: {}", cmd, e),
                    _ => {}
                }
            }

            if which::which("hyprctl").is_ok() {
                let _ = tokio::process::Command::new("hyprctl")
                    .args(["setenv", "QT_QPA_PLATFORMTHEME", "qt6ct"])
                    .output()
                    .await;
            }
            tracing::info!("[Qt] Exported QT_QPA_PLATFORMTHEME=qt6ct for new processes");
        }

        tracing::debug!(
            "[Qt] {} mode follows the portal color-scheme; palette styling is qt6ct's",
            scheme.mode
        );

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}

/// Whether the systemd user session already exports QT_QPA_PLATFORMTHEME.
async fn session_env_has_platformtheme() -> bool {
    let Ok(output) = tokio::process::Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
        .await
    else {
        return false;
    };
    output.status.success()
        && String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line.starts_with("QT_QPA_PLATFORMTHEME="))
}
