use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use crate::Result;

pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
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
}
