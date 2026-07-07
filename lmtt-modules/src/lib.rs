pub mod registry;
pub mod setup;
pub mod cleanup;
pub mod custom;
pub mod gtk;
pub mod xdg;
pub mod hyprland;
pub mod waybar;
pub mod wofi;
pub mod fuzzel;
pub mod tmux;
pub mod swaync;
pub mod wezterm;
pub mod vscode;
pub mod nvim;
pub mod fish;
pub mod qt;
pub mod xfconf;
pub mod hyprpanel;
pub mod hyprlock;
pub mod regreet;

use async_trait::async_trait;
use lmtt_core::{ColorScheme, Result, Config};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Marker text identifying lmtt managed blocks. The comment syntax around it
/// varies by file type (see `comment_style`), so detection matches on the
/// bare marker text, never on a specific comment style.
pub const MARKER_START: &str = ">>> lmtt managed block - do not edit manually >>>";
pub const MARKER_END: &str = "<<< lmtt managed block <<<";

/// Comment prefix/suffix for marker lines, chosen by file extension.
/// `#` is not a comment in CSS or Lua — injecting it corrupts the file.
/// Returns None for file types with NO line-comment syntax (strict JSON),
/// where any injected marker would corrupt the file.
fn comment_style(path: &Path) -> Option<(&'static str, &'static str)> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("css") | Some("scss") => Some(("/* ", " */")),
        Some("lua") => Some(("-- ", "")),
        Some("json") => None,
        Some("jsonc") | Some("json5") => Some(("// ", "")),
        _ => Some(("# ", "")), // conf/ini/toml/yaml/fish/tmux/hypr/…
    }
}

/// True if a line is a comment in any of the syntaxes lmtt injects into, so a
/// commented-out include line doesn't count as active configuration.
fn is_comment_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('#') || t.starts_with("//") || t.starts_with("/*") || t.starts_with("--") || t.starts_with(';')
}

/// Whether the config already has THIS module's integration active — the
/// include line appears inside an lmtt managed block, or on a non-comment
/// line the user added manually. Crucially this is keyed on the include line,
/// NOT merely on the presence of any marker: a stale block from an older lmtt
/// version (different include line) reads as not-included so the current line
/// gets injected (migration), and two modules sharing a file don't mistake
/// each other's blocks for their own.
pub fn is_included(content: &str, include_line: &str) -> bool {
    let mut in_block = false;
    for line in content.lines() {
        if line.contains(MARKER_START) {
            in_block = true;
            continue;
        }
        if line.contains(MARKER_END) {
            in_block = false;
            continue;
        }
        if in_block {
            if line.contains(include_line) {
                return true;
            }
        } else if !is_comment_line(line) && line.contains(include_line) {
            return true;
        }
    }
    false
}

/// Remove lmtt managed blocks OWNED by the given include lines (a block is
/// owned if its body contains any of them), plus standalone active copies of
/// those exact lines. Other modules' blocks and all user content are
/// preserved, including per-line endings (CRLF stays CRLF). Errors on an
/// unterminated managed block rather than guessing its extent.
fn strip_owned_blocks(content: &str, owned: &[&str]) -> Result<String> {
    let owns = |s: &str| owned.iter().any(|o| s.contains(o));

    // Split on '\n' only, so any '\r' stays on each segment and round-trips
    // exactly when we re-join with '\n' (CRLF files keep their CRLF).
    let segments: Vec<&str> = content.split('\n').collect();
    let mut out: Vec<&str> = Vec::with_capacity(segments.len());

    let mut i = 0;
    while i < segments.len() {
        let line = segments[i];
        if line.contains(MARKER_START) {
            let start = i;
            let mut j = i + 1;
            let mut body_owned = false;
            while j < segments.len() && !segments[j].contains(MARKER_END) {
                if owns(segments[j]) {
                    body_owned = true;
                }
                j += 1;
            }
            if j >= segments.len() {
                return Err(lmtt_core::Error::Module(
                    "Unterminated lmtt managed block — closing marker missing, not modifying".into(),
                ));
            }
            if body_owned {
                // Drop the whole block; also swallow one trailing blank line
                // (inject_config adds one after the block).
                i = j + 1;
                if i < segments.len() && segments[i].trim().is_empty() {
                    i += 1;
                }
                continue;
            }
            // Not ours — keep the block verbatim
            for seg in &segments[start..=j] {
                out.push(seg);
            }
            i = j + 1;
            continue;
        }
        // Standalone active include line (not in a block, not a comment):
        // only drop when the trimmed line IS the include line, so a user line
        // that merely mentions it isn't nuked.
        if !is_comment_line(line) && owned.iter().any(|o| line.trim() == o.trim()) {
            i += 1;
            continue;
        }
        out.push(line);
        i += 1;
    }

    Ok(out.join("\n"))
}

/// Information about a config file that needs lmtt integration
#[derive(Debug, Clone)]
pub struct ConfigFileInfo {
    /// Path to the config file
    pub path: PathBuf,

    /// The import/include line that should be added
    pub include_line: String,

    /// Description of what this does
    pub description: String,

    /// Whether the include is already present
    pub already_included: bool,
}

/// Standard trait that all theme modules must implement
#[async_trait]
pub trait ThemeModule: Send + Sync {
    /// Module name (e.g., "Waybar", "Hyprland")
    fn name(&self) -> &'static str;
    
    /// Binary name to check for installation (e.g., "waybar", "hyprctl")
    fn binary_name(&self) -> &'static str;
    
    /// Check if the application is installed on the system
    fn is_installed(&self) -> bool {
        which::which(self.binary_name()).is_ok()
    }
    
    /// Apply theme (non-blocking, returns immediately)
    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()>;
    
    /// Get config file(s) that need lmtt integration (for setup mode)
    /// Returns None if this module doesn't need config injection
    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
    
    /// Inject include line into config file, wrapped in marker comments that
    /// use the file type's actual comment syntax.
    async fn inject_config(&self, config_file: &ConfigFileInfo) -> Result<()> {
        let path = &config_file.path;

        if !path.exists() {
            return Err(lmtt_core::Error::Module(
                format!("Config file not found: {}", path.display())
            ));
        }

        let content = tokio::fs::read_to_string(path).await?;

        if is_included(&content, &config_file.include_line) {
            return Ok(());
        }

        let Some((prefix, suffix)) = comment_style(path) else {
            return Err(lmtt_core::Error::Module(format!(
                "Refusing to inject into strict JSON ({}) — it has no comment syntax; use JSONC or configure it manually",
                path.display()
            )));
        };

        // Migration: strip this module's own stale blocks/lines (old include
        // lines) before adding the current one, so an upgrade doesn't leave a
        // block sourcing a file this version no longer writes.
        let legacy = self.legacy_include_lines();
        let mut owned: Vec<&str> = vec![config_file.include_line.as_str()];
        owned.extend(legacy.iter().map(|s| s.as_str()));
        let base = strip_owned_blocks(&content, &owned)?;

        let new_content = format!(
            "{prefix}{MARKER_START}{suffix}\n{}\n{prefix}{MARKER_END}{suffix}\n\n{base}",
            config_file.include_line
        );

        lmtt_core::fsutil::write_atomic(path, new_content).await
    }

    /// Remove THIS module's lmtt-injected config (for cleanup): only managed
    /// blocks whose body contains this module's include line (or a legacy
    /// one), plus standalone active copies of the include line. Blocks owned
    /// by other modules sharing the file are left intact. Refuses to touch a
    /// file with an unterminated block. Preserves line endings (incl. CRLF).
    /// Returns Ok(true) if it actually changed the file, Ok(false) if there
    /// was nothing of this module's to remove (so callers don't over-report).
    async fn remove_config(&self, config_file: &ConfigFileInfo) -> Result<bool> {
        let path = &config_file.path;

        if !path.exists() {
            return Ok(false); // Already gone
        }

        let content = tokio::fs::read_to_string(path).await?;
        let mut owned: Vec<&str> = vec![config_file.include_line.as_str()];
        let legacy = self.legacy_include_lines();
        owned.extend(legacy.iter().map(|s| s.as_str()));

        let new_content = strip_owned_blocks(&content, &owned)?;
        if new_content == content {
            return Ok(false); // Nothing of ours to remove
        }
        lmtt_core::fsutil::write_atomic(path, new_content).await?;
        Ok(true)
    }
    
    /// Optional: Module-specific health check
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
    
    /// Optional: Priority (lower = runs first, for dependencies)
    /// Platform modules (GTK, XDG, Qt) should have priority < 50
    /// Application modules should have priority >= 100
    fn priority(&self) -> u8 {
        100
    }

    /// Largest timeout (seconds) this module may legitimately need for one
    /// apply. The registry watchdog uses max(this, performance.timeout) so a
    /// module that self-declares a longer timeout (e.g. a custom script) is
    /// not silently cut short by the global default.
    fn max_apply_secs(&self) -> Option<u64> {
        None
    }

    /// Include lines this module used in a PREVIOUS version. On inject the
    /// blocks/lines matching these are removed first, so an upgrade that
    /// renames the sourced file (e.g. Hyprland colors.conf → lmtt-colors.conf)
    /// migrates cleanly instead of leaving a stale block that sources a dead
    /// file forever.
    fn legacy_include_lines(&self) -> Vec<String> {
        Vec::new()
    }
    
    /// Whether this module is enabled (checks config and installation)
    fn is_enabled(&self, config: &Config) -> bool {
        // Check config (defaults to true)
        if !config.is_module_enabled(self.name()) {
            return false;
        }
        
        // Check if installed
        if !self.is_installed() {
            tracing::debug!("[{}] Not installed, skipping", self.name());
            return false;
        }
        
        true
    }
}

pub use registry::ModuleRegistry;
pub use setup::SetupManager;
pub use cleanup::CleanupManager;

#[cfg(test)]
mod inject_tests {
    use super::*;

    struct DummyModule;

    #[async_trait]
    impl ThemeModule for DummyModule {
        fn name(&self) -> &'static str { "dummy" }
        fn binary_name(&self) -> &'static str { "dummy" }
        async fn apply(&self, _s: &ColorScheme, _c: &Config) -> Result<()> { Ok(()) }
    }

    async fn round_trip(file_name: &str, body: &str, include_line: &str) -> (String, String) {
        let dir = std::env::temp_dir().join(format!("lmtt-inject-test-{}-{}", std::process::id(), file_name));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join(file_name);
        tokio::fs::write(&path, body).await.unwrap();

        let info = ConfigFileInfo {
            path: path.clone(),
            include_line: include_line.to_string(),
            description: String::new(),
            already_included: false,
        };

        let module = DummyModule;
        module.inject_config(&info).await.unwrap();
        let injected = tokio::fs::read_to_string(&path).await.unwrap();

        // Re-inject must be a no-op (marker-based idempotency)
        module.inject_config(&info).await.unwrap();
        let re_injected = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(injected, re_injected, "second inject must not duplicate");

        module.remove_config(&info).await.unwrap();
        let removed = tokio::fs::read_to_string(&path).await.unwrap();

        tokio::fs::remove_dir_all(&dir).await.unwrap();
        (injected, removed)
    }

    #[tokio::test]
    async fn css_uses_block_comments_and_round_trips() {
        let body = "* { color: red; }\n";
        let (injected, removed) = round_trip("style.css", body, "@import url('x.css');").await;
        assert!(injected.starts_with("/* >>> lmtt managed block"), "CSS must get /* */ markers, got: {}", injected);
        assert!(!injected.contains("\n# >>>"), "no #-comments in CSS");
        assert_eq!(removed, body, "cleanup must restore the original file");
    }

    #[tokio::test]
    async fn lua_uses_dash_comments_and_round_trips() {
        let body = "return {}\n";
        let (injected, removed) = round_trip("wezterm.lua", body, "local colors = require('x')").await;
        assert!(injected.starts_with("-- >>> lmtt managed block"), "Lua must get -- markers, got: {}", injected);
        assert_eq!(removed, body);
    }

    #[tokio::test]
    async fn conf_uses_hash_comments_and_round_trips() {
        let body = "monitor = ,preferred,auto,1\n";
        let (injected, removed) = round_trip("hyprland.conf", body, "source = ~/.config/hypr/colors.conf").await;
        assert!(injected.starts_with("# >>> lmtt managed block"));
        assert_eq!(removed, body);
    }

    #[tokio::test]
    async fn unterminated_block_refuses_to_modify() {
        let dir = std::env::temp_dir().join(format!("lmtt-unterminated-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("broken.conf");
        let body = format!("# {}\ninclude me\nuser content\n", MARKER_START);
        tokio::fs::write(&path, &body).await.unwrap();

        let info = ConfigFileInfo {
            path: path.clone(),
            include_line: "include me".to_string(),
            description: String::new(),
            already_included: true,
        };
        let err = DummyModule.remove_config(&info).await;
        assert!(err.is_err(), "must refuse to touch a file missing the closing marker");
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), body, "file must be untouched");
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    struct Owner(&'static str, Vec<String>);
    #[async_trait]
    impl ThemeModule for Owner {
        fn name(&self) -> &'static str { self.0 }
        fn binary_name(&self) -> &'static str { self.0 }
        async fn apply(&self, _s: &ColorScheme, _c: &Config) -> Result<()> { Ok(()) }
        fn legacy_include_lines(&self) -> Vec<String> { self.1.clone() }
    }

    #[test]
    fn is_included_is_keyed_on_the_include_line_not_the_marker() {
        // A block owned by a DIFFERENT include line must not read as included.
        let content = format!(
            "# {MARKER_START}\nsource = ~/.config/hypr/colors.conf\n# {MARKER_END}\n\nmonitor=,pref,auto,1\n"
        );
        assert!(is_included(&content, "source = ~/.config/hypr/colors.conf"));
        assert!(!is_included(&content, "source = ~/.config/hypr/lmtt-colors.conf"),
            "a stale block with a different include line must NOT count as included");
        // Commented-out active line must not count
        assert!(!is_included("# @import url('x.css');\n", "@import url('x.css');"));
    }

    #[tokio::test]
    async fn hyprland_migration_replaces_stale_block() {
        let dir = std::env::temp_dir().join(format!("lmtt-migrate-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("hyprland.conf");
        // Pre-upgrade state: block sourcing the OLD colors.conf
        let body = format!("# {MARKER_START}\nsource = ~/.config/hypr/colors.conf\n# {MARKER_END}\n\nmonitor=,pref,auto,1\n");
        tokio::fs::write(&path, &body).await.unwrap();

        let module = Owner("hyprland", vec!["source = ~/.config/hypr/colors.conf".into()]);
        let info = ConfigFileInfo {
            path: path.clone(),
            include_line: "source = ~/.config/hypr/lmtt-colors.conf".into(),
            description: String::new(),
            already_included: false,
        };
        module.inject_config(&info).await.unwrap();
        let out = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(out.contains("lmtt-colors.conf"), "new source injected: {out}");
        assert!(!out.contains("hypr/colors.conf\n"), "stale old source removed: {out}");
        assert_eq!(out.matches(MARKER_START).count(), 1, "exactly one managed block: {out}");
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn cleanup_leaves_other_modules_blocks() {
        let dir = std::env::temp_dir().join(format!("lmtt-shared-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("shared.conf");
        let body = format!(
            "# {MARKER_START}\nsource = A.conf\n# {MARKER_END}\n\n# {MARKER_START}\nsource = B.conf\n# {MARKER_END}\n\nuser line\n"
        );
        tokio::fs::write(&path, &body).await.unwrap();

        let module_a = Owner("a", vec![]);
        let info_a = ConfigFileInfo {
            path: path.clone(),
            include_line: "source = A.conf".into(),
            description: String::new(),
            already_included: true,
        };
        module_a.remove_config(&info_a).await.unwrap();
        let out = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(!out.contains("source = A.conf"), "A's block removed: {out}");
        assert!(out.contains("source = B.conf"), "B's block preserved: {out}");
        assert!(out.contains("user line"), "user content preserved: {out}");
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn crlf_endings_preserved_on_cleanup() {
        let dir = std::env::temp_dir().join(format!("lmtt-crlf-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("win.css");
        let body = format!("/* {MARKER_START} */\r\n@import url('x.css');\r\n/* {MARKER_END} */\r\n\r\n.a {{ color: red; }}\r\n");
        tokio::fs::write(&path, &body).await.unwrap();

        let module = Owner("w", vec![]);
        let info = ConfigFileInfo {
            path: path.clone(),
            include_line: "@import url('x.css');".into(),
            description: String::new(),
            already_included: true,
        };
        module.remove_config(&info).await.unwrap();
        let out = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(out, ".a { color: red; }\r\n", "CRLF preserved, block removed: {out:?}");
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn strict_json_injection_refused() {
        let dir = std::env::temp_dir().join(format!("lmtt-json-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("config.json");
        tokio::fs::write(&path, "{\n  \"a\": 1\n}\n").await.unwrap();

        let info = ConfigFileInfo {
            path: path.clone(),
            include_line: "whatever".into(),
            description: String::new(),
            already_included: false,
        };
        assert!(DummyModule.inject_config(&info).await.is_err(), "must refuse strict JSON");
        assert_eq!(tokio::fs::read_to_string(&path).await.unwrap(), "{\n  \"a\": 1\n}\n", "file untouched");
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}

/// Constructor function type for module auto-registration
pub struct ModuleConstructor {
    pub constructor: fn() -> Arc<dyn ThemeModule>,
}

inventory::collect!(ModuleConstructor);

/// Macro to auto-register a module
#[macro_export]
macro_rules! register_module {
    ($module:ty) => {
        inventory::submit! {
            $crate::ModuleConstructor {
                constructor: || std::sync::Arc::new(<$module>::new())
            }
        }
    };
}
