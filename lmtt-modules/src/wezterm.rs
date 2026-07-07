use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

crate::register_module!(WeztermModule);

pub struct WeztermModule;

impl Default for WeztermModule {
    fn default() -> Self {
        Self::new()
    }
}

impl WeztermModule {
    pub fn new() -> Self {
        Self
    }
}

/// Fixed ANSI hues per mode for the slots Material You has no semantic token
/// for. Green must be green and cyan must be cyan — `ls`, diffs, and TUIs
/// depend on the conventional hue of each slot.
struct AnsiHues {
    green: &'static str,
    bright_green: &'static str,
    yellow: &'static str,
    bright_yellow: &'static str,
    cyan: &'static str,
    bright_cyan: &'static str,
}

const DARK_HUES: AnsiHues = AnsiHues {
    green: "#98c379",
    bright_green: "#b5e890",
    yellow: "#e5c07b",
    bright_yellow: "#ecd09b",
    cyan: "#56b6c2",
    bright_cyan: "#7bdfec",
};

const LIGHT_HUES: AnsiHues = AnsiHues {
    green: "#50a14f",
    bright_green: "#3f8e3e",
    yellow: "#c18401",
    bright_yellow: "#a06d00",
    cyan: "#0184bc",
    bright_cyan: "#016d9c",
};

/// OSC color sequence: 10 = foreground, 11 = background, 12 = cursor,
/// 17/19 = selection bg/fg, 4;n = palette slot n (0-7 ansi, 8-15 brights).
fn build_osc_payload(
    foreground: &str,
    background: &str,
    cursor: &str,
    selection_bg: &str,
    selection_fg: &str,
    ansi: &[String; 8],
    brights: &[String; 8],
) -> String {
    let mut osc = String::new();
    osc.push_str(&format!("\x1b]10;{}\x07", foreground));
    osc.push_str(&format!("\x1b]11;{}\x07", background));
    osc.push_str(&format!("\x1b]12;{}\x07", cursor));
    osc.push_str(&format!("\x1b]17;{}\x07", selection_bg));
    osc.push_str(&format!("\x1b]19;{}\x07", selection_fg));
    osc.push_str("\x1b]4");
    for (i, color) in ansi.iter().chain(brights.iter()).enumerate() {
        osc.push_str(&format!(";{};{}", i, color));
    }
    osc.push('\x07');
    osc
}

/// Write the OSC payload to every wezterm pane tty. Failures are per-pane
/// and non-fatal (a pane may have vanished between list and write).
async fn apply_osc_to_panes(osc: &str) {
    let Ok(output) = tokio::process::Command::new("wezterm")
        .args(["cli", "--no-auto-start", "list", "--format", "json"])
        .output()
        .await
    else {
        return;
    };
    if !output.status.success() {
        tracing::debug!("[WezTerm] No running mux to recolor");
        return;
    }

    let Ok(panes) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return;
    };
    let Some(panes) = panes.as_array() else { return };

    let mut ttys: Vec<&str> = panes
        .iter()
        .filter_map(|p| p.get("tty_name").and_then(|t| t.as_str()))
        .collect();
    ttys.sort_unstable();
    ttys.dedup();

    let mut updated = 0;
    for tty in ttys {
        // Bound each open+write: a pane whose reader is stopped (Ctrl-S / a
        // full pty buffer) would otherwise block write_all forever and hang
        // the whole module until the registry timeout fails the switch.
        let one = async {
            use tokio::io::AsyncWriteExt;
            let mut f = tokio::fs::OpenOptions::new().write(true).open(tty).await.ok()?;
            f.write_all(osc.as_bytes()).await.ok()?;
            Some(())
        };
        match tokio::time::timeout(std::time::Duration::from_millis(200), one).await {
            Ok(Some(())) => updated += 1,
            Ok(None) => {}
            Err(_) => tracing::debug!("[WezTerm] Timed out writing OSC to {}, skipping", tty),
        }
    }
    if updated > 0 {
        tracing::info!("[WezTerm] Recolored {} live pane tty(s) via OSC", updated);
    }
}

#[async_trait]
impl ThemeModule for WeztermModule {
    fn name(&self) -> &'static str {
        "wezterm"
    }

    fn binary_name(&self) -> &'static str {
        "wezterm"
    }

    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let config_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?
            .join("wezterm");

        let colors_file = config_dir.join("lmtt-colors.lua");

        if let Some(parent) = colors_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let is_light = scheme.mode == ThemeMode::Light;
        let profile = if is_light {
            &config.theme_profiles.light
        } else {
            &config.theme_profiles.dark
        };
        let hues = if is_light { &LIGHT_HUES } else { &DARK_HUES };

        let foreground = scheme.get_or_fallback("on_surface");
        let background = scheme.get_or_fallback("surface");
        let cursor_bg = scheme.get_or_fallback("primary");
        let cursor_fg = scheme.get_or_fallback("on_primary");
        let selection_bg = scheme.get_or_fallback("primary_container");
        let selection_fg = scheme.get_or_fallback("on_primary_container");
        let outline = scheme.get_or_fallback("outline");
        let error = scheme.get_or_fallback("error");
        let on_surface_variant = scheme.get_or_fallback("on_surface_variant");
        let secondary = scheme.get_or_fallback("secondary");

        // ansi0 "black": a dark tone on dark bg, near-black fg tone on light
        let black = if is_light {
            foreground.clone()
        } else {
            scheme.get_or_fallback("surface_container_high")
        };

        // Slot order: black, red, green, yellow, blue, magenta, cyan, white.
        // blue/magenta carry the scheme accents (primary/secondary); the
        // hue-critical slots use the fixed per-mode values above.
        let ansi = [
            black,
            error.clone(),
            hues.green.to_string(),
            hues.yellow.to_string(),
            cursor_bg.clone(),
            secondary.clone(),
            hues.cyan.to_string(),
            on_surface_variant.clone(),
        ];

        let brights = [
            outline.clone(),
            error.clone(),
            hues.bright_green.to_string(),
            hues.bright_yellow.to_string(),
            cursor_bg.clone(),
            secondary.clone(),
            hues.bright_cyan.to_string(),
            foreground.clone(),
        ];

        let mut content = String::new();
        content.push_str("-- WezTerm colors generated by lmtt\n");
        content.push_str("return {\n");
        content.push_str(&format!("  foreground = '{}',\n", foreground));
        content.push_str(&format!("  background = '{}',\n", background));
        content.push_str(&format!("  cursor_bg = '{}',\n", cursor_bg));
        content.push_str(&format!("  cursor_fg = '{}',\n", cursor_fg));
        content.push_str(&format!("  cursor_border = '{}',\n", cursor_bg));
        content.push_str(&format!("  selection_fg = '{}',\n", selection_fg));
        content.push_str(&format!("  selection_bg = '{}',\n", selection_bg));
        content.push_str("  ansi = {\n");
        for color in &ansi {
            content.push_str(&format!("    '{}',\n", color));
        }
        content.push_str("  },\n");
        content.push_str("  brights = {\n");
        for color in &brights {
            content.push_str(&format!("    '{}',\n", color));
        }
        content.push_str("  },\n");

        // Tab bar / header area. Consumers with use_fancy_tab_bar should
        // also derive window_frame from tab_bar.background.
        let tab_bg = scheme.get_or_fallback("surface_container");
        let tab_hover_bg = scheme.get_or_fallback("surface_container_high");
        content.push_str("  tab_bar = {\n");
        content.push_str(&format!("    background = '{}',\n", tab_bg));
        content.push_str(&format!(
            "    active_tab = {{ bg_color = '{}', fg_color = '{}' }},\n",
            cursor_bg, cursor_fg
        ));
        content.push_str(&format!(
            "    inactive_tab = {{ bg_color = '{}', fg_color = '{}' }},\n",
            tab_bg, on_surface_variant
        ));
        content.push_str(&format!(
            "    inactive_tab_hover = {{ bg_color = '{}', fg_color = '{}' }},\n",
            tab_hover_bg, foreground
        ));
        content.push_str(&format!(
            "    new_tab = {{ bg_color = '{}', fg_color = '{}' }},\n",
            tab_bg, on_surface_variant
        ));
        content.push_str(&format!(
            "    new_tab_hover = {{ bg_color = '{}', fg_color = '{}' }},\n",
            tab_hover_bg, foreground
        ));
        content.push_str("  },\n");
        content.push_str("}\n");

        lmtt_core::fsutil::write_atomic(&colors_file, content).await?;
        tracing::info!("[WezTerm] Updated colors at {}", colors_file.display());

        // Profile file: omit unset/zero values entirely — emitting font = ''
        // or font_size = 0 verbatim gives consumers an invisible terminal.
        let profile_file = config_dir.join("lmtt-profile.lua");
        let mut profile_content = String::new();
        profile_content.push_str("-- WezTerm profile settings generated by lmtt\n");
        profile_content.push_str("-- Missing keys mean \"keep your default\"\n");
        profile_content.push_str("return {\n");

        if let Some(font) = profile.terminal_font.as_deref().filter(|f| !f.is_empty()) {
            // Escape for a single-quoted Lua string so an apostrophe in the
            // font name (e.g. "D'Ni Sans") can't produce invalid Lua that
            // breaks wezterm's config load on every evaluation.
            let escaped = font.replace('\\', "\\\\").replace('\'', "\\'");
            profile_content.push_str(&format!("  font = '{}',\n", escaped));
        }
        if profile.terminal_font_size > 0 {
            profile_content.push_str(&format!("  font_size = {},\n", profile.terminal_font_size));
        }
        if profile.terminal_opacity > 0.0 {
            profile_content.push_str(&format!("  opacity = {},\n", profile.terminal_opacity));
        }
        profile_content.push_str(&format!("  blur = {},\n", profile.window_blur));
        profile_content.push_str("}\n");

        lmtt_core::fsutil::write_atomic(&profile_file, profile_content).await?;
        tracing::info!("[WezTerm] Updated profile settings at {}", profile_file.display());

        // Recolor RUNNING panes via OSC escape sequences written to each
        // pane's tty — the same live-update path terminals support for
        // base16-style theme switching. No config reload is involved:
        // wezterm re-evaluates its Lua config once per window per reload
        // trigger, so configs should run automatically_reload_config=false
        // and rely on this for live updates (new panes read the colors file
        // at spawn). NEVER signal wezterm-gui: it has no reload signal
        // handler and SIGUSR1's default disposition terminates it.
        let osc = build_osc_payload(
            &foreground,
            &background,
            &cursor_bg,
            &selection_bg,
            &selection_fg,
            &ansi,
            &brights,
        );
        apply_osc_to_panes(&osc).await;

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        let config_dir = dirs::config_dir()
            .ok_or(lmtt_core::Error::Config("No config dir".into()))?;

        let wezterm_lua = config_dir.join("wezterm/wezterm.lua");

        if !wezterm_lua.exists() {
            return Ok(vec![]);
        }

        let content = tokio::fs::read_to_string(&wezterm_lua).await?;
        let include_line = "local colors = require('lmtt-colors')";
        let already_included = crate::is_included(&content, "lmtt-colors");

        Ok(vec![ConfigFileInfo {
            path: wezterm_lua,
            include_line: include_line.to_string(),
            description: "Require lmtt-colors in wezterm config (apply its values to your returned config)".to_string(),
            already_included,
        }])
    }
}
