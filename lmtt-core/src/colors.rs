use std::collections::HashMap;

/// Convert hex color to RGB tuple (0-255)
pub fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), String> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: {}", hex));
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    
    Ok((r, g, b))
}

/// Convert RGB to comma-separated string for KDE
pub fn rgb_to_kde_string(r: u8, g: u8, b: u8) -> String {
    format!("{},{},{}", r, g, b)
}

/// Convert hex to sRGB tuple (0.0-1.0) for XDG portal
pub fn hex_to_srgb_tuple(hex: &str) -> Result<(f64, f64, f64), String> {
    let (r, g, b) = hex_to_rgb(hex)?;
    
    Ok((
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
    ))
}

/// Map Material You color to closest gsettings accent-color enum
pub fn map_to_accent_color(hex: &str) -> Result<String, String> {
    let (r, g, b) = hex_to_rgb(hex)?;
    
    // Find dominant channel
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let saturation = max.saturating_sub(min);
    
    // Low saturation = slate
    if saturation < 30 {
        return Ok("slate".to_string());
    }
    
    // Map based on hue
    if r >= g && r >= b {
        // Red dominant
        if g > b + 30 {
            Ok("orange".to_string())
        } else if b > g + 30 {
            Ok("pink".to_string())
        } else {
            Ok("red".to_string())
        }
    } else if g >= r && g >= b {
        // Green dominant
        if b > r + 20 {
            Ok("teal".to_string())
        } else if r > b + 20 {
            Ok("yellow".to_string())
        } else {
            Ok("green".to_string())
        }
    } else {
        // Blue dominant
        if r > g + 20 {
            Ok("purple".to_string())
        } else if g > r + 20 {
            Ok("teal".to_string())
        } else {
            Ok("blue".to_string())
        }
    }
}

/// Parse matugen JSON output into color map
/// Supports matugen 3.0.0 format: { "colors": { "color_name": { "dark": "#xxx", "light": "#yyy" } } }
pub fn parse_matugen_colors(json: &str, mode: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Failed to parse matugen JSON: {}", e))?;

    let colors = value
        .get("colors")
        .ok_or_else(|| "No colors object in matugen output".to_string())?;

    let mut map = HashMap::new();

    if let Some(obj) = colors.as_object() {
        for (color_name, color_value) in obj {
            // matugen 3.0.0: each color has dark/light/default variants
            if let Some(hex) = color_value.get(mode).and_then(|v| v.as_str()) {
                map.insert(color_name.clone(), hex.to_string());
            }
        }
    }

    if map.is_empty() {
        return Err(format!("No colors found for mode: {}", mode));
    }

    Ok(map)
}
