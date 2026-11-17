//! Where the Sunrise launcher is on this computer, and where it keeps its data, for `install` and `vanilla`.
//!
//! - The launcher: `$SUNRISE_LAUNCHER`, else `sunrise-launcher` on the `PATH`, else a `sunrise-launcher*.AppImage` in
//!   the usual folders (the user's download folder as `xdg-user-dir` names it, `~/Downloads`, `~/Applications`,
//!   `~/.local/bin`, `~/bin`).
//! - Its data: `$SUNRISE_LAUNCHER_DATA`, else `$XDG_DATA_HOME/sunrise`, else `~/.local/share/sunrise`.
use std::path::{Path, PathBuf};
use std::process::Command;

fn home() -> crate::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".into())
}

/// The launcher's command.
pub fn binary() -> crate::Result<String> {
    if let Some(path) = std::env::var_os("SUNRISE_LAUNCHER") {
        return Ok(path.to_string_lossy().into_owned());
    }
    if on_path("sunrise-launcher") {
        return Ok("sunrise-launcher".into());
    }
    let home = home()?;
    let mut folders: Vec<PathBuf> = Vec::new();
    if let Some(downloads) = xdg_download_dir() {
        folders.push(downloads);
    }
    folders.extend(["Downloads", "Applications", ".local/bin", "bin"].map(|f| home.join(f)));
    folders
        .iter()
        .find_map(|dir| appimage_in(dir))
        .map(|p| p.to_string_lossy().into_owned())
        .ok_or_else(|| {
            "no Sunrise launcher found: set SUNRISE_LAUNCHER to its path, or put `sunrise-launcher` on the PATH".into()
        })
}

/// The launcher's data folder (`launcher/`: versions, Rust; `game/`: the instances).
pub fn data_dir() -> crate::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("SUNRISE_LAUNCHER_DATA") {
        return Ok(PathBuf::from(dir));
    }
    match std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        Some(dir) => Ok(PathBuf::from(dir).join("sunrise")),
        None => Ok(home()?.join(".local/share/sunrise")),
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join(name).is_file()))
}

fn xdg_download_dir() -> Option<PathBuf> {
    let out = Command::new("xdg-user-dir").arg("DOWNLOAD").output().ok()?;
    let dir = String::from_utf8(out.stdout).ok()?.trim().to_owned();
    (!dir.is_empty()).then(|| PathBuf::from(dir))
}

/// The newest-named launcher AppImage in `dir`.
fn appimage_in(dir: &Path) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_launcher_appimage(p))
        .collect();
    found.sort();
    found.pop()
}

fn is_launcher_appimage(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("sunrise-launcher") && n.ends_with(".AppImage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_launcher_appimage() {
        assert!(is_launcher_appimage(Path::new("/a/sunrise-launcher.AppImage")));
        assert!(is_launcher_appimage(Path::new(
            "/a/sunrise-launcher-linux-x86_64.AppImage"
        )));
        assert!(!is_launcher_appimage(Path::new("/a/sunrise-launcher.AppImage.part")));
        assert!(!is_launcher_appimage(Path::new("/a/other.AppImage")));
    }
}
