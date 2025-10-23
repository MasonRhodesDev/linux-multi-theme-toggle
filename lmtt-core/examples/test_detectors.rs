fn main() {
    println!("Testing GTK theme detection:");
    let gtk = lmtt_core::theme_detection::detect_gtk_themes();
    println!("  Found {} themes: {:?}", gtk.len(), gtk);
    
    println!("\nTesting icon theme detection:");
    let icons = lmtt_core::theme_detection::detect_icon_themes();
    println!("  Found {} themes: {:?}", icons.len(), icons);
    
    println!("\nTesting cursor theme detection:");
    let cursors = lmtt_core::theme_detection::detect_cursor_themes();
    println!("  Found {} themes: {:?}", cursors.len(), cursors);
    
    println!("\nTesting font detection:");
    let fonts = lmtt_core::theme_detection::detect_fonts();
    println!("  Found {} fonts (showing first 10): {:?}", fonts.len(), fonts.iter().take(10).collect::<Vec<_>>());
}
