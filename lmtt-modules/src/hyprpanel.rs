use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};
use serde_json::{Map, Value};

crate::register_module!(HyprPanelModule);

pub struct HyprPanelModule;

impl Default for HyprPanelModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HyprPanelModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for HyprPanelModule {
    fn name(&self) -> &'static str {
        "hyprpanel"
    }

    fn binary_name(&self) -> &'static str {
        "hyprpanel"
    }

    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        // Only defer to swaync when it's the daemon actually RUNNING — the
        // swaync binary being merely installed (a common dependency) must not
        // leave a running hyprpanel stuck in the wrong theme.
        if process_running("swaync").await {
            tracing::debug!("[HyprPanel] SwayNC is running; skipping hyprpanel theming");
            return Ok(());
        }

        let config_file = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("hyprpanel")
            .join("config.json");

        if !config_file.exists() {
            return Ok(());
        }

        let mode = match scheme.mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        };

        let content = tokio::fs::read_to_string(&config_file).await?;
        // A parse failure must abort: falling back to an empty map and
        // writing it back would replace the user's entire config.
        let mut json: Map<String, Value> = serde_json::from_str(&content).map_err(|e| {
            lmtt_core::Error::Module(format!(
                "Refusing to rewrite unparseable {}: {}",
                config_file.display(),
                e
            ))
        })?;

        json.insert("theme.matugen_settings.mode".to_string(), Value::String(mode.to_string()));

        let new_content = serde_json::to_string_pretty(&json)?;
        lmtt_core::fsutil::write_atomic(&config_file, new_content).await?;

        if process_running("hyprpanel").await {
            // Exact-match kill only: a bare `pkill hyprpanel` pattern-matches
            // any process whose name contains the string.
            let _ = tokio::process::Command::new("pkill")
                .args(["-x", "hyprpanel"])
                .output()
                .await;

            // Wait for the old instance to actually exit (bounded), then
            // respawn detached so it isn't a child of this short-lived process.
            let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
            while process_running("hyprpanel").await && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            tokio::process::Command::new("hyprpanel")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .process_group(0)
                .spawn()
                .map_err(|e| lmtt_core::Error::Module(format!("Failed to restart hyprpanel: {}", e)))?;

            tracing::info!("[HyprPanel] Restarted with {} theme", mode);
        }

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}

async fn process_running(name: &str) -> bool {
    tokio::process::Command::new("pgrep")
        .args(["-x", name])
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}
