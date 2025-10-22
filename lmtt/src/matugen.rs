use lmtt_core::{Config, ThemeMode, ColorScheme, Result, Error};
use std::path::Path;

/// Generate color scheme using matugen, custom colors, or fallback
pub async fn generate_colors(config: &Config, mode: ThemeMode) -> Result<ColorScheme> {
    let mut scheme = ColorScheme::new(mode);
    
    // Try default colors first
    let default_path = match mode {
        ThemeMode::Light => &config.general.default_light_colors,
        ThemeMode::Dark => &config.general.default_dark_colors,
    };
    
    let default_path_buf = std::path::PathBuf::from(default_path);
    
    if default_path_buf.exists() {
        tracing::info!("Using default color scheme from {}", default_path_buf.display());
        let colors = load_custom_colors(&default_path_buf).await?;
        for (key, value) in colors {
            scheme.set(key, value);
        }
    } else if config.general.use_matugen && which::which("matugen").is_ok() {
        // Try matugen if enabled and available
        tracing::info!("Generating colors with matugen");
        let colors = generate_with_matugen(config, mode).await?;
        for (key, value) in colors {
            scheme.set(key, value);
        }
    } else {
        // Fallback to built-in themes
        tracing::info!("Using built-in fallback theme (matugen not available or disabled)");
        let colors = match mode {
            ThemeMode::Light => lmtt_core::fallback::fallback_light_colors(),
            ThemeMode::Dark => lmtt_core::fallback::fallback_dark_colors(),
        };
        for (key, value) in colors {
            scheme.set(key, value);
        }
    }
    
    // Apply color overrides from config
    for (key, value) in &config.colors.colors {
        scheme.set(key.clone(), value.clone());
    }
    
    Ok(scheme)
}

/// Load custom colors from JSON file
async fn load_custom_colors(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| Error::Config(format!("Failed to read custom colors: {}", e)))?;
    
    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::Config(format!("Failed to parse custom colors JSON: {}", e)))?;
    
    let mut colors = std::collections::HashMap::new();
    
    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            if let Some(color) = value.as_str() {
                colors.insert(key.clone(), color.to_string());
            }
        }
    }
    
    Ok(colors)
}

/// Generate colors using matugen
async fn generate_with_matugen(config: &Config, mode: ThemeMode) -> Result<std::collections::HashMap<String, String>> {
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
        tracing::warn!("Wallpaper not found: {}, falling back to default colors", wallpaper_path);
        return Ok(match mode {
            ThemeMode::Light => lmtt_core::fallback::fallback_light_colors(),
            ThemeMode::Dark => lmtt_core::fallback::fallback_dark_colors(),
        });
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
        tracing::warn!("matugen failed: {}, falling back to default colors", stderr);
        return Ok(match mode {
            ThemeMode::Light => lmtt_core::fallback::fallback_light_colors(),
            ThemeMode::Dark => lmtt_core::fallback::fallback_dark_colors(),
        });
    }
    
    let json = String::from_utf8_lossy(&output.stdout);
    
    // Parse colors from JSON
    lmtt_core::colors::parse_matugen_colors(&json, &mode.to_string())
        .map_err(|e| Error::Matugen(e))
}
