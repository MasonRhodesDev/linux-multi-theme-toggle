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
    /// Binary whose presence gates the module. Optional: reload-only modules
    /// (and templates for files no binary owns) don't need one.
    #[serde(default)]
    pub binary: Option<String>,
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
    /// A module that only runs a reload command — no file output. Used to
    /// poke apps that pick colors up from a shared file written elsewhere.
    ReloadOnly {
        reload: ReloadConfig,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutputConfig {
    pub path: String,
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
fn default_timeout() -> u64 { 10000 }

pub struct CustomModule {
    definition: CustomModuleDefinition,
    // ThemeModule::name()/binary_name() return &'static str; leak exactly
    // once at construction instead of on every call.
    name: &'static str,
    binary_name: &'static str,
}

impl CustomModule {
    pub fn new(definition: CustomModuleDefinition) -> Self {
        let name: &'static str = Box::leak(definition.name.clone().into_boxed_str());
        let binary_name: &'static str = match &definition.binary {
            Some(binary) => Box::leak(binary.clone().into_boxed_str()),
            None => "",
        };
        Self { definition, name, binary_name }
    }

    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        // Validate the raw shape BEFORE the untagged enum resolves. Untagged
        // deserialization silently picks whichever variant happens to match,
        // so a typo'd [ouput]/[templete] table degrades a Declarative module
        // to ReloadOnly with no error — the color file is never written yet
        // apply() reports success. Catch structural mistakes here instead.
        let table: toml::Table = toml::from_str(&content)
            .map_err(|e| lmtt_core::Error::Config(format!("Invalid module file {}: {}", path.display(), e)))?;
        validate_shape(&table)
            .map_err(|e| lmtt_core::Error::Config(format!("Invalid module file {}: {}", path.display(), e)))?;

        let definition: CustomModuleDefinition = table.try_into()
            .map_err(|e| lmtt_core::Error::Config(format!("Invalid module file {}: {}", path.display(), e)))?;
        Ok(Self::new(definition))
    }
}

/// Reject unknown top-level keys (usually typos) and incomplete variants, so
/// the untagged enum can't silently resolve to the wrong module type.
fn validate_shape(table: &toml::Table) -> std::result::Result<(), String> {
    const KNOWN: &[&str] = &[
        "name", "description", "binary", "priority",
        "output", "template", "reload", "setup", "script",
    ];
    for key in table.keys() {
        if !KNOWN.contains(&key.as_str()) {
            return Err(format!(
                "unknown key '{}' (expected one of: output, template, reload, setup, script, name, description, binary, priority)",
                key
            ));
        }
    }

    let has = |k: &str| table.contains_key(k);
    if has("script") {
        if has("output") || has("template") {
            return Err("a [script] module must not also define output/template".into());
        }
    } else if has("output") || has("template") {
        if !(has("output") && has("template")) {
            return Err("a declarative module requires BOTH [output] and [template]".into());
        }
    } else if !has("reload") {
        return Err("module defines no [output]+[template], [script], or [reload]".into());
    }
    Ok(())
}

#[async_trait]
impl ThemeModule for CustomModule {
    fn name(&self) -> &'static str {
        self.name
    }

    fn binary_name(&self) -> &'static str {
        self.binary_name
    }

    fn is_installed(&self) -> bool {
        // No binary configured means nothing to gate on
        self.binary_name.is_empty() || which::which(self.binary_name).is_ok()
    }

    fn priority(&self) -> u8 {
        self.definition.priority
    }

    fn max_apply_secs(&self) -> Option<u64> {
        // Report this module's own configured timeout(s) so the registry
        // watchdog doesn't cap a legitimately long script/reload.
        let ms = match &self.definition.module_type {
            CustomModuleType::Script { script } => script.timeout,
            CustomModuleType::Declarative { reload, .. } => reload.as_ref().map(|r| r.timeout).unwrap_or(0),
            CustomModuleType::ReloadOnly { reload } => reload.timeout,
        };
        // 0 means "no timeout" (clamp_timeout maps it to 1h) — report that.
        Some(if ms == 0 { 3600 } else { ms.div_ceil(1000) })
    }

    async fn apply(&self, scheme: &ColorScheme, _config: &Config) -> Result<()> {
        match &self.definition.module_type {
            CustomModuleType::Declarative { output, template, reload, .. } => {
                self.apply_declarative(scheme, output, template, reload.as_ref()).await
            }
            CustomModuleType::Script { script } => {
                self.apply_script(scheme, script).await
            }
            CustomModuleType::ReloadOnly { reload } => {
                self.run_reload(reload).await
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
                let already_included = crate::is_included(&content, &setup.include_line);

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

        let mut handlebars = handlebars::Handlebars::new();
        // Values are written to config files, not HTML — never entity-escape
        // them — and a template typo should fail loudly, not render "".
        handlebars.register_escape_fn(handlebars::no_escape);
        handlebars.set_strict_mode(true);

        let mut data = HashMap::new();
        for (key, value) in scheme.colors.iter() {
            data.insert(key.as_str(), value.clone());
        }
        // Insert after the colors so "mode" always wins
        data.insert("mode", scheme.mode.to_string());

        let rendered = handlebars.render_template(&template.content, &data)
            .map_err(|e| lmtt_core::Error::Module(format!("Template error: {}", e)))?;

        lmtt_core::fsutil::write_atomic(&output_path, rendered).await?;

        tracing::info!("[{}] Updated colors at {}", self.definition.name, output_path.display());

        if let Some(reload_cfg) = reload {
            self.run_reload(reload_cfg).await?;
        }

        Ok(())
    }

    async fn run_reload(&self, reload: &ReloadConfig) -> Result<()> {
        // kill_on_drop: when the timeout fires and this future is dropped, the
        // child sh (and its process group) is killed instead of orphaned.
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(&reload.command).kill_on_drop(true);
        let output = tokio::time::timeout(clamp_timeout(reload.timeout), cmd.output())
            .await
            .map_err(|_| lmtt_core::Error::Module(format!(
                "[{}] Reload command timed out after {}ms", self.definition.name, reload.timeout
            )))?
            .map_err(|e| lmtt_core::Error::Module(format!(
                "[{}] Failed to run reload command: {}", self.definition.name, e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(
                "[{}] Reload command failed ({}): {}",
                self.definition.name,
                output.status,
                stderr.trim()
            );
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
        // Kill the script (not orphan it) if it outruns its timeout, so the
        // temp colors file isn't unlinked from under a still-running reader.
        cmd.arg(&mode).kill_on_drop(true);

        // Keep any temp file alive until the script has finished
        let _colors_file: Option<tempfile::NamedTempFile>;

        if script.pass_as_env {
            for (key, value) in &scheme.colors {
                let env_key = format!("LMTT_{}", key.to_uppercase());
                cmd.env(env_key, value);
            }
            cmd.env("LMTT_MODE", &mode);
            _colors_file = None;
        } else {
            let colors_json = serde_json::to_string(&scheme.colors)
                .map_err(|e| lmtt_core::Error::Module(format!("JSON error: {}", e)))?;

            // Unpredictable per-run temp file: a fixed /tmp/lmtt-<name>.json
            // is a symlink-attack target and races concurrent lmtt runs.
            let file = tempfile::Builder::new()
                .prefix("lmtt-colors-")
                .suffix(".json")
                .tempfile()
                .map_err(|e| lmtt_core::Error::Module(format!("Temp file error: {}", e)))?;
            tokio::fs::write(file.path(), colors_json).await?;
            cmd.arg(file.path());
            _colors_file = Some(file);
        }

        let output = tokio::time::timeout(clamp_timeout(script.timeout), cmd.output())
            .await
            .map_err(|_| lmtt_core::Error::Module("Script timeout".into()))??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(lmtt_core::Error::Module(format!("Script failed: {}", stderr)));
        }

        tracing::info!("[{}] Script executed successfully", self.definition.name);

        Ok(())
    }
}

/// Timeout of 0 means "no timeout" (a very large duration), not "fire
/// immediately" — otherwise timeout=0 would kill every command instantly.
fn clamp_timeout(ms: u64) -> std::time::Duration {
    if ms == 0 {
        std::time::Duration::from_secs(3600)
    } else {
        std::time::Duration::from_millis(ms)
    }
}

/// Expand ~, $VAR, and ${VAR} — shares lmtt-core's implementation so custom
/// module paths behave the same as paths in the main config.
fn expand_tilde(path: &str) -> String {
    lmtt_core::config::expand_path(path)
}

/// Load custom modules from ~/.config/lmtt/modules/*.toml. Parse failures are
/// returned so callers can surface them — a module that silently fails to
/// load looks exactly like a module that ran.
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
                        // Print to stderr as well: a broken module definition
                        // must be visible in normal CLI output, not just logs
                        eprintln!("✗ [custom module] {}", e);
                        tracing::warn!("Failed to load module {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(modules)
}
