//! Where the Horizon launcher is on this computer, and where it keeps its data, for `install` and `vanilla`.
//!
//! - The launcher: `$HORIZON_LAUNCHER`, else `horizon-launcher` on the `PATH`, else a `horizon-launcher*.AppImage` in
//!   the usual folders (the user's download folder as `xdg-user-dir` names it, `~/Downloads`, `~/Applications`,
//!   `~/.local/bin`, `~/bin`).
//! - Its data: `$HORIZON_LAUNCHER_DATA`, else `$XDG_DATA_HOME/horizon-launcher`, else
//!   `~/.local/share/horizon-launcher`.
use std::path::{Path, PathBuf};
use std::process::Command;

fn home() -> crate::Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".into())
}

/// The launcher's command.
pub fn binary() -> crate::Result<String> {
    if let Some(path) = std::env::var_os("HORIZON_LAUNCHER") {
        return Ok(path.to_string_lossy().into_owned());
    }
    if on_path("horizon-launcher") {
        return Ok("horizon-launcher".into());
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
            "no Horizon launcher found: set HORIZON_LAUNCHER to its path, or put `horizon-launcher` on the PATH".into()
        })
}

/// The launcher's data folder (instances, native builds, the prepared Minecraft jars).
pub fn data_dir() -> crate::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("HORIZON_LAUNCHER_DATA") {
        return Ok(PathBuf::from(dir));
    }
    match std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        Some(dir) => Ok(PathBuf::from(dir).join("horizon-launcher")),
        None => Ok(home()?.join(".local/share/horizon-launcher")),
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
        .is_some_and(|n| n.starts_with("horizon-launcher") && n.ends_with(".AppImage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_the_launcher_appimage() {
        assert!(is_launcher_appimage(Path::new("/a/horizon-launcher.AppImage")));
        assert!(is_launcher_appimage(Path::new(
            "/a/horizon-launcher-0.2.64-x86_64.AppImage"
        )));
        assert!(!is_launcher_appimage(Path::new("/a/horizon-launcher.AppImage.part")));
        assert!(!is_launcher_appimage(Path::new("/a/other.AppImage")));
    }
}
