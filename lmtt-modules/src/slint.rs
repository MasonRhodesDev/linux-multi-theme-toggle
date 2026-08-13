use crate::ThemeModule;
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(SlintModule);

/// Writes Material You tokens for Slint apps (`slint-kit` ThemeBridge).
/// Always-on: no binary gate — any Slint consumer can read the file.
pub struct SlintModule;

impl SlintModule {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ThemeModule for SlintModule {
    fn name(&self) -> &'static str {
        "slint"
    }

    fn binary_name(&self) -> &'static str {
        "slint-kit"
    }

    fn is_installed(&self) -> bool {
        true
    }

    fn priority(&self) -> u8 {
        40 // After platform portals; before app modules
    }

    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        let path = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("matugen")
            .join("lmtt-slint.json");

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(scheme).map_err(|e| {
            lmtt_core::Error::Module(format!("Failed to serialize slint theme JSON: {e}"))
        })?;
        tokio::fs::write(&path, json).await?;

        tracing::info!("[slint] Updated tokens at {}", path.display());
        Ok(())
    }
}
