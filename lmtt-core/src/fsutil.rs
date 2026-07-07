use std::path::Path;
use crate::Result;

/// Write a file atomically: write to a temp file in the same directory, then
/// rename over the target. Readers (inotify watchers, `@import`ing apps,
/// concurrent lmtt runs) never observe a truncated or partial file.
///
/// If the target is a symlink (a stow/chezmoi-managed dotfile), the write
/// goes THROUGH the link to its real destination and the link is preserved —
/// renaming over it would destroy the link and desync the dotfiles repo.
/// Existing file permissions are carried onto the replacement.
pub async fn write_atomic(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    // Resolve a symlinked target to its real path so temp+rename happens in
    // the real file's directory (same filesystem) and keeps the link intact.
    let real_path = match tokio::fs::read_link(path).await {
        Ok(link_target) => {
            if link_target.is_absolute() {
                link_target
            } else {
                path.parent().unwrap_or_else(|| Path::new(".")).join(link_target)
            }
        }
        Err(_) => path.to_path_buf(),
    };

    let dir = real_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = real_path
        .file_name()
        .ok_or_else(|| crate::Error::Config(format!("Not a file path: {}", real_path.display())))?;

    // Preserve the existing file's permissions on the replacement.
    let existing_perms = tokio::fs::metadata(&real_path)
        .await
        .ok()
        .map(|m| m.permissions());

    // Same-directory temp file so the rename stays on one filesystem.
    let tmp = dir.join(format!(".{}.lmtt-tmp", file_name.to_string_lossy()));

    tokio::fs::write(&tmp, contents).await?;
    if let Some(perms) = existing_perms {
        let _ = tokio::fs::set_permissions(&tmp, perms).await;
    }
    if let Err(e) = tokio::fs::rename(&tmp, &real_path).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_and_replaces() {
        let dir = std::env::temp_dir().join(format!("lmtt-fsutil-test-{}", std::process::id()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let target = dir.join("out.css");

        write_atomic(&target, "first").await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&target).await.unwrap(), "first");

        write_atomic(&target, "second").await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&target).await.unwrap(), "second");

        // No temp file left behind
        assert!(!dir.join(".out.css.lmtt-tmp").exists());
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
