use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use sha2::{Sha256, Digest};
use crate::{Config, Result, ThemeMode};

pub struct Cache {
    cache_dir: PathBuf,
    // Memoized wallpaper hashes: a cache-miss switch hashes the wallpaper in
    // get_cached_colors and again in set_cached_colors — for a multi-MB image
    // that's two full reads on the slow path the cache exists to avoid.
    hash_memo: Mutex<HashMap<PathBuf, String>>,
}

impl Cache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir, hash_memo: Mutex::new(HashMap::new()) })
    }

    /// Create cache from config (cache.dir is already tilde-expanded by Config::load)
    pub fn from_config(config: &Config) -> Result<Self> {
        Self::new(PathBuf::from(&config.cache.dir))
    }

    /// Calculate SHA256 hash of wallpaper file (memoized per process run).
    pub async fn wallpaper_hash(&self, wallpaper_path: &Path) -> Result<String> {
        if let Ok(memo) = self.hash_memo.lock() {
            if let Some(hash) = memo.get(wallpaper_path) {
                return Ok(hash.clone());
            }
        }
        let contents = tokio::fs::read(wallpaper_path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = format!("{:x}", hasher.finalize());
        if let Ok(mut memo) = self.hash_memo.lock() {
            memo.insert(wallpaper_path.to_path_buf(), hash.clone());
        }
        Ok(hash)
    }

    /// Get cached theme state, falling back to `default_mode` when the state
    /// file is missing or holds something unparseable (e.g. a torn write).
    pub async fn get_theme_state(&self, default_mode: ThemeMode) -> Result<ThemeMode> {
        let state_file = self.cache_dir.join("theme_state");
        match tokio::fs::read_to_string(&state_file).await {
            Ok(content) => Ok(content.trim().parse().unwrap_or(default_mode)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(default_mode),
            Err(e) => Err(e.into()),
        }
    }

    /// Set theme state
    pub async fn set_theme_state(&self, mode: ThemeMode) -> Result<()> {
        let state_file = self.cache_dir.join("theme_state");
        crate::fsutil::write_atomic(&state_file, mode.to_string()).await
    }

    /// Get cached colors for a wallpaper/mode/scheme combination
    pub async fn get_cached_colors(
        &self,
        wallpaper_path: &Path,
        mode: &str,
        scheme_type: &str,
    ) -> Result<Option<HashMap<String, String>>> {
        let cache_file = self.colors_cache_file(wallpaper_path, mode, scheme_type).await?;

        let content = match tokio::fs::read_to_string(&cache_file).await {
            Ok(content) => content,
            // Missing or unreadable cache is a miss, not an error
            Err(_) => return Ok(None),
        };
        match serde_json::from_str(&content) {
            Ok(colors) => Ok(Some(colors)),
            Err(e) => {
                tracing::debug!("Ignoring corrupt color cache {}: {}", cache_file.display(), e);
                Ok(None)
            }
        }
    }

    /// Cache colors for a wallpaper/mode/scheme combination
    pub async fn set_cached_colors(
        &self,
        wallpaper_path: &Path,
        mode: &str,
        scheme_type: &str,
        colors: &HashMap<String, String>,
    ) -> Result<()> {
        let cache_file = self.colors_cache_file(wallpaper_path, mode, scheme_type).await?;
        let json = serde_json::to_string(colors)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize colors: {}", e)))?;
        crate::fsutil::write_atomic(&cache_file, json).await
    }

    async fn colors_cache_file(
        &self,
        wallpaper_path: &Path,
        mode: &str,
        scheme_type: &str,
    ) -> Result<PathBuf> {
        let hash = self.wallpaper_hash(wallpaper_path).await?;
        let hash_prefix = &hash[..16];
        Ok(self.cache_dir.join(format!("colors_{}_{}_{}.json", hash_prefix, mode, scheme_type)))
    }
}
