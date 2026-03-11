use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode, find_icon_theme_variant};

crate::register_module!(XfconfModule);

pub struct XfconfModule;

impl XfconfModule {
    pub fn new() -> Self {
        Self
    }

    async fn xfconf_set(&self, channel: &str, property: &str, value: &str) {
        // Try to set existing property first, fall back to creating it
        let result = tokio::process::Command::new("xfconf-query")
            .args(&["-c", channel, "-p", property, "-s", value])
            .output()
            .await;

        if result.as_ref().map(|o| !o.status.success()).unwrap_or(true) {
            tokio::process::Command::new("xfconf-query")
                .args(&["-c", channel, "-p", property, "-s", value, "--create", "-t", "string"])
                .output()
                .await
                .ok();
        }
    }

    async fn xfconf_set_bool(&self, channel: &str, property: &str, value: bool) {
        let val = if value { "true" } else { "false" };
        let result = tokio::process::Command::new("xfconf-query")
            .args(&["-c", channel, "-p", property, "-s", val])
            .output()
            .await;

        if result.as_ref().map(|o| !o.status.success()).unwrap_or(true) {
            tokio::process::Command::new("xfconf-query")
                .args(&["-c", channel, "-p", property, "-s", val, "--create", "-t", "bool"])
                .output()
                .await
                .ok();
        }
    }
}

#[async_trait]
impl ThemeModule for XfconfModule {
    fn name(&self) -> &'static str {
        "xfconf"
    }

    fn binary_name(&self) -> &'static str {
        "xfconf-query"
    }

    fn priority(&self) -> u8 {
        15  // Platform module - after GTK, before app modules
    }

    async fn apply(&self, _scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let mode = _scheme.mode;
        let profile = match mode {
            ThemeMode::Light => &_config.theme_profiles.light,
            ThemeMode::Dark => &_config.theme_profiles.dark,
        };

        // xfsettingsd's GTK module uses ThemeName + ApplicationPreferDarkTheme to
        // select the dark variant. Strip "-dark" suffix so xfconf always gets the
        // base theme name (e.g. "Adwaita" not "Adwaita-dark"), and let
        // ApplicationPreferDarkTheme (set below) control the variant.
        let theme = profile.gtk_theme.as_deref().unwrap_or("Adwaita");
        let base_theme = theme.strip_suffix("-dark").unwrap_or(theme);

        self.xfconf_set("xsettings", "/Net/ThemeName", base_theme).await;

        // Set icon theme
        let icon_theme = if let Some(ref explicit) = profile.gtk_icon_theme {
            explicit.clone()
        } else {
            let current = tokio::process::Command::new("xfconf-query")
                .args(&["-c", "xsettings", "-p", "/Net/IconThemeName"])
                .output()
                .await
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
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

        self.xfconf_set("xsettings", "/Net/IconThemeName", &icon_theme).await;

        if let Some(ref cursor_theme) = profile.cursor_theme {
            self.xfconf_set("xsettings", "/Gtk/CursorThemeName", cursor_theme).await;
            self.xfconf_set("xsettings", "/Gtk/CursorThemeSize", &profile.cursor_size.to_string()).await;
        }

        // Set ApplicationPreferDarkTheme so xfce's GTK module applies the correct
        // dark-theme preference to Thunar and other XFCE-managed GTK3 apps.
        let prefer_dark = matches!(mode, ThemeMode::Dark);
        self.xfconf_set_bool("xsettings", "/Gtk/ApplicationPreferDarkTheme", prefer_dark).await;

        tracing::info!("[xfconf] Set ThemeName={}, IconThemeName={}, PreferDark={}", base_theme, icon_theme, prefer_dark);

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
