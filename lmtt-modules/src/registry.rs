use crate::{ThemeModule, ModuleConstructor};
use lmtt_core::{ColorScheme, Config, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ModuleRegistry {
    pub modules: Vec<Arc<dyn ThemeModule>>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut modules: Vec<Arc<dyn ThemeModule>> = Vec::new();

        // Auto-discover built-in modules using inventory
        for constructor in inventory::iter::<ModuleConstructor> {
            modules.push((constructor.constructor)());
        }

        // Load custom modules from ~/.config/lmtt/modules/.
        // A custom module with the same name replaces the built-in one —
        // otherwise both would run on every switch and neither could be
        // disabled independently (they share the [modules.<name>] config key).
        match crate::custom::load_custom_modules() {
            Ok(custom_modules) => {
                for module in custom_modules {
                    let module: Arc<dyn ThemeModule> = Arc::new(module);
                    if let Some(existing) = modules.iter_mut().find(|m| m.name() == module.name()) {
                        tracing::info!(
                            "Custom module '{}' overrides the built-in module of the same name",
                            module.name()
                        );
                        *existing = module;
                    } else {
                        modules.push(module);
                    }
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

    /// Apply theme to all enabled modules:
    /// - Phase 1 (sequential): Platform modules with priority < 50, run in order
    /// - Phase 2: App modules with priority >= 50, run in priority tiers —
    ///   parallel within a tier, tiers in ascending order. This keeps the
    ///   priority contract for write-then-reload module pairs.
    ///
    /// Every module apply is wrapped in `performance.timeout` so one hung
    /// command (e.g. gsettings on a wedged D-Bus) can't stall the switch,
    /// and panicked module tasks are reported as failures instead of
    /// silently vanishing from the summary.
    pub async fn apply_all(&self, scheme: &ColorScheme, config: &Config) -> Vec<ModuleResult> {
        use tokio::task::JoinSet;

        let base_secs = config.performance.timeout.max(1);
        let mut results = Vec::new();

        let (platform, app): (Vec<_>, Vec<_>) = self
            .modules
            .iter()
            .filter(|m| m.is_enabled(config))
            .partition(|m| m.priority() < 50);

        // Phase 1: sequential platform modules (already sorted by priority).
        // Spawn each so a panic becomes a reported failure instead of
        // unwinding through apply_all and aborting the whole switch.
        for module in &platform {
            let module = Arc::clone(module);
            let scheme = scheme.clone();
            let config = config.clone();
            let secs = base_secs.max(module.max_apply_secs().unwrap_or(0));
            let name = module.name().to_string();
            let joined = tokio::spawn(async move {
                run_module(module, &scheme, &config, Duration::from_secs(secs)).await
            })
            .await;
            results.push(joined.unwrap_or_else(|join_err| ModuleResult {
                name,
                duration_ms: 0,
                result: Err(lmtt_core::Error::Module(format!("module task panicked: {}", join_err))),
            }));
        }

        // Phase 2: priority tiers of app modules (app inherits the sort order)
        for tier in app.chunk_by(|a, b| a.priority() == b.priority()) {
            let mut tasks = JoinSet::new();
            let mut names: HashMap<tokio::task::Id, String> = HashMap::new();

            for module in tier {
                let module = Arc::clone(module);
                let scheme = scheme.clone();
                let config = config.clone();
                let name = module.name().to_string();
                let secs = base_secs.max(module.max_apply_secs().unwrap_or(0));

                let handle = tasks.spawn(async move {
                    run_module(module, &scheme, &config, Duration::from_secs(secs)).await
                });
                names.insert(handle.id(), name);
            }

            while let Some(joined) = tasks.join_next_with_id().await {
                match joined {
                    Ok((_id, module_result)) => results.push(module_result),
                    Err(join_err) => {
                        let name = names
                            .get(&join_err.id())
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());
                        results.push(ModuleResult {
                            name,
                            duration_ms: 0,
                            result: Err(lmtt_core::Error::Module(format!(
                                "module task panicked: {}",
                                join_err
                            ))),
                        });
                    }
                }
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

async fn run_module(
    module: Arc<dyn ThemeModule>,
    scheme: &ColorScheme,
    config: &Config,
    timeout: Duration,
) -> ModuleResult {
    let name = module.name().to_string();
    let start = Instant::now();
    let result = match tokio::time::timeout(timeout, module.apply(scheme, config)).await {
        Ok(result) => result,
        Err(_) => Err(lmtt_core::Error::Module(format!(
            "timed out after {}s",
            timeout.as_secs()
        ))),
    };
    let duration_ms = start.elapsed().as_millis() as u64;
    tracing::debug!("[Registry] {} completed in {}ms", name, duration_ms);
    ModuleResult {
        name,
        duration_ms,
        result,
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
