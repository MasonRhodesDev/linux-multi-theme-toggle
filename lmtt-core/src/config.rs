use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::{Error, Result, ThemeMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    
    #[serde(default)]
    pub notifications: NotificationConfig,
    
    #[serde(default)]
    pub performance: PerformanceConfig,
    
    #[serde(default)]
    pub modules: ModuleConfig,
    
    #[serde(default)]
    pub colors: ColorOverrides,
    
    #[serde(default)]
    pub cache: CacheConfig,
    
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_wallpaper")]
    pub wallpaper: String,
    
    #[serde(default = "default_mode")]
    pub default_mode: ThemeMode,
    
    #[serde(default = "default_scheme_type")]
    pub scheme_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default = "default_notification_timeout")]
    pub timeout: i32,
    
    #[serde(default)]
    pub show_module_progress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    #[serde(default = "default_slow_threshold")]
    pub slow_module_threshold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleConfig {
    #[serde(flatten)]
    pub modules: HashMap<String, ModuleSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSetting {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default)]
    pub restart: bool,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColorOverrides {
    #[serde(flatten)]
    pub colors: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default = "default_cache_dir")]
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    
    #[serde(default = "default_log_file")]
    pub log_file: String,
    
    #[serde(default = "default_max_log_size")]
    pub max_log_size: u64,
}

// Default implementations
impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            wallpaper: default_wallpaper(),
            default_mode: default_mode(),
            scheme_type: default_scheme_type(),
        }
    }
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout: 5000,
            show_module_progress: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            timeout: 10,
            slow_module_threshold: 250,
        }
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dir: default_cache_dir(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_file: default_log_file(),
            max_log_size: 10,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            notifications: NotificationConfig::default(),
            performance: PerformanceConfig::default(),
            modules: ModuleConfig::default(),
            colors: ColorOverrides::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

// Default value functions
fn default_wallpaper() -> String {
    "~/Pictures/forrest.png".to_string()
}

fn default_mode() -> ThemeMode {
    ThemeMode::Dark
}

fn default_scheme_type() -> String {
    "scheme-expressive".to_string()
}

fn default_true() -> bool {
    true
}

fn default_notification_timeout() -> i32 {
    5000
}

fn default_timeout() -> u64 {
    10
}

fn default_slow_threshold() -> u64 {
    250
}

fn default_cache_dir() -> String {
    "~/.cache/lmtt".to_string()
}

fn default_log_file() -> String {
    "~/.cache/lmtt/lmtt.log".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_log_size() -> u64 {
    10
}

impl Config {
    /// Load config from file, falling back to defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&contents)?;
            
            // Expand tilde in paths
            config.general.wallpaper = expand_tilde(&config.general.wallpaper);
            config.cache.dir = expand_tilde(&config.cache.dir);
            config.logging.log_file = expand_tilde(&config.logging.log_file);
            
            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }
    
    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let toml = toml::to_string_pretty(self)
            .map_err(|e| Error::Config(e.to_string()))?;
        
        std::fs::write(&config_path, toml)?;
        Ok(())
    }
    
    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| Error::Config("No config directory found".to_string()))?;
        
        Ok(config_dir.join("lmtt").join("config.toml"))
    }
    
    /// Check if a module is enabled (enabled by default, returns true if not in config)
    pub fn is_module_enabled(&self, module_name: &str) -> bool {
        self.modules.modules
            .get(module_name)
            .map(|m| m.enabled)
            .unwrap_or(true) // Default to enabled if not in config
    }
    
    /// Check if a module should restart
    pub fn should_module_restart(&self, module_name: &str) -> bool {
        self.modules.modules
            .get(module_name)
            .map(|m| m.restart)
            .unwrap_or(false)
    }
    
    /// Get custom command for a module
    pub fn module_command(&self, module_name: &str) -> Option<&str> {
        self.modules.modules
            .get(module_name)
            .and_then(|m| m.command.as_deref())
    }
}

/// Expand ~ to home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

impl Default for ModuleSetting {
    fn default() -> Self {
        Self {
            enabled: true, // Enabled by default
            restart: false,
            command: None,
        }
    }
}
