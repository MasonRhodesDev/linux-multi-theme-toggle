use std::collections::HashMap;

/// Whether a string is a syntactically valid hex color: optional leading '#'
/// then exactly 3, 6, or 8 hex digits and nothing else. This is the trust
/// boundary for color values — they get interpolated into shell scripts
/// (`fish -c`), Lua, and terminal escape sequences, so anything that isn't a
/// bare hex color must be rejected before it reaches those sinks.
pub fn is_hex_color(value: &str) -> bool {
    let digits = value.strip_prefix('#').unwrap_or(value);
    matches!(digits.len(), 3 | 6 | 8) && digits.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Convert hex color to RGB tuple (0-255).
/// Accepts #rgb, #rrggbb, and #rrggbbaa (alpha ignored).
pub fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), String> {
    let digits = hex.trim_start_matches('#');

    if !digits.is_ascii() {
        return Err(format!("Invalid hex color: {}", hex));
    }

    let parse =
        |s: &str| u8::from_str_radix(s, 16).map_err(|_| format!("Invalid hex color: {}", hex));

    match digits.len() {
        3 => {
            let expand = |s: &str| parse(s).map(|v| v * 17); // "a" -> 0xaa
            Ok((
                expand(&digits[0..1])?,
                expand(&digits[1..2])?,
                expand(&digits[2..3])?,
            ))
        }
        6 | 8 => Ok((
            parse(&digits[0..2])?,
            parse(&digits[2..4])?,
            parse(&digits[4..6])?,
        )),
        _ => Err(format!("Invalid hex color: {}", hex)),
    }
}

/// GNOME `org.gnome.desktop.interface accent-color` names. The portal exposes
/// accent as `(ddd)`; the writable host key is this enum, not arbitrary RGB.
const GNOME_ACCENTS: &[(&str, u8, u8, u8)] = &[
    ("blue", 0x35, 0x84, 0xe4),
    ("teal", 0x21, 0x90, 0xa4),
    ("green", 0x3a, 0x94, 0x4a),
    ("yellow", 0xc8, 0x88, 0x00),
    ("orange", 0xed, 0x5b, 0x00),
    ("red", 0xe6, 0x2d, 0x42),
    ("pink", 0xd5, 0x61, 0x99),
    ("purple", 0x91, 0x41, 0xac),
    ("slate", 0x6f, 0x83, 0x96),
];

/// Nearest named GNOME accent for a hex color, if the hex is valid.
pub fn nearest_gnome_accent(hex: &str) -> Option<&'static str> {
    let (r, g, b) = hex_to_rgb(hex).ok()?;
    let mut best = GNOME_ACCENTS[0];
    let mut best_dist = u32::MAX;
    for entry in GNOME_ACCENTS {
        let dr = i32::from(r) - i32::from(entry.1);
        let dg = i32::from(g) - i32::from(entry.2);
        let db = i32::from(b) - i32::from(entry.3);
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best = *entry;
        }
    }
    Some(best.0)
}

/// Parse matugen JSON output into color map
/// Supports both formats:
/// - v3 actual: { "colors": { "dark": { "primary": "#xxx", ... }, "light": { ... } } }
/// - v3 legacy:  { "colors": { "primary": { "dark": "#xxx", "light": "#yyy" }, ... } }
pub fn parse_matugen_colors(json: &str, mode: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse matugen JSON: {}", e))?;

    let colors = value
        .get("colors")
        .ok_or_else(|| "No colors object in matugen output".to_string())?;

    let mut map = HashMap::new();

    // Try v3 actual format first: colors[mode][color_name]
    if let Some(mode_obj) = colors.get(mode).and_then(|v| v.as_object()) {
        for (color_name, color_value) in mode_obj {
            if let Some(hex) = color_value.as_str() {
                map.insert(color_name.clone(), hex.to_string());
            }
        }
    }

    // Fall back to legacy format: colors[color_name][mode]
    if map.is_empty() {
        if let Some(obj) = colors.as_object() {
            for (color_name, color_value) in obj {
                if let Some(hex) = color_value.get(mode).and_then(|v| v.as_str()) {
                    map.insert(color_name.clone(), hex.to_string());
                }
            }
        }
    }

    if map.is_empty() {
        return Err(format!("No colors found for mode: {}", mode));
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb_forms() {
        assert_eq!(hex_to_rgb("#ffb77a").unwrap(), (0xff, 0xb7, 0x7a));
        assert_eq!(hex_to_rgb("ffb77a").unwrap(), (0xff, 0xb7, 0x7a));
        assert_eq!(hex_to_rgb("#abc").unwrap(), (0xaa, 0xbb, 0xcc));
        assert_eq!(hex_to_rgb("#ffb77a80").unwrap(), (0xff, 0xb7, 0x7a));
        assert!(hex_to_rgb("#xyz").is_err());
        assert!(hex_to_rgb("#ffff").is_err());
        // Multibyte input must error, not panic on byte slicing
        assert!(hex_to_rgb("€€").is_err());
        assert!(hex_to_rgb("#€€€€€€").is_err());
    }

    #[test]
    fn test_parse_v3_actual_format() {
        let json = r##"{"colors":{"dark":{"primary":"#d0bcff","secondary":"#ccc2dc"},"light":{"primary":"#6750a4","secondary":"#625b71"}}}"##;
        let colors = parse_matugen_colors(json, "dark").unwrap();
        assert_eq!(colors.get("primary").unwrap(), "#d0bcff");
        assert_eq!(colors.get("secondary").unwrap(), "#ccc2dc");

        let colors = parse_matugen_colors(json, "light").unwrap();
        assert_eq!(colors.get("primary").unwrap(), "#6750a4");
    }

    #[test]
    fn test_parse_legacy_format() {
        let json = r##"{"colors":{"primary":{"dark":"#d0bcff","light":"#6750a4"},"secondary":{"dark":"#ccc2dc","light":"#625b71"}}}"##;
        let colors = parse_matugen_colors(json, "dark").unwrap();
        assert_eq!(colors.get("primary").unwrap(), "#d0bcff");
        assert_eq!(colors.get("secondary").unwrap(), "#ccc2dc");

        let colors = parse_matugen_colors(json, "light").unwrap();
        assert_eq!(colors.get("primary").unwrap(), "#6750a4");
    }

    #[test]
    fn test_parse_missing_mode() {
        let json = r##"{"colors":{"dark":{"primary":"#d0bcff"}}}"##;
        let result = parse_matugen_colors(json, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No colors found"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_matugen_colors("not json", "dark");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse"));
    }

    #[test]
    fn nearest_accent_picks_green_for_material_primary() {
        assert_eq!(nearest_gnome_accent("#3a944a"), Some("green"));
        assert_eq!(nearest_gnome_accent("#e62d42"), Some("red"));
        assert_eq!(nearest_gnome_accent("#3584e4"), Some("blue"));
    }

    #[test]
    fn test_parse_no_colors_key() {
        let result = parse_matugen_colors(r##"{"other": {}}"##, "dark");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No colors object"));
    }
}
