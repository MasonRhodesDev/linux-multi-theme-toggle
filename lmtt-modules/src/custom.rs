use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CustomModuleDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub binary: String,
    #[serde(default = "default_priority")]
    pub priority: u8,
    
    #[serde(flatten)]
    pub module_type: CustomModuleType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CustomModuleType {
    Declarative {
        output: OutputConfig,
        template: TemplateConfig,
        #[serde(default)]
        reload: Option<ReloadConfig>,
        #[serde(default)]
        setup: Option<SetupConfig>,
    },
    Script {
        script: ScriptConfig,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputConfig {
    pub path: String,
    #[serde(default = "default_format")]
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateConfig {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReloadConfig {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SetupConfig {
    pub config_file: String,
    pub include_line: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptConfig {
    pub path: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub pass_as_env: bool,
}

fn default_priority() -> u8 { 100 }
fn default_format() -> String { "conf".to_string() }
fn default_timeout() -> u64 { 10000 }

pub struct CustomModule {
    definition: CustomModuleDefinition,
}

impl CustomModule {
    pub fn new(definition: CustomModuleDefinition) -> Self {
        Self { definition }
    }
    
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let definition: CustomModuleDefinition = toml::from_str(&content)
            .map_err(|e| lmtt_core::Error::Config(format!("Invalid module file {}: {}", path.display(), e)))?;
        Ok(Self::new(definition))
    }
}

#[async_trait]
impl ThemeModule for CustomModule {
    fn name(&self) -> &'static str {
        Box::leak(self.definition.name.clone().into_boxed_str())
    }
    
    fn binary_name(&self) -> &'static str {
        Box::leak(self.definition.binary.clone().into_boxed_str())
    }
    
    fn priority(&self) -> u8 {
        self.definition.priority
    }
    
    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        match &self.definition.module_type {
            CustomModuleType::Declarative { output, template, reload, .. } => {
                self.apply_declarative(scheme, output, template, reload.as_ref()).await
            }
            CustomModuleType::Script { script } => {
                self.apply_script(scheme, script).await
            }
        }
    }
    
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        match &self.definition.module_type {
            CustomModuleType::Declarative { setup: Some(setup), .. } => {
                let path = PathBuf::from(expand_tilde(&setup.config_file));
                if !path.exists() {
                    return Ok(vec![]);
                }
                
                let content = tokio::fs::read_to_string(&path).await?;
                let already_included = content.contains(&setup.include_line);
                
                Ok(vec![ConfigFileInfo {
                    path,
                    include_line: setup.include_line.clone(),
                    description: if setup.description.is_empty() {
                        format!("Include LMTT colors for {}", self.definition.name)
                    } else {
                        setup.description.clone()
                    },
                    already_included,
                }])
            }
            _ => Ok(vec![]),
        }
    }
}

impl CustomModule {
    async fn apply_declarative(
        &self,
        scheme: &ColorScheme,
        output: &OutputConfig,
        template: &TemplateConfig,
        reload: Option<&ReloadConfig>,
    ) -> Result<()> {
        let output_path = PathBuf::from(expand_tilde(&output.path));
        
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let handlebars = handlebars::Handlebars::new();
        let mut data = HashMap::new();
        
        data.insert("mode", scheme.mode.to_string());
        for (key, value) in scheme.colors.iter() {
            data.insert(key.as_str(), value.clone());
        }
        
        let rendered = handlebars.render_template(&template.content, &data)
            .map_err(|e| lmtt_core::Error::Module(format!("Template error: {}", e)))?;
        
        tokio::fs::write(&output_path, rendered).await?;
        
        tracing::info!("[{}] Updated colors at {}", self.definition.name, output_path.display());
        
        if let Some(reload_cfg) = reload {
            let output = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&reload_cfg.command)
                .output()
                .await;
            
            if let Ok(output) = output {
                if !output.status.success() {
                    tracing::warn!("[{}] Reload command failed", self.definition.name);
                }
            }
        }
        
        Ok(())
    }
    
    async fn apply_script(&self, scheme: &ColorScheme, script: &ScriptConfig) -> Result<()> {
        let script_path = expand_tilde(&script.path);
        
        if !PathBuf::from(&script_path).exists() {
            return Err(lmtt_core::Error::Module(format!("Script not found: {}", script_path)));
        }
        
        let mode = scheme.mode.to_string();
        
        let mut cmd = tokio::process::Command::new(&script_path);
        cmd.arg(&mode);
        
        if script.pass_as_env {
            for (key, value) in &scheme.colors {
                let env_key = format!("LMTT_{}", key.to_uppercase());
                cmd.env(env_key, value);
            }
            cmd.env("LMTT_MODE", &mode);
        } else {
            let colors_json = serde_json::to_string(&scheme.colors)
                .map_err(|e| lmtt_core::Error::Module(format!("JSON error: {}", e)))?;
            
            let temp_file = std::env::temp_dir().join(format!("lmtt-{}.json", self.definition.name));
            tokio::fs::write(&temp_file, colors_json).await?;
            cmd.arg(temp_file.to_str().unwrap());
        }
        
        let output = tokio::time::timeout(
            std::time::Duration::from_millis(script.timeout),
            cmd.output()
        ).await
            .map_err(|_| lmtt_core::Error::Module("Script timeout".into()))??;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(lmtt_core::Error::Module(format!("Script failed: {}", stderr)));
        }
        
        tracing::info!("[{}] Script executed successfully", self.definition.name);
        
        Ok(())
    }
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.display().to_string(), 1);
        }
    }
    path.to_string()
}

pub fn load_custom_modules() -> Result<Vec<CustomModule>> {
    let config_dir = dirs::config_dir()
        .ok_or(lmtt_core::Error::Config("No config dir".into()))?;
    
    let modules_dir = config_dir.join("lmtt").join("modules");
    
    if !modules_dir.exists() {
        return Ok(vec![]);
    }
    
    let mut modules = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&modules_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match CustomModule::from_file(&path) {
                    Ok(module) => {
                        tracing::debug!("Loaded custom module: {}", module.definition.name);
                        modules.push(module);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load module {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    
    Ok(modules)
}
