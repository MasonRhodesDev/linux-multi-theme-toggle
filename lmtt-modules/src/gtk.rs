use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode, find_icon_theme_variant};

crate::register_module!(GtkModule);

pub struct GtkModule;

impl GtkModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for GtkModule {
    fn name(&self) -> &'static str {
        "gtk"
    }

    fn binary_name(&self) -> &'static str {
        "gsettings"
    }

    fn priority(&self) -> u8 {
        10  // Platform module - run first
    }

    async fn apply(&self, _scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let mode = _scheme.mode;
        let profile = match mode {
            ThemeMode::Light => &_config.theme_profiles.light,
            ThemeMode::Dark => &_config.theme_profiles.dark,
        };

        let preference = match mode {
            ThemeMode::Light => "prefer-light",
            ThemeMode::Dark => "prefer-dark",
        };

        // Set gtk-theme, icon-theme, and cursor BEFORE color-scheme.
        // Changing any org.gnome.desktop.interface key causes Electron/Chromium
        // to re-evaluate the system theme via nativeTheme. If color-scheme is
        // set first, a subsequent gtk-theme change triggers a re-evaluation that
        // races with the portal signal and reverts Electron apps to light mode.
        // By setting color-scheme last, it's the final signal apps see.

        let theme = profile.gtk_theme.as_deref().unwrap_or(match mode {
            ThemeMode::Light => "Adwaita",
            ThemeMode::Dark => "Adwaita-dark",
        });

        tokio::process::Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "gtk-theme", theme])
            .output()
            .await?;

        // Set icon theme: explicit config > auto-detect variant > fallback to Adwaita
        let icon_theme = if let Some(ref explicit) = profile.gtk_icon_theme {
            explicit.clone()
        } else {
            let current = tokio::process::Command::new("gsettings")
                .args(&["get", "org.gnome.desktop.interface", "icon-theme"])
                .output()
                .await
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok().map(|s| {
                            s.trim().trim_matches('\'').to_string()
                        })
                    } else {
                        None
                    }
                });

            match current {
                Some(current_theme) => {
                    find_icon_theme_variant(&current_theme, mode)
                        .unwrap_or(current_theme)
                }
                None => "Adwaita".to_string(),
            }
        };

        tokio::process::Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "icon-theme", &icon_theme])
            .output()
            .await?;

        if let Some(ref cursor_theme) = profile.cursor_theme {
            tokio::process::Command::new("gsettings")
                .args(&["set", "org.gnome.desktop.interface", "cursor-theme", cursor_theme])
                .output()
                .await?;

            let cursor_size = profile.cursor_size.to_string();
            tokio::process::Command::new("gsettings")
                .args(&["set", "org.gnome.desktop.interface", "cursor-size", &cursor_size])
                .output()
                .await?;
        }

        // color-scheme MUST be last — this is the signal Electron apps use via
        // the portal's SettingChanged. Setting it after all other gsettings
        // changes prevents re-evaluation races.
        tokio::process::Command::new("gsettings")
            .args(&["set", "org.gnome.desktop.interface", "color-scheme", preference])
            .output()
            .await?;

        tracing::info!("[GTK] Set gtk-theme to {}, icon-theme to {}, color-scheme to {}",
            theme, icon_theme, preference);

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // GTK doesn't need config file injection - it uses gsettings
        Ok(vec![])
    }
}
