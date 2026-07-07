use crate::{ThemeModule, ConfigFileInfo};
use async_trait::async_trait;
use lmtt_core::{ColorScheme, Config, Result, ThemeMode};

crate::register_module!(VSCodeModule);

pub struct VSCodeModule;

impl Default for VSCodeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl VSCodeModule {
    pub fn new() -> Self {
        Self
    }
}

const THEME_KEY: &str = "workbench.colorTheme";

/// Set a top-level string key in a settings.json WITHOUT parsing it as JSON.
/// VSCode settings are JSONC (comments, trailing commas) — round-tripping
/// through serde_json rejects valid files, and any "fall back to empty map"
/// path destroys the user's entire configuration. Targeted string surgery
/// preserves comments, formatting, and key order byte-for-byte.
///
/// A JSONC-aware scan (tracks strings, // and /* */ comments, and object/
/// array nesting) locates the key only when it is a genuine key of the ROOT
/// object — never a match inside a comment, string value, or nested object —
/// and replaces its entire value (even a non-string one) with the quoted new
/// value. Returns None when no root object is found.
fn set_string_key(content: &str, key: &str, value: &str) -> Option<String> {
    let escaped_value = format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""));
    let b = content.as_bytes();

    // Locate the root object's opening brace (skipping leading comments/ws).
    let mut i = skip_trivia(b, 0);
    if i >= b.len() || b[i] != b'{' {
        return None;
    }
    let root_brace = i;
    i += 1;
    let mut depth = 1;

    while i < b.len() {
        i = skip_trivia(b, i);
        if i >= b.len() {
            break;
        }
        match b[i] {
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'"' => {
                let (s, after) = read_json_string(b, i)?;
                let mut j = skip_trivia(b, after);
                // A string directly inside the root object followed by ':' is a key.
                if depth == 1 && j < b.len() && b[j] == b':' && s == key {
                    j = skip_trivia(b, j + 1);
                    let value_end = scan_value_end(b, j)?;
                    let mut out = String::with_capacity(content.len() + escaped_value.len());
                    out.push_str(&content[..j]);
                    out.push_str(&escaped_value);
                    out.push_str(&content[value_end..]);
                    return Some(out);
                }
                i = after;
            }
            _ => i += 1,
        }
    }

    // Key absent: insert right after the root object's opening brace.
    let mut out = String::with_capacity(content.len() + escaped_value.len() + key.len() + 16);
    out.push_str(&content[..=root_brace]);
    out.push_str(&format!("\n  \"{}\": {},", key, escaped_value));
    out.push_str(&content[root_brace + 1..]);
    Some(out)
}

/// Advance past JSON whitespace and // line / /* block */ comments.
fn skip_trivia(b: &[u8], mut i: usize) -> usize {
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
            i += 2;
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(b.len());
        } else {
            break;
        }
    }
    i
}

/// Read a JSON string starting at the opening quote b[i] == '"'.
/// Returns (unescaped-enough-for-key-compare content, index past closing quote).
fn read_json_string(b: &[u8], i: usize) -> Option<(String, usize)> {
    debug_assert_eq!(b[i], b'"');
    let mut j = i + 1;
    let mut s = String::new();
    while j < b.len() {
        match b[j] {
            b'\\' if j + 1 < b.len() => {
                s.push(b[j + 1] as char);
                j += 2;
            }
            b'"' => return Some((s, j + 1)),
            c => {
                s.push(c as char);
                j += 1;
            }
        }
    }
    None
}

/// Given the start of a JSON value, return the index just past its end
/// (before the trailing comma / closing bracket). Handles scalars, strings,
/// and nested objects/arrays, skipping strings and comments within.
fn scan_value_end(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    if i >= b.len() {
        return None;
    }
    if b[i] == b'"' {
        let (_, after) = read_json_string(b, i)?;
        return Some(after);
    }
    if b[i] == b'{' || b[i] == b'[' {
        let mut depth = 0;
        while i < b.len() {
            i = skip_trivia(b, i);
            if i >= b.len() {
                break;
            }
            match b[i] {
                b'"' => {
                    let (_, after) = read_json_string(b, i)?;
                    i = after;
                }
                b'{' | b'[' => {
                    depth += 1;
                    i += 1;
                }
                b'}' | b']' => {
                    depth -= 1;
                    i += 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => i += 1,
            }
        }
        return None;
    }
    // Scalar (number / true / false / null): ends at the next , } ] or trivia.
    while i < b.len() && !matches!(b[i], b',' | b'}' | b']') && !b[i].is_ascii_whitespace() && b[i] != b'/' {
        i += 1;
    }
    Some(i)
}

#[async_trait]
impl ThemeModule for VSCodeModule {
    fn name(&self) -> &'static str {
        "vscode"
    }

    fn binary_name(&self) -> &'static str {
        "code"
    }

    /// Any supported editor counts as installed — gating on `code` alone
    /// skips Cursor/VSCodium/Code-OSS-only setups whose settings paths this
    /// module explicitly supports.
    fn is_installed(&self) -> bool {
        ["code", "cursor", "codium", "code-oss"]
            .iter()
            .any(|bin| which::which(bin).is_ok())
    }

    async fn apply(&self, scheme: &ColorScheme, config: &Config) -> Result<()> {
        let home = dirs::home_dir()
            .ok_or(lmtt_core::Error::Config("No home dir".into()))?;

        let settings_paths = vec![
            home.join(".config/Code/User/settings.json"),
            home.join(".config/Cursor/User/settings.json"),
            home.join(".config/Code - OSS/User/settings.json"),
            home.join(".config/VSCodium/User/settings.json"),
        ];

        let is_light = scheme.mode == ThemeMode::Light;

        let profile = if is_light {
            &config.theme_profiles.light
        } else {
            &config.theme_profiles.dark
        };

        let theme = profile.vscode_theme.as_deref().unwrap_or({
            if is_light { "Default Light+" } else { "Default Dark+" }
        });

        let mut updated_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for path in settings_paths {
            if !path.exists() {
                continue;
            }

            let content = tokio::fs::read_to_string(&path).await?;
            match set_string_key(&content, THEME_KEY, theme) {
                Some(new_content) => {
                    if new_content != content {
                        lmtt_core::fsutil::write_atomic(&path, new_content).await?;
                        tracing::info!("[VSCode] Updated {}", path.display());
                    } else {
                        tracing::debug!("[VSCode] {} already set in {}", theme, path.display());
                    }
                    updated_count += 1;
                }
                None => {
                    errors.push(format!(
                        "could not safely edit {} (no object braces or malformed value)",
                        path.display()
                    ));
                }
            }
        }

        if !errors.is_empty() {
            return Err(lmtt_core::Error::Module(errors.join("; ")));
        }
        if updated_count == 0 {
            tracing::debug!("[VSCode] No installations found");
        }

        Ok(())
    }

    async fn config_files(&self) -> Result<Vec<ConfigFileInfo>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_existing_value_preserving_jsonc() {
        let content = "{\n  // my comment\n  \"editor.fontSize\": 14,\n  \"workbench.colorTheme\": \"Old Theme\",\n  \"files.trimTrailingWhitespace\": true,\n}\n";
        let out = set_string_key(content, THEME_KEY, "New Theme").unwrap();
        assert!(out.contains("// my comment"));
        assert!(out.contains("\"editor.fontSize\": 14,"));
        assert!(out.contains("\"workbench.colorTheme\": \"New Theme\""));
        assert!(out.contains("\"files.trimTrailingWhitespace\": true,"));
        assert!(!out.contains("Old Theme"));
    }

    #[test]
    fn inserts_missing_key_after_brace() {
        let content = "{\n  \"editor.fontSize\": 14\n}\n";
        let out = set_string_key(content, THEME_KEY, "Theme").unwrap();
        assert!(out.starts_with("{\n  \"workbench.colorTheme\": \"Theme\","));
        assert!(out.contains("\"editor.fontSize\": 14"));
    }

    #[test]
    fn handles_escaped_quotes_in_value() {
        let content = "{ \"workbench.colorTheme\": \"Weird \\\"Theme\\\"\" }";
        let out = set_string_key(content, THEME_KEY, "Plain").unwrap();
        assert!(out.contains("\"workbench.colorTheme\": \"Plain\""));
    }

    #[test]
    fn escapes_new_value() {
        let content = "{}";
        let out = set_string_key(content, THEME_KEY, "A \"B\" C").unwrap();
        assert!(out.contains("\\\"B\\\""));
    }

    #[test]
    fn returns_none_on_no_object() {
        assert!(set_string_key("", THEME_KEY, "X").is_none());
    }

    #[test]
    fn ignores_key_name_inside_array_value() {
        // The key name appears first inside another setting's array value —
        // must not anchor there and corrupt the following setting.
        let content = "{\n  \"settingsSync.ignoredSettings\": [\"workbench.colorTheme\"],\n  \"editor.fontFamily\": \"Fira Code\",\n  \"workbench.colorTheme\": \"Old\"\n}\n";
        let out = set_string_key(content, THEME_KEY, "New").unwrap();
        assert!(out.contains("\"editor.fontFamily\": \"Fira Code\""), "fontFamily must be untouched: {out}");
        assert!(out.contains("[\"workbench.colorTheme\"]"), "ignoredSettings array untouched: {out}");
        assert!(out.contains("\"workbench.colorTheme\": \"New\""));
        assert!(!out.contains("\"Old\""));
    }

    #[test]
    fn ignores_key_inside_comment() {
        let content = "{\n  // \"workbench.colorTheme\": \"commented\",\n  \"workbench.colorTheme\": \"Real\"\n}\n";
        let out = set_string_key(content, THEME_KEY, "New").unwrap();
        assert!(out.contains("// \"workbench.colorTheme\": \"commented\""), "comment preserved: {out}");
        assert!(out.contains("\"workbench.colorTheme\": \"New\""));
        assert!(!out.contains("\"Real\""));
    }

    #[test]
    fn replaces_non_string_value_without_corrupting_next_key() {
        // Existing value is null — must be replaced with a quoted string,
        // and the following key/value must be left intact.
        let content = "{ \"workbench.colorTheme\": null, \"editor.fontSize\": 14 }";
        let out = set_string_key(content, THEME_KEY, "Default Dark+").unwrap();
        assert!(out.contains("\"workbench.colorTheme\": \"Default Dark+\""), "{out}");
        assert!(out.contains("\"editor.fontSize\": 14"), "fontSize preserved: {out}");
        assert!(!out.contains("null"));
    }

    #[test]
    fn insertion_uses_root_brace_not_comment_brace() {
        let content = "// see { docs } here\n{\n  \"editor.fontSize\": 14\n}\n";
        let out = set_string_key(content, THEME_KEY, "T").unwrap();
        // Inserted key must be inside the real object, after the real brace
        assert!(out.contains("// see { docs } here\n{\n  \"workbench.colorTheme\": \"T\","), "{out}");
    }
}
