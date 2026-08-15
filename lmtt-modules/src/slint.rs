use crate::ThemeModule;
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};

crate::register_module!(SlintModule);

/// Writes Material You tokens through lmtt-core. Always-on: no binary gate.
pub struct SlintModule;

impl Default for SlintModule {
    fn default() -> Self {
        Self::new()
    }
}

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
        let path = lmtt_core::tokens::write_current(scheme)?;
        tracing::info!("[slint] Updated tokens at {}", path.display());
        Ok(())
    }
}
