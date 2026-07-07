use lmtt_core::{Config, ThemeMode, ColorScheme, Result, Error};
use lmtt_core::cache::Cache;
use std::path::Path;

/// Generate color scheme using matugen, custom colors, or fallback.
///
/// Resolution order:
/// 1. matugen (when `use_matugen = true` and the binary exists)
/// 2. the configured default_{light,dark}_colors JSON file
/// 3. the built-in fallback palette
///
/// Failures at each step fall through to the next — a broken wallpaper or
/// malformed JSON must never abort the whole switch.
pub async fn generate_colors(config: &Config, mode: ThemeMode, cache: Option<&Cache>) -> Result<ColorScheme> {
    let mut scheme = ColorScheme::new(mode);

    for (key, value) in resolve_colors(config, mode, cache).await {
        insert_color(&mut scheme, key, value);
    }

    // Apply color overrides from config
    for (key, value) in &config.colors.colors {
        insert_color(&mut scheme, key.clone(), value.clone());
    }

    Ok(scheme)
}

/// Insert a color only if it's a valid hex value. Color values reach shells,
/// Lua, and terminal escapes downstream, so a non-hex value (a typo, or a
/// malicious string in a downloaded palette JSON) must be dropped here — the
/// token then resolves via the built-in fallback instead of propagating.
fn insert_color(scheme: &mut ColorScheme, key: String, value: String) {
    if lmtt_core::colors::is_hex_color(&value) {
        scheme.set(key, value);
    } else {
        tracing::warn!("Dropping non-hex color '{}' = {:?} (not a valid hex color)", key, value);
    }
}

async fn resolve_colors(
    config: &Config,
    mode: ThemeMode,
    cache: Option<&Cache>,
) -> std::collections::HashMap<String, String> {
    if config.general.use_matugen && which::which("matugen").is_ok() {
        match generate_with_matugen(config, mode, cache).await {
            Ok(colors) => return colors,
            Err(e) => {
                tracing::warn!("matugen color generation failed: {}, trying default colors", e);
            }
        }
    }

    let default_path = match mode {
        ThemeMode::Light => &config.general.default_light_colors,
        ThemeMode::Dark => &config.general.default_dark_colors,
    };
    let default_path = std::path::PathBuf::from(default_path);

    if default_path.exists() {
        match load_custom_colors(&default_path).await {
            Ok(colors) => {
                tracing::info!("Using default color scheme from {}", default_path.display());
                return colors;
            }
            Err(e) => {
                tracing::warn!("Ignoring default colors {}: {}", default_path.display(), e);
            }
        }
    }

    tracing::info!("Using built-in fallback theme");
    fallback_colors(mode)
}

/// Load custom colors from JSON file
async fn load_custom_colors(path: &Path) -> Result<std::collections::HashMap<String, String>> {
    let content = tokio::fs::read_to_string(path).await
        .map_err(|e| Error::Config(format!("Failed to read custom colors: {}", e)))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| Error::Config(format!("Failed to parse custom colors JSON: {}", e)))?;

    let obj = json.as_object()
        .ok_or_else(|| Error::Config("Custom colors JSON must be an object of name -> hex".into()))?;

    let mut colors = std::collections::HashMap::new();
    for (key, value) in obj {
        if let Some(color) = value.as_str() {
            colors.insert(key.clone(), color.to_string());
        }
    }

    if colors.is_empty() {
        return Err(Error::Config("Custom colors JSON contains no color entries".into()));
    }

    Ok(colors)
}

fn fallback_colors(mode: ThemeMode) -> std::collections::HashMap<String, String> {
    match mode {
        ThemeMode::Light => lmtt_core::fallback::fallback_light_colors(),
        ThemeMode::Dark => lmtt_core::fallback::fallback_dark_colors(),
    }
}

/// Generate colors using matugen
async fn generate_with_matugen(config: &Config, mode: ThemeMode, cache: Option<&Cache>) -> Result<std::collections::HashMap<String, String>> {
    let wallpaper = &config.general.wallpaper;
    let scheme_type = &config.general.scheme_type;
    let mode_str = mode.to_string();
    let wallpaper_path = Path::new(wallpaper);

    if !wallpaper_path.exists() {
        return Err(Error::Matugen(format!("Wallpaper not found: {}", wallpaper)));
    }

    // Check color cache
    if let Some(cache) = cache {
        match cache.get_cached_colors(wallpaper_path, &mode_str, scheme_type).await {
            Ok(Some(colors)) => {
                tracing::info!("Using cached colors for {} mode", mode_str);
                return Ok(colors);
            }
            Ok(None) => {}
            Err(e) => {
                tracing::debug!("Cache lookup failed: {}", e);
            }
        }
    }

    // Run matugen to generate colors
    let output = tokio::process::Command::new("matugen")
        .args([
            "--json",
            "hex",
            "--dry-run",
            "image",
            wallpaper,
            "--mode",
            &mode_str,
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

    let colors = lmtt_core::colors::parse_matugen_colors(&json, &mode_str)
        .map_err(Error::Matugen)?;

    // Write to cache on success
    if let Some(cache) = cache {
        if let Err(e) = cache.set_cached_colors(wallpaper_path, &mode_str, scheme_type, &colors).await {
            tracing::debug!("Failed to cache colors: {}", e);
        }
    }

    Ok(colors)
}
