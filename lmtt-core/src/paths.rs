use std::path::PathBuf;

use crate::{Error, Result};

pub const SYSTEM_ROOT: &str = "/etc/lmtt";
pub const PACKAGED_ROOT: &str = "/usr/share/lmtt";

pub fn user_dirs() -> Result<hypr_paths::ConfigDirs> {
    hypr_paths::ConfigDirs::from_env().map_err(|e| Error::Config(e.to_string()))
}

pub fn user_config_dir() -> Result<PathBuf> {
    Ok(user_dirs()?.config_dir("lmtt"))
}

pub fn user_data_dir() -> Result<PathBuf> {
    Ok(user_dirs()?.data_dir("lmtt"))
}

pub fn user_cache_dir() -> Result<PathBuf> {
    hypr_paths::cache_dir("lmtt").map_err(|e| Error::Config(e.to_string()))
}

/// User, then system, then packaged. First filename wins at the caller.
pub fn module_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(user) = user_config_dir() {
        dirs.push(user.join("modules"));
    }
    dirs.push(PathBuf::from(SYSTEM_ROOT).join("modules"));
    dirs.push(PathBuf::from(PACKAGED_ROOT).join("modules"));
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_module_roots_are_absolute() {
        let dirs = module_search_dirs();
        assert!(dirs.iter().any(|p| p == &PathBuf::from("/etc/lmtt/modules")));
        assert!(dirs
            .iter()
            .any(|p| p == &PathBuf::from("/usr/share/lmtt/modules")));
        for dir in &dirs {
            assert!(dir.is_absolute(), "{}", dir.display());
        }
    }
}
