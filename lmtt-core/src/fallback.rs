use std::collections::HashMap;

pub fn fallback_dark_colors() -> HashMap<String, String> {
    let mut colors = HashMap::new();
    
    colors.insert("surface".to_string(), "#12131a".to_string());
    colors.insert("on_surface".to_string(), "#e3e1ec".to_string());
    colors.insert("surface_variant".to_string(), "#44464f".to_string());
    colors.insert("on_surface_variant".to_string(), "#c5c5d6".to_string());
    colors.insert("surface_container".to_string(), "#1e1f27".to_string());
    colors.insert("surface_container_high".to_string(), "#292931".to_string());
    colors.insert("surface_container_highest".to_string(), "#33343c".to_string());
    
    colors.insert("primary".to_string(), "#9fd491".to_string());
    colors.insert("on_primary".to_string(), "#003a03".to_string());
    colors.insert("primary_container".to_string(), "#22511c".to_string());
    colors.insert("on_primary_container".to_string(), "#bbf0aa".to_string());
    
    colors.insert("secondary".to_string(), "#edb8cd".to_string());
    colors.insert("on_secondary".to_string(), "#4a2532".to_string());
    colors.insert("secondary_container".to_string(), "#633b48".to_string());
    colors.insert("on_secondary_container".to_string(), "#ffd9e3".to_string());
    
    colors.insert("tertiary".to_string(), "#bbc3fa".to_string());
    colors.insert("on_tertiary".to_string(), "#1e2a5a".to_string());
    colors.insert("tertiary_container".to_string(), "#3b4472".to_string());
    colors.insert("on_tertiary_container".to_string(), "#dee0ff".to_string());
    
    colors.insert("error".to_string(), "#ffb4ab".to_string());
    colors.insert("on_error".to_string(), "#690005".to_string());
    colors.insert("error_container".to_string(), "#93000a".to_string());
    colors.insert("on_error_container".to_string(), "#ffdad6".to_string());
    
    colors.insert("outline".to_string(), "#8f909f".to_string());
    colors.insert("outline_variant".to_string(), "#44464f".to_string());
    
    colors.insert("background".to_string(), "#12131a".to_string());
    colors.insert("on_background".to_string(), "#e3e1ec".to_string());
    
    colors
}

pub fn fallback_light_colors() -> HashMap<String, String> {
    let mut colors = HashMap::new();
    
    colors.insert("surface".to_string(), "#fbf8ff".to_string());
    colors.insert("on_surface".to_string(), "#1a1b23".to_string());
    colors.insert("surface_variant".to_string(), "#e0e2ec".to_string());
    colors.insert("on_surface_variant".to_string(), "#44464f".to_string());
    colors.insert("surface_container".to_string(), "#efedf4".to_string());
    colors.insert("surface_container_high".to_string(), "#e9e7ef".to_string());
    colors.insert("surface_container_highest".to_string(), "#e3e1ec".to_string());
    
    colors.insert("primary".to_string(), "#3a6a33".to_string());
    colors.insert("on_primary".to_string(), "#ffffff".to_string());
    colors.insert("primary_container".to_string(), "#bbf0aa".to_string());
    colors.insert("on_primary_container".to_string(), "#003a03".to_string());
    
    colors.insert("secondary".to_string(), "#7d525f".to_string());
    colors.insert("on_secondary".to_string(), "#ffffff".to_string());
    colors.insert("secondary_container".to_string(), "#ffd9e3".to_string());
    colors.insert("on_secondary_container".to_string(), "#31101d".to_string());
    
    colors.insert("tertiary".to_string(), "#555d8f".to_string());
    colors.insert("on_tertiary".to_string(), "#ffffff".to_string());
    colors.insert("tertiary_container".to_string(), "#dee0ff".to_string());
    colors.insert("on_tertiary_container".to_string(), "#0e1848".to_string());
    
    colors.insert("error".to_string(), "#ba1a1a".to_string());
    colors.insert("on_error".to_string(), "#ffffff".to_string());
    colors.insert("error_container".to_string(), "#ffdad6".to_string());
    colors.insert("on_error_container".to_string(), "#410002".to_string());
    
    colors.insert("outline".to_string(), "#74767f".to_string());
    colors.insert("outline_variant".to_string(), "#c4c6d0".to_string());
    
    colors.insert("background".to_string(), "#fbf8ff".to_string());
    colors.insert("on_background".to_string(), "#1a1b23".to_string());
    
    colors
}
