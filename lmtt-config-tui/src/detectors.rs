use schema_tui::OptionProvider;
use anyhow::Result;

pub struct GtkThemeDetector;

impl OptionProvider for GtkThemeDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_gtk_themes())
    }
}

pub struct IconThemeDetector;

impl OptionProvider for IconThemeDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_icon_themes())
    }
}

pub struct CursorThemeDetector;

impl OptionProvider for CursorThemeDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_cursor_themes())
    }
}

pub struct VSCodeThemeDetector;

impl OptionProvider for VSCodeThemeDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_vscode_themes())
    }
}

pub struct FontDetector;

impl OptionProvider for FontDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_fonts())
    }
}

pub struct NeovimColorschemeDetector;

impl OptionProvider for NeovimColorschemeDetector {
    fn get_options(&self) -> Result<Vec<String>> {
        Ok(lmtt_core::theme_detection::detect_neovim_colorschemes())
    }
}

/// Register all LMTT option providers with the schema-tui builder
pub fn register_all(builder: schema_tui::SchemaTUIBuilder) -> schema_tui::SchemaTUIBuilder {
    builder
        .register_option_provider("gtk_themes", Box::new(GtkThemeDetector))
        .register_option_provider("icon_themes", Box::new(IconThemeDetector))
        .register_option_provider("cursor_themes", Box::new(CursorThemeDetector))
        .register_option_provider("vscode_themes", Box::new(VSCodeThemeDetector))
        .register_option_provider("fonts", Box::new(FontDetector))
        .register_option_provider("nvim_colorschemes", Box::new(NeovimColorschemeDetector))
}
