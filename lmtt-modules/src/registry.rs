use crate::{ThemeModule, ModuleConstructor};
use lmtt_core::{ColorScheme, Config, Result};
use std::sync::Arc;
use std::time::Instant;

pub struct ModuleRegistry {
    pub modules: Vec<Arc<dyn ThemeModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut modules: Vec<Arc<dyn ThemeModule>> = Vec::new();
        
        // Auto-discover built-in modules using inventory
        for constructor in inventory::iter::<ModuleConstructor> {
            modules.push((constructor.constructor)());
        }
        
        // Load custom modules from ~/.config/lmtt/modules/
        match crate::custom::load_custom_modules() {
            Ok(custom_modules) => {
                for module in custom_modules {
                    modules.push(Arc::new(module));
                }
            }
            Err(e) => {
                tracing::warn!("Failed to load custom modules: {}", e);
            }
        }
        
        // Sort by priority (platform modules first, then apps)
        modules.sort_by_key(|m| m.priority());
        
        Self { modules }
    }
    
    /// Apply theme to all enabled modules in two phases:
    /// - Phase 1 (sequential): Platform modules with priority < 50, run in order
    /// - Phase 2 (parallel): App modules with priority >= 50, spawned concurrently
    pub async fn apply_all(&self, scheme: &ColorScheme, config: &Config) -> Vec<ModuleResult> {
        use tokio::task::JoinSet;

        let mut results = Vec::new();

        let (platform, app): (Vec<_>, Vec<_>) = self
            .modules
            .iter()
            .filter(|m| m.is_enabled(config))
            .partition(|m| m.priority() < 50);

        // Phase 1: sequential platform modules (already sorted by priority)
        for module in &platform {
            let name = module.name();
            let start = Instant::now();
            let result = module.apply(scheme, config).await;
            let duration_ms = start.elapsed().as_millis() as u64;
            tracing::debug!("[Registry] Phase 1 (sequential): {} completed in {}ms", name, duration_ms);
            results.push(ModuleResult {
                name: name.to_string(),
                duration_ms,
                result,
            });
        }

        // Phase 2: parallel app modules
        let mut tasks = JoinSet::new();
        for module in &app {
            let module = Arc::clone(module);
            let scheme = scheme.clone();
            let config = config.clone();

            tasks.spawn(async move {
                let name = module.name();
                let start = Instant::now();
                let result = module.apply(&scheme, &config).await;
                let duration_ms = start.elapsed().as_millis() as u64;
                tracing::debug!("[Registry] Phase 2 (parallel): {} completed in {}ms", name, duration_ms);
                ModuleResult {
                    name: name.to_string(),
                    duration_ms,
                    result,
                }
            });
        }

        while let Some(result) = tasks.join_next().await {
            if let Ok(module_result) = result {
                results.push(module_result);
            }
        }

        results
    }
    
    /// Get list of all enabled module names
    pub fn enabled_modules(&self, config: &Config) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|m| m.is_enabled(config))
            .map(|m| m.name())
            .collect()
    }
    
    /// Get list of all installed module names
    pub fn installed_modules(&self) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|m| m.is_installed())
            .map(|m| m.name())
            .collect()
    }
}

#[derive(Debug)]
pub struct ModuleResult {
    pub name: String,
    pub duration_ms: u64,
    pub result: Result<()>,
}

impl ModuleResult {
    pub fn is_success(&self) -> bool {
        self.result.is_ok()
    }
    
    pub fn is_slow(&self, threshold_ms: u64) -> bool {
        self.duration_ms > threshold_ms
    }
}
