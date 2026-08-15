use std::path::{Path, PathBuf};

use crate::fallback::fallback_colors;
use crate::types::{ColorScheme, ThemeMode};
use crate::{Error, Result};

const TOKENS_FILE: &str = "tokens.json";
const LEGACY_SLINT_JSON: &str = "matugen/lmtt-slint.json";

pub fn user_tokens_path() -> Result<PathBuf> {
    Ok(crate::paths::user_data_dir()?.join(TOKENS_FILE))
}

pub fn system_tokens_path() -> PathBuf {
    PathBuf::from(crate::paths::SYSTEM_ROOT).join(TOKENS_FILE)
}

pub fn packaged_tokens_path() -> PathBuf {
    PathBuf::from(crate::paths::PACKAGED_ROOT).join(TOKENS_FILE)
}

pub fn published_tokens_path(user: &str) -> Result<PathBuf> {
    let profile = appearance_profiles::published_profile_path(user)
        .map_err(|e| Error::Config(e.to_string()))?;
    let parent = profile
        .parent()
        .ok_or_else(|| Error::Config("published profile has no parent".into()))?;
    Ok(parent.join(TOKENS_FILE))
}

pub fn write_current(scheme: &ColorScheme) -> Result<PathBuf> {
    let path = user_tokens_path()?;
    write_scheme(&path, scheme)?;
    Ok(path)
}

pub fn load_file(path: &Path) -> Result<ColorScheme> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// User tokens, migrating a legacy matugen JSON once if needed.
pub fn load_current() -> Result<ColorScheme> {
    let path = user_tokens_path()?;
    if !path.exists() {
        migrate_legacy_slint_json(&path)?;
    }
    load_file(&path)
}

pub fn load_published(user: &str) -> Result<ColorScheme> {
    load_file(&published_tokens_path(user)?)
}

/// User → system → packaged → embedded fallback for `mode`.
pub fn load_preferring(mode: ThemeMode) -> ColorScheme {
    if let Ok(scheme) = load_current() {
        return scheme;
    }
    load_system(mode)
}

/// System → packaged → embedded. Does not read the user tree.
pub fn load_system(mode: ThemeMode) -> ColorScheme {
    for path in [system_tokens_path(), packaged_tokens_path()] {
        if let Ok(scheme) = load_file(&path) {
            return scheme;
        }
    }
    ColorScheme {
        mode,
        colors: fallback_colors(mode),
    }
}

fn write_scheme(path: &Path, scheme: &ColorScheme) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(scheme)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn migrate_legacy_slint_json(dest: &Path) -> Result<()> {
    let legacy = crate::paths::user_dirs()?.config_home().join(LEGACY_SLINT_JSON);
    if !legacy.is_file() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(&legacy, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_and_packaged_paths_are_absolute() {
        assert_eq!(system_tokens_path(), PathBuf::from("/etc/lmtt/tokens.json"));
        assert_eq!(
            packaged_tokens_path(),
            PathBuf::from("/usr/share/lmtt/tokens.json")
        );
    }

    #[test]
    fn published_tokens_sit_beside_the_profile() {
        let path = published_tokens_path("mason").unwrap();
        assert_eq!(
            path,
            PathBuf::from("/var/lib/appearance-profiles/users/mason/tokens.json")
        );
    }

    #[test]
    fn published_user_is_rejected_when_unsafe() {
        assert!(published_tokens_path("../root").is_err());
    }
}
