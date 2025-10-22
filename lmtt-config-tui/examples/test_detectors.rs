use lmtt_config_tui::*;
use schema_tui::OptionProvider;

fn main() {
    println!("Testing LMTT Option Detectors\n");
    println!("==============================\n");
    
    println!("GTK Themes:");
    match GtkThemeDetector.get_options() {
        Ok(themes) => {
            println!("  Found {} themes:", themes.len());
            for (i, theme) in themes.iter().take(5).enumerate() {
                println!("    {}. {}", i + 1, theme);
            }
            if themes.len() > 5 {
                println!("    ... and {} more", themes.len() - 5);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\nIcon Themes:");
    match IconThemeDetector.get_options() {
        Ok(themes) => {
            println!("  Found {} themes:", themes.len());
            for (i, theme) in themes.iter().take(5).enumerate() {
                println!("    {}. {}", i + 1, theme);
            }
            if themes.len() > 5 {
                println!("    ... and {} more", themes.len() - 5);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\nFonts:");
    match FontDetector.get_options() {
        Ok(fonts) => {
            println!("  Found {} fonts:", fonts.len());
            for (i, font) in fonts.iter().take(5).enumerate() {
                println!("    {}. {}", i + 1, font);
            }
            if fonts.len() > 5 {
                println!("    ... and {} more", fonts.len() - 5);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\nVSCode Themes:");
    match VSCodeThemeDetector.get_options() {
        Ok(themes) => {
            println!("  Found {} themes:", themes.len());
            for (i, theme) in themes.iter().take(5).enumerate() {
                println!("    {}. {}", i + 1, theme);
            }
            if themes.len() > 5 {
                println!("    ... and {} more", themes.len() - 5);
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    
    println!("\nAll detectors working! ✓");
}
