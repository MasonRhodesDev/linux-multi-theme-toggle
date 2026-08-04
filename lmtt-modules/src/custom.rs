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

/// Exactly one of `content` (inline template) or `path` (template file) must
/// be set — enforced by validate_shape at load time and again at apply time.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TemplateConfig {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
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
        // The untagged enum can't distinguish "inline template" from "file
        // template" — both are one TemplateConfig — so enforce the
        // exactly-one rule here where it produces a load-time error.
        if let Some(template) = table.get("template").and_then(|t| t.as_table()) {
            match (template.contains_key("content"), template.contains_key("path")) {
                (true, true) => {
                    return Err("[template] must set either 'content' or 'path', not both".into());
                }
                (false, false) => {
                    return Err("[template] must set 'content' (inline) or 'path' (template file)".into());
                }
                _ => {}
            }
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

        // validate_shape enforces the exactly-one rule at load time; recheck
        // here because CustomModule::new can be reached without from_file.
        let template_source = match (&template.content, &template.path) {
            (Some(content), None) => content.clone(),
            (None, Some(path)) => {
                let template_path = PathBuf::from(expand_tilde(path));
                tokio::fs::read_to_string(&template_path).await
                    .map_err(|e| lmtt_core::Error::Module(format!(
                        "Failed to read template file {}: {}", template_path.display(), e
                    )))?
            }
            (Some(_), Some(_)) => {
                return Err(lmtt_core::Error::Module(
                    "[template] must set either 'content' or 'path', not both".into(),
                ));
            }
            (None, None) => {
                return Err(lmtt_core::Error::Module(
                    "[template] must set 'content' (inline) or 'path' (template file)".into(),
                ));
            }
        };

        let rendered = handlebars.render_template(&template_source, &data)
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

/// Load custom modules from the module search path, in precedence order:
/// $XDG_CONFIG_HOME/lmtt/modules, then each dir in $XDG_CONFIG_DIRS
/// (default /etc/xdg) as <dir>/lmtt/modules, then /usr/share/lmtt/modules.
/// Parse failures are returned so callers can surface them — a module that
/// silently fails to load looks exactly like a module that ran.
pub fn load_custom_modules() -> Result<Vec<CustomModule>> {
    let config_dir = dirs::config_dir()
        .ok_or(lmtt_core::Error::Config("No config dir".into()))?;

    let mut search_dirs = vec![config_dir.join("lmtt").join("modules")];
    let xdg_config_dirs = std::env::var("XDG_CONFIG_DIRS")
        .unwrap_or_else(|_| "/etc/xdg".to_string());
    for dir in xdg_config_dirs.split(':').filter(|d| !d.is_empty()) {
        search_dirs.push(PathBuf::from(dir).join("lmtt").join("modules"));
    }
    search_dirs.push(PathBuf::from("/usr/share/lmtt/modules"));

    Ok(load_modules_from_dirs(&search_dirs))
}

/// Load *.toml module definitions (non-recursive) from the given directories.
/// Dedupe by FILENAME, first directory wins — a user file shadows a system
/// file of the same name entirely, even if the definitions differ.
fn load_modules_from_dirs(search_dirs: &[PathBuf]) -> Vec<CustomModule> {
    let mut modules = Vec::new();
    let mut seen_files = std::collections::HashSet::new();

    for modules_dir in search_dirs {
        let Ok(entries) = std::fs::read_dir(modules_dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let Some(file_name) = path.file_name().map(|n| n.to_os_string()) else { continue };
            if !seen_files.insert(file_name) {
                tracing::debug!("Module file {} shadowed by an earlier search dir", path.display());
                continue;
            }
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

    modules
}

#[cfg(test)]
mod tests {
    use super::*;
    use lmtt_core::ThemeMode;

    fn scheme() -> ColorScheme {
        let mut scheme = ColorScheme::new(ThemeMode::Dark);
        scheme.colors.insert("primary".to_string(), "#123456".to_string());
        scheme
    }

    /// Minimal module just to have a &self for apply_declarative — the
    /// output/template under test are passed in explicitly.
    fn test_module() -> CustomModule {
        CustomModule::new(CustomModuleDefinition {
            name: "test".to_string(),
            description: String::new(),
            binary: None,
            priority: 100,
            module_type: CustomModuleType::ReloadOnly {
                reload: ReloadConfig { command: "true".to_string(), timeout: 1000 },
            },
        })
    }

    #[tokio::test]
    async fn template_path_renders_file() {
        let dir = tempfile::tempdir().unwrap();
        let hbs_path = dir.path().join("colors.hbs");
        tokio::fs::write(&hbs_path, "primary={{primary}} mode={{mode}}\n").await.unwrap();
        let out_path = dir.path().join("out.conf");

        let output = OutputConfig { path: out_path.to_string_lossy().into_owned() };
        let template = TemplateConfig {
            content: None,
            path: Some(hbs_path.to_string_lossy().into_owned()),
        };
        test_module().apply_declarative(&scheme(), &output, &template, None).await.unwrap();

        let rendered = tokio::fs::read_to_string(&out_path).await.unwrap();
        assert_eq!(rendered, "primary=#123456 mode=dark\n");
    }

    #[tokio::test]
    async fn template_rejects_both_content_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let output = OutputConfig { path: dir.path().join("out.conf").to_string_lossy().into_owned() };
        let template = TemplateConfig {
            content: Some("{{primary}}".to_string()),
            path: Some("/nonexistent.hbs".to_string()),
        };
        let err = test_module().apply_declarative(&scheme(), &output, &template, None).await.unwrap_err();
        assert!(err.to_string().contains("not both"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn template_rejects_neither_content_nor_path() {
        let dir = tempfile::tempdir().unwrap();
        let output = OutputConfig { path: dir.path().join("out.conf").to_string_lossy().into_owned() };
        let template = TemplateConfig { content: None, path: None };
        let err = test_module().apply_declarative(&scheme(), &output, &template, None).await.unwrap_err();
        assert!(err.to_string().contains("must set 'content'"), "unexpected error: {err}");
    }

    #[test]
    fn validate_shape_accepts_template_path() {
        let table: toml::Table = toml::from_str(
            "name = \"x\"\n[output]\npath = \"~/x\"\n[template]\npath = \"~/t.hbs\"\n",
        ).unwrap();
        assert!(validate_shape(&table).is_ok());
    }

    #[test]
    fn validate_shape_rejects_both_content_and_path() {
        let table: toml::Table = toml::from_str(
            "name = \"x\"\n[output]\npath = \"~/x\"\n[template]\ncontent = \"c\"\npath = \"~/t.hbs\"\n",
        ).unwrap();
        assert!(validate_shape(&table).unwrap_err().contains("not both"));
    }

    #[test]
    fn validate_shape_rejects_empty_template_table() {
        let table: toml::Table = toml::from_str(
            "name = \"x\"\n[output]\npath = \"~/x\"\n[template]\n",
        ).unwrap();
        assert!(validate_shape(&table).unwrap_err().contains("must set 'content'"));
    }

    #[test]
    fn module_dedup_first_dir_wins() {
        let user_dir = tempfile::tempdir().unwrap();
        let system_dir = tempfile::tempdir().unwrap();
        let def = |name: &str| format!("name = \"{name}\"\n\n[reload]\ncommand = \"true\"\n");
        std::fs::write(user_dir.path().join("shared.toml"), def("user-shared")).unwrap();
        std::fs::write(system_dir.path().join("shared.toml"), def("system-shared")).unwrap();
        std::fs::write(system_dir.path().join("only-system.toml"), def("only-system")).unwrap();
        // Non-toml files are ignored
        std::fs::write(system_dir.path().join("notes.txt"), "ignored").unwrap();

        let modules = load_modules_from_dirs(&[
            user_dir.path().to_path_buf(),
            system_dir.path().to_path_buf(),
        ]);
        let names: Vec<&str> = modules.iter().map(|m| m.definition.name.as_str()).collect();
        assert_eq!(names.len(), 2, "loaded: {names:?}");
        assert!(names.contains(&"user-shared"), "user file must shadow system: {names:?}");
        assert!(names.contains(&"only-system"), "unshadowed system file loads: {names:?}");
    }
}
