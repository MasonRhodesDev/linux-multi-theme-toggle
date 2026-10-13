//! `lmtt wallpaper unset` against a throwaway XDG tree, publish skipped.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sandbox() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lmtt-unset-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("config")).unwrap();
    std::fs::create_dir_all(dir.join("cache")).unwrap();
    dir
}

fn lmtt(sandbox: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lmtt"))
        .args(args)
        .env("HOME", sandbox)
        .env("XDG_CONFIG_HOME", sandbox.join("config"))
        .env("XDG_CACHE_HOME", sandbox.join("cache"))
        .env("XDG_DATA_HOME", sandbox.join("data"))
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("run lmtt")
}

fn ok(out: &Output) -> String {
    assert!(
        out.status.success(),
        "lmtt failed ({}): {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn unset_removes_only_that_override_and_is_idempotent() {
    let sb = sandbox();
    let image = sb.join("wall.png");
    std::fs::write(&image, b"set only canonicalizes; publish is skipped").unwrap();
    let image = image.to_str().unwrap();
    let profile = sb.join("config/appearance-profiles/default.toml");

    ok(&lmtt(&sb, &["wallpaper", "set", image, "--no-publish"]));
    ok(&lmtt(
        &sb,
        &[
            "wallpaper",
            "set",
            image,
            "--output",
            "desc:Test Monitor",
            "--no-publish",
        ],
    ));
    ok(&lmtt(
        &sb,
        &[
            "wallpaper",
            "set",
            image,
            "--output",
            "DP-9",
            "--no-publish",
        ],
    ));
    let text = std::fs::read_to_string(&profile).unwrap();
    assert!(text.contains("desc:Test Monitor"), "{text}");
    assert!(text.contains("DP-9"), "{text}");

    let out = ok(&lmtt(
        &sb,
        &[
            "wallpaper",
            "unset",
            "--output",
            "desc:Test Monitor",
            "--no-publish",
        ],
    ));
    assert!(out.contains("Removed"), "{out}");
    let text = std::fs::read_to_string(&profile).unwrap();
    assert!(!text.contains("desc:Test Monitor"), "{text}");
    assert!(text.contains("DP-9"), "other overrides survive: {text}");
    assert!(
        text.contains("[background]"),
        "the global rule survives: {text}"
    );

    // Idempotent: exit 0, nothing rewritten.
    let before = std::fs::metadata(&profile).unwrap().modified().unwrap();
    let out = ok(&lmtt(
        &sb,
        &[
            "wallpaper",
            "unset",
            "--output",
            "desc:Test Monitor",
            "--no-publish",
        ],
    ));
    assert!(out.contains("No wallpaper override"), "{out}");
    assert_eq!(
        std::fs::metadata(&profile).unwrap().modified().unwrap(),
        before
    );

    std::fs::remove_dir_all(&sb).ok();
}
