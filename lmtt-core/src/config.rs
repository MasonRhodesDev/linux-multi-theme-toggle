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
    
    #[serde(default)]
    pub theme_profiles: ThemeProfiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_wallpaper")]
    pub wallpaper: String,
    
    #[serde(default = "default_mode")]
    pub default_mode: ThemeMode,
    
    #[serde(default = "default_scheme_type")]
    pub scheme_type: String,
    
    #[serde(default = "default_true")]
    pub use_matugen: bool,
    
    #[serde(default = "default_light_colors")]
    pub default_light_colors: String,
    
    #[serde(default = "default_dark_colors")]
    pub default_dark_colors: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeProfiles {
    #[serde(default)]
    pub light: ThemeProfile,
    
    #[serde(default)]
    pub dark: ThemeProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gtk_theme: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gtk_icon_theme: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_theme: Option<String>,
    
    #[serde(default = "default_cursor_size")]
    pub cursor_size: u32,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_font: Option<String>,
    
    #[serde(default = "default_font_size")]
    pub terminal_font_size: u32,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_font: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vscode_theme: Option<String>,
    
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neovim_colorscheme: Option<String>,
    
    #[serde(default = "default_opacity")]
    pub terminal_opacity: f32,
    
    #[serde(default)]
    pub window_blur: bool,
}

// Default implementations
impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            wallpaper: default_wallpaper(),
            default_mode: default_mode(),
            scheme_type: default_scheme_type(),
            use_matugen: true,
            default_light_colors: default_light_colors(),
            default_dark_colors: default_dark_colors(),
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
            theme_profiles: ThemeProfiles::default(),
        }
    }
}

impl Default for ThemeProfiles {
    fn default() -> Self {
        Self {
            light: ThemeProfile::default(),
            dark: ThemeProfile::default(),
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

fn default_light_colors() -> String {
    "~/.config/lmtt/colors-light.json".to_string()
}

fn default_dark_colors() -> String {
    "~/.config/lmtt/colors-dark.json".to_string()
}

fn default_cursor_size() -> u32 {
    24
}

fn default_font_size() -> u32 {
    11
}

fn default_opacity() -> f32 {
    0.95
}

impl Config {
    /// Get description for a config field
    pub fn get_field_description(section: &str, field: &str) -> &'static str {
        match (section, field) {
            // General
            ("general", "wallpaper") => "Path to wallpaper image for theme color generation (supports ~)",
            ("general", "default_mode") => "Default theme mode on startup: Light or Dark",
            ("general", "scheme_type") => "Material color scheme type (scheme-expressive, scheme-tonal-spot, scheme-neutral, etc.)",
            ("general", "use_matugen") => "Enable automatic color generation from wallpaper using matugen",
            ("general", "default_light_colors") => "Path to fallback color JSON for light mode when matugen disabled",
            ("general", "default_dark_colors") => "Path to fallback color JSON for dark mode when matugen disabled",
            
            // Notifications
            ("notifications", "enabled") => "Show desktop notifications when theme changes",
            ("notifications", "timeout") => "Notification display duration in milliseconds (default: 5000)",
            ("notifications", "show_module_progress") => "Show individual notification for each module being applied",
            
            // Performance
            ("performance", "timeout") => "Maximum seconds to wait for all modules to complete (default: 10)",
            ("performance", "slow_module_threshold") => "Log warning if any module takes longer than this in milliseconds (default: 250)",
            
            // Cache
            ("cache", "enabled") => "Cache matugen color generation results to speed up repeated theme switches",
            ("cache", "dir") => "Directory to store cached color schemes (supports ~)",
            
            // Logging
            ("logging", "level") => "Log verbosity level: debug, info, warn, error",
            ("logging", "log_file") => "Path to log file for debugging (supports ~)",
            ("logging", "max_log_size") => "Maximum log file size in megabytes before rotation",
            
            _ => "No description available",
        }
    }
    
    /// Load config from file, falling back to defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&contents)?;
            
            // Expand tilde in paths
            config.general.wallpaper = expand_tilde(&config.general.wallpaper);
            config.general.default_light_colors = expand_tilde(&config.general.default_light_colors);
            config.general.default_dark_colors = expand_tilde(&config.general.default_dark_colors);
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
    
    /// Save config to file with descriptive comments
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut output = String::new();
        
        // Header
        output.push_str("# LMTT (Linux Matugen Theme Toggle) Configuration\n");
        output.push_str("# This file is auto-generated but safe to edit manually\n\n");
        
        // General section
        output.push_str("[general]\n");
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "wallpaper")));
        output.push_str(&format!("wallpaper = \"{}\"\n\n", self.general.wallpaper));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "default_mode")));
        output.push_str(&format!("default_mode = \"{:?}\"\n\n", self.general.default_mode));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "scheme_type")));
        output.push_str(&format!("scheme_type = \"{}\"\n\n", self.general.scheme_type));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "use_matugen")));
        output.push_str(&format!("use_matugen = {}\n\n", self.general.use_matugen));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "default_light_colors")));
        output.push_str(&format!("default_light_colors = \"{}\"\n\n", self.general.default_light_colors));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("general", "default_dark_colors")));
        output.push_str(&format!("default_dark_colors = \"{}\"\n\n", self.general.default_dark_colors));
        
        // Notifications section
        output.push_str("[notifications]\n");
        output.push_str(&format!("# {}\n", Self::get_field_description("notifications", "enabled")));
        output.push_str(&format!("enabled = {}\n\n", self.notifications.enabled));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("notifications", "timeout")));
        output.push_str(&format!("timeout = {}\n\n", self.notifications.timeout));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("notifications", "show_module_progress")));
        output.push_str(&format!("show_module_progress = {}\n\n", self.notifications.show_module_progress));
        
        // Performance section
        output.push_str("[performance]\n");
        output.push_str(&format!("# {}\n", Self::get_field_description("performance", "timeout")));
        output.push_str(&format!("timeout = {}\n\n", self.performance.timeout));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("performance", "slow_module_threshold")));
        output.push_str(&format!("slow_module_threshold = {}\n\n", self.performance.slow_module_threshold));
        
        // Modules section
        output.push_str("[modules]\n");
        if self.modules.modules.is_empty() {
            output.push_str("# Module settings (add entries to customize specific modules)\n");
            output.push_str("# Example:\n");
            output.push_str("# [modules.gtk]\n");
            output.push_str("# enabled = true\n");
            output.push_str("# restart = false\n\n");
        } else {
            for (name, setting) in &self.modules.modules {
                output.push_str(&format!("[modules.{}]\n", name));
                output.push_str(&format!("enabled = {}\n", setting.enabled));
                output.push_str(&format!("restart = {}\n", setting.restart));
                if let Some(cmd) = &setting.command {
                    output.push_str(&format!("command = \"{}\"\n", cmd));
                }
                output.push_str("\n");
            }
        }
        
        // Cache section
        output.push_str("[cache]\n");
        output.push_str(&format!("# {}\n", Self::get_field_description("cache", "enabled")));
        output.push_str(&format!("enabled = {}\n\n", self.cache.enabled));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("cache", "dir")));
        output.push_str(&format!("dir = \"{}\"\n\n", self.cache.dir));
        
        // Logging section
        output.push_str("[logging]\n");
        output.push_str(&format!("# {}\n", Self::get_field_description("logging", "level")));
        output.push_str(&format!("level = \"{}\"\n\n", self.logging.level));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("logging", "log_file")));
        output.push_str(&format!("log_file = \"{}\"\n\n", self.logging.log_file));
        
        output.push_str(&format!("# {}\n", Self::get_field_description("logging", "max_log_size")));
        output.push_str(&format!("max_log_size = {}\n\n", self.logging.max_log_size));
        
        // Theme profiles section
        output.push_str("[theme_profiles.light]\n");
        output.push_str("# Theme profile settings for light mode\n");
        if let Some(gtk) = &self.theme_profiles.light.gtk_theme {
            output.push_str(&format!("gtk_theme = \"{}\"\n", gtk));
        }
        if let Some(icon) = &self.theme_profiles.light.gtk_icon_theme {
            output.push_str(&format!("gtk_icon_theme = \"{}\"\n", icon));
        }
        if let Some(cursor) = &self.theme_profiles.light.cursor_theme {
            output.push_str(&format!("cursor_theme = \"{}\"\n", cursor));
        }
        output.push_str(&format!("cursor_size = {}\n", self.theme_profiles.light.cursor_size));
        if let Some(font) = &self.theme_profiles.light.terminal_font {
            output.push_str(&format!("terminal_font = \"{}\"\n", font));
        }
        output.push_str(&format!("terminal_font_size = {}\n", self.theme_profiles.light.terminal_font_size));
        if let Some(sys_font) = &self.theme_profiles.light.system_font {
            output.push_str(&format!("system_font = \"{}\"\n", sys_font));
        }
        if let Some(vscode) = &self.theme_profiles.light.vscode_theme {
            output.push_str(&format!("vscode_theme = \"{}\"\n", vscode));
        }
        if let Some(nvim) = &self.theme_profiles.light.neovim_colorscheme {
            output.push_str(&format!("neovim_colorscheme = \"{}\"\n", nvim));
        }
        output.push_str(&format!("terminal_opacity = {}\n", self.theme_profiles.light.terminal_opacity));
        output.push_str(&format!("window_blur = {}\n\n", self.theme_profiles.light.window_blur));
        
        output.push_str("[theme_profiles.dark]\n");
        output.push_str("# Theme profile settings for dark mode\n");
        if let Some(gtk) = &self.theme_profiles.dark.gtk_theme {
            output.push_str(&format!("gtk_theme = \"{}\"\n", gtk));
        }
        if let Some(icon) = &self.theme_profiles.dark.gtk_icon_theme {
            output.push_str(&format!("gtk_icon_theme = \"{}\"\n", icon));
        }
        if let Some(cursor) = &self.theme_profiles.dark.cursor_theme {
            output.push_str(&format!("cursor_theme = \"{}\"\n", cursor));
        }
        output.push_str(&format!("cursor_size = {}\n", self.theme_profiles.dark.cursor_size));
        if let Some(font) = &self.theme_profiles.dark.terminal_font {
            output.push_str(&format!("terminal_font = \"{}\"\n", font));
        }
        output.push_str(&format!("terminal_font_size = {}\n", self.theme_profiles.dark.terminal_font_size));
        if let Some(sys_font) = &self.theme_profiles.dark.system_font {
            output.push_str(&format!("system_font = \"{}\"\n", sys_font));
        }
        if let Some(vscode) = &self.theme_profiles.dark.vscode_theme {
            output.push_str(&format!("vscode_theme = \"{}\"\n", vscode));
        }
        if let Some(nvim) = &self.theme_profiles.dark.neovim_colorscheme {
            output.push_str(&format!("neovim_colorscheme = \"{}\"\n", nvim));
        }
        output.push_str(&format!("terminal_opacity = {}\n", self.theme_profiles.dark.terminal_opacity));
        output.push_str(&format!("window_blur = {}\n", self.theme_profiles.dark.window_blur));
        
        // Color overrides section
        if !self.colors.colors.is_empty() {
            output.push_str("\n[colors]\n");
            output.push_str("# Custom color overrides\n");
            for (key, value) in &self.colors.colors {
                output.push_str(&format!("{} = \"{}\"\n", key, value));
            }
        }
        
        std::fs::write(&config_path, output)?;
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

/// Expand ~ to home directory and environment variables
fn expand_tilde(path: &str) -> String {
    let mut expanded = path.to_string();
    
    // Expand tilde
    if expanded.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            expanded = expanded.replacen("~", &home.display().to_string(), 1);
        }
    }
    
    // Expand environment variables like $HOME, ${HOME}, $USER, etc.
    expand_env_vars(&expanded)
}

/// Expand environment variables in a string
fn expand_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    
    // Handle ${VAR} syntax
    while let Some(start) = result.find("${") {
        if let Some(end) = result[start..].find('}') {
            let var_name = &result[start + 2..start + end];
            if let Ok(value) = std::env::var(var_name) {
                result.replace_range(start..start + end + 1, &value);
            } else {
                // If variable doesn't exist, leave it as is
                break;
            }
        } else {
            break;
        }
    }
    
    // Handle $VAR syntax (without braces)
    let mut idx = 0;
    while idx < result.len() {
        if result[idx..].starts_with('$') {
            let var_start = idx + 1;
            let var_end = result[var_start..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|i| var_start + i)
                .unwrap_or(result.len());
            
            if var_end > var_start {
                let var_name = &result[var_start..var_end];
                if let Ok(value) = std::env::var(var_name) {
                    result.replace_range(idx..var_end, &value);
                    idx += value.len();
                    continue;
                }
            }
        }
        idx += 1;
    }
    
    result
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
