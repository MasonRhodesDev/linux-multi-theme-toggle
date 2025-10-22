use crate::ThemeModule;
use lmtt_core::{ColorScheme, Config, Result};
use std::sync::Arc;
use std::time::Instant;

pub struct ModuleRegistry {
    pub modules: Vec<Arc<dyn ThemeModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        let mut modules: Vec<Arc<dyn ThemeModule>> = vec![
            // Platform modules (low priority - run first)
            Arc::new(crate::gtk::GtkModule::new()),      // priority: 10
            Arc::new(crate::xdg::XdgModule::new()),      // priority: 15
            
            // Application modules (priority 100+)
            Arc::new(crate::hyprland::HyprlandModule::new()),
            Arc::new(crate::waybar::WaybarModule::new()),
            Arc::new(crate::wofi::WofiModule::new()),
            Arc::new(crate::tmux::TmuxModule::new()),
        ];
        
        // Sort by priority
        modules.sort_by_key(|m| m.priority());
        
        Self { modules }
    }
    
    /// Apply theme to all enabled and installed modules in parallel
    pub async fn apply_all(&self, scheme: &ColorScheme, config: &Config) -> Vec<ModuleResult> {
        use tokio::task::JoinSet;
        
        let mut tasks = JoinSet::new();
        
        for module in &self.modules {
            // Skip if not enabled or not installed
            if !module.is_enabled(config) {
                continue;
            }
            
            let module = Arc::clone(module);
            let scheme = scheme.clone();
            let config = config.clone();
            
            tasks.spawn(async move {
                let name = module.name();
                let start = Instant::now();
                
                let result = module.apply(&scheme, &config).await;
                let duration_ms = start.elapsed().as_millis() as u64;
                
                ModuleResult {
                    name: name.to_string(),
                    duration_ms,
                    result,
                }
            });
        }
        
        // Wait for all tasks
        let mut results = Vec::new();
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
