use std::path::{Path, PathBuf};

use crate::fallback::fallback_colors;
use crate::types::{ColorScheme, ThemeMode, SCHEMA_VERSION};
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
    let scheme: ColorScheme = serde_json::from_str(&text)?;
    if scheme.version != SCHEMA_VERSION {
        return Err(Error::Config(format!(
            "unsupported tokens version {} (expected {SCHEMA_VERSION})",
            scheme.version
        )));
    }
    Ok(scheme)
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
    let mut scheme = ColorScheme::new(mode);
    scheme.colors = fallback_colors(mode);
    scheme
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

    fn write_temp_tokens(name: &str, json: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "lmtt-tokens-{}-{}-{name}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, json).unwrap();
        path
    }

    #[test]
    fn load_file_defaults_missing_version_to_one() {
        let path = write_temp_tokens(
            "default",
            r##"{"mode":"dark","colors":{"primary":"#123456"}}"##,
        );
        let scheme = load_file(&path);
        let _ = std::fs::remove_file(&path);
        let scheme = scheme.unwrap();
        assert_eq!(scheme.version, SCHEMA_VERSION);
        assert_eq!(scheme.mode, ThemeMode::Dark);
        assert_eq!(scheme.colors.get("primary").unwrap(), "#123456");
    }

    #[test]
    fn load_file_rejects_unsupported_version() {
        let path = write_temp_tokens("unsupported", r#"{"version":2,"mode":"dark","colors":{}}"#);
        let err = load_file(&path);
        let _ = std::fs::remove_file(&path);
        let err = err.unwrap_err().to_string();
        assert!(err.contains("unsupported tokens version 2"));
    }
}
