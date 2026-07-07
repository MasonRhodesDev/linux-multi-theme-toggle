use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(FishModule);

pub struct FishModule;

impl Default for FishModule {
    fn default() -> Self {
        Self::new()
    }
}

impl FishModule {
    pub fn new() -> Self {
        Self
    }
}

/// fish color values are RRGGBB without the leading '#'
fn fish_hex(color: &str) -> String {
    color.trim_start_matches('#').to_string()
}

#[async_trait]
impl ThemeModule for FishModule {
    fn name(&self) -> &'static str {
        "fish"
    }

    fn binary_name(&self) -> &'static str {
        "fish"
    }

    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        // Universal variables (set -U) propagate to every RUNNING fish shell
        // instantly — a conf.d file with set -g only affects new shells, and
        // no signal makes fish re-source conf.d.
        let assignments: Vec<(&str, String)> = vec![
            ("fish_color_normal", fish_hex(&scheme.get_or_fallback("on_surface"))),
            ("fish_color_command", fish_hex(&scheme.get_or_fallback("primary"))),
            ("fish_color_param", fish_hex(&scheme.get_or_fallback("on_surface"))),
            ("fish_color_redirection", fish_hex(&scheme.get_or_fallback("secondary"))),
            ("fish_color_comment", fish_hex(&scheme.get_or_fallback("outline"))),
            ("fish_color_error", fish_hex(&scheme.get_or_fallback("error"))),
            ("fish_color_escape", fish_hex(&scheme.get_or_fallback("tertiary"))),
            ("fish_color_operator", fish_hex(&scheme.get_or_fallback("primary"))),
            ("fish_color_quote", fish_hex(&scheme.get_or_fallback("secondary"))),
            ("fish_color_autosuggestion", fish_hex(&scheme.get_or_fallback("outline"))),
            ("fish_pager_color_completion", fish_hex(&scheme.get_or_fallback("on_surface"))),
            ("fish_pager_color_description", fish_hex(&scheme.get_or_fallback("on_surface_variant"))),
            ("fish_pager_color_prefix", fish_hex(&scheme.get_or_fallback("primary"))),
            ("fish_pager_color_progress", fish_hex(&scheme.get_or_fallback("outline"))),
        ];

        let mut script = String::new();
        for (var, value) in &assignments {
            script.push_str(&format!("set -U {} {}\n", var, value));
        }
        script.push_str(&format!(
            "set -U fish_color_selection --background={}\n",
            fish_hex(&scheme.get_or_fallback("primary_container"))
        ));
        script.push_str(&format!(
            "set -U fish_color_search_match --background={}\n",
            fish_hex(&scheme.get_or_fallback("tertiary_container"))
        ));
        script.push_str(&format!(
            "set -U fish_pager_color_selected_background --background={}\n",
            fish_hex(&scheme.get_or_fallback("surface_container_high"))
        ));

        // NOT --no-config: fish skips the universal variable store entirely
        // in that mode, so set -U would silently not persist.
        let output = tokio::process::Command::new("fish")
            .args(["-c", &script])
            .output()
            .await
            .map_err(|e| lmtt_core::Error::Module(format!("fish failed to run: {}", e)))?;

        if !output.status.success() {
            return Err(lmtt_core::Error::Module(format!(
                "fish color update failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        // Remove the legacy conf.d file from the old set -g approach — its
        // startup-time globals would fight the universal variables.
        let legacy = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("fish")
            .join("conf.d")
            .join("lmtt-colors.fish");
        if legacy.exists() {
            let _ = tokio::fs::remove_file(&legacy).await;
            tracing::info!("[Fish] Removed legacy conf.d/lmtt-colors.fish (colors now universal variables)");
        }

        tracing::info!("[Fish] Updated colors via universal variables (live shells included)");

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
