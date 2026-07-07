use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode, find_icon_theme_variant};

crate::register_module!(XfconfModule);

pub struct XfconfModule;

impl Default for XfconfModule {
    fn default() -> Self {
        Self::new()
    }
}

impl XfconfModule {
    pub fn new() -> Self {
        Self
    }

    async fn xfconf_set_typed(&self, channel: &str, property: &str, value: &str, type_name: &str) -> Result<()> {
        // Try to set existing property first, fall back to creating it.
        // A failure of BOTH attempts is a real failure and must surface.
        let set = tokio::process::Command::new("xfconf-query")
            .args(["-c", channel, "-p", property, "-s", value])
            .output()
            .await;

        if matches!(&set, Ok(o) if o.status.success()) {
            return Ok(());
        }

        let create = tokio::process::Command::new("xfconf-query")
            .args(["-c", channel, "-p", property, "-s", value, "--create", "-t", type_name])
            .output()
            .await
            .map_err(|e| lmtt_core::Error::Module(format!("xfconf-query failed to run: {}", e)))?;

        if !create.status.success() {
            return Err(lmtt_core::Error::Module(format!(
                "xfconf-query could not set {}{}: {}",
                channel,
                property,
                String::from_utf8_lossy(&create.stderr).trim()
            )));
        }
        Ok(())
    }

    async fn xfconf_set(&self, channel: &str, property: &str, value: &str) -> Result<()> {
        self.xfconf_set_typed(channel, property, value, "string").await
    }

    async fn xfconf_set_bool(&self, channel: &str, property: &str, value: bool) -> Result<()> {
        let val = if value { "true" } else { "false" };
        self.xfconf_set_typed(channel, property, val, "bool").await
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

        // An explicitly configured theme name is passed through verbatim —
        // some theme families ship dark variants as separate "Foo-dark"
        // directories with no in-theme variant, and stripping the suffix
        // would silently render them light. Only the built-in Adwaita
        // default relies on ApplicationPreferDarkTheme for its variant.
        let base_theme = profile.gtk_theme.as_deref().unwrap_or("Adwaita");

        self.xfconf_set("xsettings", "/Net/ThemeName", base_theme).await?;

        // Set icon theme
        let icon_theme = if let Some(ref explicit) = profile.gtk_icon_theme {
            explicit.clone()
        } else {
            let current = tokio::process::Command::new("xfconf-query")
                .args(["-c", "xsettings", "-p", "/Net/IconThemeName"])
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

        self.xfconf_set("xsettings", "/Net/IconThemeName", &icon_theme).await?;

        if let Some(ref cursor_theme) = profile.cursor_theme {
            self.xfconf_set("xsettings", "/Gtk/CursorThemeName", cursor_theme).await?;
            // 0 means "not configured" — never push a zero-sized cursor
            if profile.cursor_size > 0 {
                self.xfconf_set_typed("xsettings", "/Gtk/CursorThemeSize", &profile.cursor_size.to_string(), "int").await?;
            }
        }

        // Set ApplicationPreferDarkTheme so xfce's GTK module applies the correct
        // dark-theme preference to Thunar and other XFCE-managed GTK3 apps.
        let prefer_dark = matches!(mode, ThemeMode::Dark);
        self.xfconf_set_bool("xsettings", "/Gtk/ApplicationPreferDarkTheme", prefer_dark).await?;

        tracing::info!("[xfconf] Set ThemeName={}, IconThemeName={}, PreferDark={}", base_theme, icon_theme, prefer_dark);

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
