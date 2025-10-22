use std::path::PathBuf;
use std::collections::HashSet;

pub fn detect_gtk_themes() -> Vec<String> {
    let mut themes = HashSet::new();
    
    let search_paths = vec![
        dirs::home_dir().map(|h| h.join(".themes")),
        dirs::home_dir().map(|h| h.join(".local/share/themes")),
        Some(PathBuf::from("/usr/share/themes")),
        Some(PathBuf::from("/usr/local/share/themes")),
    ];
    
    for path in search_paths.into_iter().flatten() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        // Check if it has gtk-3.0 or gtk-4.0 subdirectory
                        let theme_name = entry.file_name();
                        let theme_path = entry.path();
                        if theme_path.join("gtk-3.0").exists() || theme_path.join("gtk-4.0").exists() {
                            if let Some(name) = theme_name.to_str() {
                                themes.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut theme_list: Vec<String> = themes.into_iter().collect();
    theme_list.sort();
    theme_list
}

pub fn detect_icon_themes() -> Vec<String> {
    let mut themes = HashSet::new();
    
    let search_paths = vec![
        dirs::home_dir().map(|h| h.join(".icons")),
        dirs::home_dir().map(|h| h.join(".local/share/icons")),
        Some(PathBuf::from("/usr/share/icons")),
        Some(PathBuf::from("/usr/local/share/icons")),
    ];
    
    for path in search_paths.into_iter().flatten() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        // Check if it has index.theme
                        let theme_name = entry.file_name();
                        if entry.path().join("index.theme").exists() {
                            if let Some(name) = theme_name.to_str() {
                                themes.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut theme_list: Vec<String> = themes.into_iter().collect();
    theme_list.sort();
    theme_list
}

pub fn detect_cursor_themes() -> Vec<String> {
    let mut themes = HashSet::new();
    
    let search_paths = vec![
        dirs::home_dir().map(|h| h.join(".icons")),
        dirs::home_dir().map(|h| h.join(".local/share/icons")),
        Some(PathBuf::from("/usr/share/icons")),
    ];
    
    for path in search_paths.into_iter().flatten() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        // Check if it has cursors subdirectory
                        let theme_name = entry.file_name();
                        if entry.path().join("cursors").exists() {
                            if let Some(name) = theme_name.to_str() {
                                themes.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    let mut theme_list: Vec<String> = themes.into_iter().collect();
    theme_list.sort();
    theme_list
}

pub fn detect_vscode_themes() -> Vec<String> {
    let mut themes = vec![
        "Default Dark+".to_string(),
        "Default Light+".to_string(),
        "Default Dark Modern".to_string(),
        "Default Light Modern".to_string(),
        "Default High Contrast".to_string(),
    ];
    
    let vscode_paths = vec![
        dirs::home_dir().map(|h| h.join(".vscode/extensions")),
        dirs::home_dir().map(|h| h.join(".config/Code/User/extensions")),
        dirs::home_dir().map(|h| h.join(".config/Cursor/User/extensions")),
    ];
    
    for path in vscode_paths.into_iter().flatten() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let entry_name = entry.file_name();
                if let Some(name) = entry_name.to_str() {
                    // Check for theme extensions
                    if name.contains("theme") {
                        let package_json = entry.path().join("package.json");
                        if let Ok(content) = std::fs::read_to_string(package_json) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(contributes) = json.get("contributes") {
                                    if let Some(theme_array) = contributes.get("themes").and_then(|t| t.as_array()) {
                                        for theme in theme_array {
                                            if let Some(label) = theme.get("label").and_then(|l| l.as_str()) {
                                                themes.push(label.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    themes.sort();
    themes.dedup();
    themes
}

pub fn detect_fonts() -> Vec<String> {
    let mut fonts = HashSet::new();
    
    // Use fc-list if available
    if let Ok(output) = std::process::Command::new("fc-list")
        .arg(":")
        .arg("family")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // fc-list can return multiple families comma-separated
                for family in line.split(',') {
                    let family = family.trim();
                    if !family.is_empty() {
                        fonts.insert(family.to_string());
                    }
                }
            }
        }
    }
    
    let mut font_list: Vec<String> = fonts.into_iter().collect();
    font_list.sort();
    font_list
}

pub fn detect_neovim_colorschemes() -> Vec<String> {
    let mut colorschemes = vec![
        "default".to_string(),
        "habamax".to_string(),
        "darkplus".to_string(),
        "oxocarbon".to_string(),
    ];
    
    if let Some(home) = dirs::home_dir() {
        let nvim_paths = vec![
            home.join(".config/nvim/colors"),
            home.join(".local/share/nvim/site/colors"),
            home.join(".local/share/nvim/lazy"),  // lazy.nvim plugins
        ];
        
        for path in nvim_paths {
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".vim") || name.ends_with(".lua") {
                            let colorscheme = name
                                .trim_end_matches(".vim")
                                .trim_end_matches(".lua")
                                .to_string();
                            colorschemes.push(colorscheme);
                        }
                    }
                }
            }
        }
    }
    
    colorschemes.sort();
    colorschemes.dedup();
    colorschemes
}
