pub mod cache;
pub mod colors;
pub mod config;
pub mod error;
pub mod fallback;
pub mod theme_detection;
pub mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use theme_detection::find_icon_theme_variant;
pub use types::{ColorScheme, ThemeMode};
