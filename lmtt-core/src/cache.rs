use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use crate::{Config, Result};

pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Create cache from config (cache.dir is already tilde-expanded by Config::load)
    pub fn from_config(config: &Config) -> Result<Self> {
        Self::new(PathBuf::from(&config.cache.dir))
    }
    
    /// Calculate SHA256 hash of wallpaper file
    pub async fn wallpaper_hash(&self, wallpaper_path: &Path) -> Result<String> {
        let contents = tokio::fs::read(wallpaper_path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }
    
    /// Check if wallpaper has changed since last run
    pub async fn wallpaper_changed(&self, wallpaper_path: &Path) -> Result<bool> {
        let current_hash = self.wallpaper_hash(wallpaper_path).await?;
        let cache_file = self.cache_dir.join("wallpaper.hash");
        
        if !cache_file.exists() {
            return Ok(true);
        }
        
        let cached_hash = tokio::fs::read_to_string(&cache_file).await?;
        Ok(current_hash != cached_hash.trim())
    }
    
    /// Update wallpaper hash cache
    pub async fn update_wallpaper_cache(&self, wallpaper_path: &Path) -> Result<()> {
        let hash = self.wallpaper_hash(wallpaper_path).await?;
        let cache_file = self.cache_dir.join("wallpaper.hash");
        tokio::fs::write(&cache_file, hash).await?;
        Ok(())
    }
    
    /// Get cached theme state
    pub async fn get_theme_state(&self) -> Result<String> {
        let state_file = self.cache_dir.join("theme_state");
        if state_file.exists() {
            Ok(tokio::fs::read_to_string(&state_file).await?.trim().to_string())
        } else {
            Ok("dark".to_string())
        }
    }
    
    /// Set theme state
    pub async fn set_theme_state(&self, mode: &str) -> Result<()> {
        let state_file = self.cache_dir.join("theme_state");
        tokio::fs::write(&state_file, mode).await?;
        Ok(())
    }

    /// Get cached colors for a wallpaper/mode/scheme combination
    pub async fn get_cached_colors(
        &self,
        wallpaper_path: &Path,
        mode: &str,
        scheme_type: &str,
    ) -> Result<Option<HashMap<String, String>>> {
        let hash = self.wallpaper_hash(wallpaper_path).await?;
        let hash_prefix = &hash[..16];
        let cache_file = self.cache_dir.join(format!("colors_{}_{}_{}.json", hash_prefix, mode, scheme_type));

        if !cache_file.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&cache_file).await?;
        let colors: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| crate::Error::Config(format!("Failed to parse cached colors: {}", e)))?;
        Ok(Some(colors))
    }

    /// Cache colors for a wallpaper/mode/scheme combination
    pub async fn set_cached_colors(
        &self,
        wallpaper_path: &Path,
        mode: &str,
        scheme_type: &str,
        colors: &HashMap<String, String>,
    ) -> Result<()> {
        let hash = self.wallpaper_hash(wallpaper_path).await?;
        let hash_prefix = &hash[..16];
        let cache_file = self.cache_dir.join(format!("colors_{}_{}_{}.json", hash_prefix, mode, scheme_type));

        let json = serde_json::to_string(colors)
            .map_err(|e| crate::Error::Config(format!("Failed to serialize colors: {}", e)))?;
        tokio::fs::write(&cache_file, json).await?;
        Ok(())
    }
}
