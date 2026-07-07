use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode, find_icon_theme_variant};

crate::register_module!(GtkModule);

pub struct GtkModule;

impl Default for GtkModule {
    fn default() -> Self {
        Self::new()
    }
}

impl GtkModule {
    pub fn new() -> Self {
        Self
    }
}

/// Run a `gsettings set` and surface failures — a missing schema on a
/// non-GNOME setup must show up as a module failure, not silent success.
/// No-op writes are skipped: every org.gnome.desktop.interface change fans
/// out as an XDG portal SettingChanged signal that GTK/Electron apps and
/// wezterm react to (wezterm re-evaluates its config per window per signal),
/// so unchanged values must not emit signals at all.
async fn gsettings_set(key: &str, value: &str) -> Result<()> {
    if let Ok(output) = tokio::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", key])
        .output()
        .await
    {
        if output.status.success() {
            let current = String::from_utf8_lossy(&output.stdout);
            if current.trim().trim_matches('\'') == value {
                return Ok(());
            }
        }
    }

    let output = tokio::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", key, value])
        .output()
        .await
        .map_err(|e| lmtt_core::Error::Module(format!("gsettings failed to run: {}", e)))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(lmtt_core::Error::Module(format!(
            "gsettings set {} failed: {}", key, stderr.trim()
        )));
    }
    Ok(())
}

/// Update `key = value` entries in one INI section, preserving every other
/// line (user keys, comments, other sections). Creates the section if
/// missing; appends missing keys at the end of the section.
fn merge_ini(content: &str, section: &str, updates: &[(&str, String)]) -> String {
    let header = format!("[{}]", section);
    let mut lines: Vec<String> = Vec::new();
    // Keys still to write, and keys already written (to collapse duplicates).
    let mut pending: Vec<(&str, &String)> = updates.iter().map(|(k, v)| (*k, v)).collect();
    let mut written: Vec<&str> = Vec::new();
    let mut in_section = false;
    let mut section_seen = false;

    let flush_pending = |lines: &mut Vec<String>, pending: &mut Vec<(&str, &String)>| {
        for (key, value) in pending.drain(..) {
            lines.push(format!("{} = {}", key, value));
        }
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_section {
                flush_pending(&mut lines, &mut pending);
                in_section = false;
            }
            if trimmed == header {
                in_section = true;
                section_seen = true;
            }
            lines.push(line.to_string());
            continue;
        }
        if in_section {
            if let Some((key, _)) = trimmed.split_once('=') {
                let key = key.trim();
                if let Some(pos) = pending.iter().position(|(k, _)| *k == key) {
                    // First occurrence of an update key: rewrite it.
                    let (k, v) = pending.remove(pos);
                    written.push(k);
                    lines.push(format!("{} = {}", k, v));
                    continue;
                }
                if written.contains(&key) {
                    // Later duplicate of an already-rewritten key: drop it, or
                    // GKeyFile's last-occurrence-wins keeps the stale value.
                    continue;
                }
            }
        }
        lines.push(line.to_string());
    }

    if in_section {
        flush_pending(&mut lines, &mut pending);
    }
    if !section_seen {
        if !lines.is_empty() && !lines.last().map(|l| l.is_empty()).unwrap_or(true) {
            lines.push(String::new());
        }
        lines.push(header);
        flush_pending(&mut lines, &mut pending);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
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

    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let mode = scheme.mode;
        let profile = match mode {
            ThemeMode::Light => &config.theme_profiles.light,
            ThemeMode::Dark => &config.theme_profiles.dark,
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

        gsettings_set("gtk-theme", theme).await?;

        // Set icon theme: explicit config > auto-detect variant > fallback to Adwaita
        let icon_theme = if let Some(ref explicit) = profile.gtk_icon_theme {
            explicit.clone()
        } else {
            let current = tokio::process::Command::new("gsettings")
                .args(["get", "org.gnome.desktop.interface", "icon-theme"])
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

        gsettings_set("icon-theme", &icon_theme).await?;

        if let Some(ref cursor_theme) = profile.cursor_theme {
            gsettings_set("cursor-theme", cursor_theme).await?;
            // cursor_size 0 means "not configured" — writing 0 to gsettings
            // and settings.ini gives apps a zero-sized cursor request
            if profile.cursor_size > 0 {
                gsettings_set("cursor-size", &profile.cursor_size.to_string()).await?;
            }
        }

        // color-scheme MUST be last — this is the signal Electron apps use via
        // the portal's SettingChanged. Setting it after all other gsettings
        // changes prevents re-evaluation races.
        gsettings_set("color-scheme", preference).await?;

        // Update GTK settings.ini files for apps that read these directly
        // (e.g., XFCE apps without xsettingsd). These are picked up on next
        // window open. Merge into the existing file — users keep font,
        // hinting, and any other keys they've configured.
        let prefer_dark = matches!(mode, ThemeMode::Dark);
        let home = dirs::home_dir()
            .ok_or(lmtt_core::Error::Config("No home dir".into()))?;

        let mut gtk3_updates = vec![
            ("gtk-theme-name", theme.to_string()),
            ("gtk-icon-theme-name", icon_theme.clone()),
            ("gtk-application-prefer-dark-theme", prefer_dark.to_string()),
        ];
        let mut gtk4_settings_updates = vec![
            ("gtk-theme-name", theme.to_string()),
            ("gtk-icon-theme-name", icon_theme.clone()),
        ];
        if let Some(ref cursor_theme) = profile.cursor_theme {
            gtk3_updates.push(("gtk-cursor-theme-name", cursor_theme.clone()));
            gtk4_settings_updates.push(("gtk-cursor-theme-name", cursor_theme.clone()));
            if profile.cursor_size > 0 {
                gtk3_updates.push(("gtk-cursor-theme-size", profile.cursor_size.to_string()));
                gtk4_settings_updates.push(("gtk-cursor-theme-size", profile.cursor_size.to_string()));
            }
        }

        let gtk3_path = home.join(".config/gtk-3.0/settings.ini");
        tokio::fs::create_dir_all(gtk3_path.parent().unwrap()).await?;
        let gtk3_current = tokio::fs::read_to_string(&gtk3_path).await.unwrap_or_default();
        let gtk3_content = merge_ini(&gtk3_current, "Settings", &gtk3_updates);
        lmtt_core::fsutil::write_atomic(&gtk3_path, gtk3_content).await?;

        let gtk4_path = home.join(".config/gtk-4.0/settings.ini");
        tokio::fs::create_dir_all(gtk4_path.parent().unwrap()).await?;
        let gtk4_current = tokio::fs::read_to_string(&gtk4_path).await.unwrap_or_default();
        let adw_scheme = if prefer_dark { "ADW_COLOR_SCHEME_PREFER_DARK" } else { "ADW_COLOR_SCHEME_PREFER_LIGHT" };
        let gtk4_content = merge_ini(&gtk4_current, "Settings", &gtk4_settings_updates);
        let gtk4_content = merge_ini(&gtk4_content, "AdwStyleManager", &[("color-scheme", adw_scheme.to_string())]);
        lmtt_core::fsutil::write_atomic(&gtk4_path, gtk4_content).await?;

        tracing::info!("[GTK] Set gtk-theme to {}, icon-theme to {}, color-scheme to {}",
            theme, icon_theme, preference);

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        // GTK doesn't need config file injection - it uses gsettings
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::merge_ini;

    #[test]
    fn preserves_user_keys_and_comments() {
        let existing = "# my settings\n[Settings]\ngtk-font-name = Sans 11\ngtk-theme-name = Old\n\n[Other]\nkey = 1\n";
        let merged = merge_ini(existing, "Settings", &[
            ("gtk-theme-name", "New".to_string()),
            ("gtk-icon-theme-name", "Icons".to_string()),
        ]);
        assert!(merged.contains("# my settings"));
        assert!(merged.contains("gtk-font-name = Sans 11"));
        assert!(merged.contains("gtk-theme-name = New"));
        assert!(merged.contains("gtk-icon-theme-name = Icons"));
        assert!(merged.contains("[Other]"));
        assert!(!merged.contains("Old"));
    }

    #[test]
    fn creates_missing_section() {
        let merged = merge_ini("", "Settings", &[("gtk-theme-name", "T".to_string())]);
        assert!(merged.contains("[Settings]"));
        assert!(merged.contains("gtk-theme-name = T"));
    }

    #[test]
    fn appends_missing_key_within_section() {
        let existing = "[Settings]\ngtk-font-name = Sans 11\n\n[Zzz]\na = b\n";
        let merged = merge_ini(existing, "Settings", &[("gtk-theme-name", "T".to_string())]);
        let settings_idx = merged.find("[Settings]").unwrap();
        let zzz_idx = merged.find("[Zzz]").unwrap();
        let theme_idx = merged.find("gtk-theme-name = T").unwrap();
        assert!(theme_idx > settings_idx && theme_idx < zzz_idx);
    }

    #[test]
    fn collapses_duplicate_key_so_stale_last_one_cannot_win() {
        // GKeyFile takes the LAST occurrence; a stale later duplicate must be
        // dropped, not left to override the rewritten first one.
        let existing = "[Settings]\ngtk-theme-name = Adwaita\ngtk-font-name = Sans 11\ngtk-theme-name = Materia-dark\n";
        let merged = merge_ini(existing, "Settings", &[("gtk-theme-name", "New".to_string())]);
        assert_eq!(merged.matches("gtk-theme-name").count(), 1, "exactly one occurrence: {merged}");
        assert!(merged.contains("gtk-theme-name = New"));
        assert!(!merged.contains("Materia-dark"), "stale duplicate removed: {merged}");
        assert!(merged.contains("gtk-font-name = Sans 11"));
    }
}
