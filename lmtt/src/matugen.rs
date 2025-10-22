use lmtt_core::{Config, ThemeMode, ColorScheme, Result, Error};
use std::path::Path;

/// Generate color scheme using matugen
pub async fn generate_colors(config: &Config, mode: ThemeMode) -> Result<ColorScheme> {
    let wallpaper = &config.general.wallpaper;
    let scheme_type = &config.general.scheme_type;
    
    // Expand tilde in wallpaper path
    let wallpaper_path = if wallpaper.starts_with("~/") {
        let home = dirs::home_dir()
            .ok_or(Error::Config("No home directory".into()))?;
        wallpaper.replacen("~", &home.display().to_string(), 1)
    } else {
        wallpaper.clone()
    };
    
    // Check if wallpaper exists
    if !Path::new(&wallpaper_path).exists() {
        return Err(Error::Config(format!("Wallpaper not found: {}", wallpaper_path)));
    }
    
    // Run matugen to generate colors
    let output = tokio::process::Command::new("matugen")
        .args(&[
            "--json",
            "hex",
            "--dry-run",
            "image",
            &wallpaper_path,
            "--mode",
            &mode.to_string(),
            "--type",
            scheme_type,
        ])
        .output()
        .await
        .map_err(|e| Error::Matugen(format!("Failed to run matugen: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Matugen(format!("matugen failed: {}", stderr)));
    }
    
    let json = String::from_utf8_lossy(&output.stdout);
    
    // Parse colors from JSON
    let colors = lmtt_core::colors::parse_matugen_colors(&json, &mode.to_string())
        .map_err(|e| Error::Matugen(e))?;
    
    // Create color scheme
    let mut scheme = ColorScheme::new(mode);
    for (key, value) in colors {
        scheme.set(key, value);
    }
    
    // Apply color overrides from config
    for (key, value) in &config.colors.colors {
        scheme.set(key.clone(), value.clone());
    }
    
    Ok(scheme)
}
