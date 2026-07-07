use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};
use std::path::PathBuf;

crate::register_module!(NvimModule);

pub struct NvimModule;

impl Default for NvimModule {
    fn default() -> Self {
        Self::new()
    }
}

impl NvimModule {
    pub fn new() -> Self {
        Self
    }
}

/// Find sockets of running Neovim instances.
/// Modern Neovim (>= 0.8) listens at $XDG_RUNTIME_DIR/nvim.<pid>.0;
/// older releases used /tmp/nvim<user>/<random>/0-style paths.
async fn find_nvim_sockets() -> Vec<PathBuf> {
    let mut sockets = Vec::new();

    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        roots.push(PathBuf::from(runtime_dir));
    }
    roots.push(std::env::temp_dir());

    for root in roots {
        let Ok(mut entries) = tokio::fs::read_dir(&root).await else { continue };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if !name.starts_with("nvim") {
                continue;
            }
            if path.is_dir() {
                // Legacy layout: /tmp/nvimXXXX/0 or /tmp/nvim.<user>/<rand>/nvim.<pid>.0
                let direct = path.join("0");
                if direct.exists() {
                    sockets.push(direct);
                    continue;
                }
                let Ok(mut inner) = tokio::fs::read_dir(&path).await else { continue };
                while let Ok(Some(sub)) = inner.next_entry().await {
                    let sub_path = sub.path();
                    if sub_path.is_dir() {
                        let Ok(mut leaf) = tokio::fs::read_dir(&sub_path).await else { continue };
                        while let Ok(Some(f)) = leaf.next_entry().await {
                            let p = f.path();
                            if p.file_name().and_then(|n| n.to_str())
                                .map(|n| n.starts_with("nvim.") && n.ends_with(".0"))
                                .unwrap_or(false)
                            {
                                sockets.push(p);
                            }
                        }
                    }
                }
            } else if name.starts_with("nvim.") && name.ends_with(".0") {
                // Modern layout: $XDG_RUNTIME_DIR/nvim.<pid>.0
                sockets.push(path);
            }
        }
    }

    sockets
}

#[async_trait]
impl ThemeModule for NvimModule {
    fn name(&self) -> &'static str {
        "nvim"
    }

    fn binary_name(&self) -> &'static str {
        "nvim"
    }

    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let is_light = scheme.mode == ThemeMode::Light;

        let profile = if is_light {
            &config.theme_profiles.light
        } else {
            &config.theme_profiles.dark
        };

        let mode = if is_light { "light" } else { "dark" };

        // The colorscheme name is interpolated into a Vimscript string sent to
        // every running nvim — reject anything that isn't a plain scheme name
        // so a value with a quote can't break the expression or inject code.
        let colorscheme = profile
            .neovim_colorscheme
            .as_deref()
            .filter(|cs| !cs.is_empty() && cs.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_')));
        if profile.neovim_colorscheme.as_deref().is_some_and(|cs| !cs.is_empty()) && colorscheme.is_none() {
            tracing::warn!(
                "[Nvim] Ignoring unsafe neovim_colorscheme {:?} (allowed: letters, digits, - and _)",
                profile.neovim_colorscheme
            );
        }

        // --remote-expr evaluates VIMSCRIPT: a global Lua function is reached
        // via v:lua, not _G (which is Lua syntax and always errors with E121).
        let expr = if let Some(cs) = colorscheme {
            format!("v:lua.set_nvim_theme('{}', '{}')", mode, cs)
        } else {
            format!("v:lua.set_nvim_theme('{}')", mode)
        };

        let mut updated = 0;
        let mut failed = 0;

        for socket in find_nvim_sockets().await {
            // Bound each call: a socket that accepts but never replies (a
            // stale or foreign /tmp socket on a shared host) would otherwise
            // block until the registry timeout. kill_on_drop reaps the client
            // when the per-socket timeout fires instead of leaking it.
            let mut cmd = tokio::process::Command::new("nvim");
            cmd.arg("--server").arg(&socket).arg("--remote-expr").arg(&expr).kill_on_drop(true);
            let result = tokio::time::timeout(std::time::Duration::from_secs(2), cmd.output()).await;

            // Command::output() is Ok even when nvim exits non-zero — only a
            // successful exit means the expression actually ran.
            match result {
                Ok(Ok(output)) if output.status.success() => updated += 1,
                Ok(Ok(output)) => {
                    failed += 1;
                    tracing::debug!(
                        "[Nvim] {} rejected theme expr: {}",
                        socket.display(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
                Ok(Err(e)) => {
                    failed += 1;
                    tracing::debug!("[Nvim] Failed to reach {}: {}", socket.display(), e);
                }
                Err(_) => {
                    failed += 1;
                    tracing::debug!("[Nvim] Timed out on {}", socket.display());
                }
            }
        }

        if updated > 0 {
            tracing::info!("[Nvim] Updated {} instance(s)", updated);
        }
        if failed > 0 {
            tracing::warn!(
                "[Nvim] {} instance(s) not updated — is a global set_nvim_theme(mode, colorscheme?) Lua function defined?",
                failed
            );
        }
        if updated == 0 && failed == 0 {
            tracing::debug!("[Nvim] No running instances found");
        }

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}
