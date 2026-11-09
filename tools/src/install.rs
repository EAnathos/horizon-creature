//! Builds modules (`build.rs`) and installs them in a launcher instance, as the Mods page's "Build" button would: the
//! library always, then the mods asked for and the mods they require (all the buildable ones by default), each with its
//! content part (`.hmod`) and its native part (`.hnmod`); then the instance's native build. Pressing Play on that
//! instance then starts the game with them. A mod the launcher leaves out of that build fails the install.
//!
//! The launcher is `$HORIZON_LAUNCHER`, else `~/Téléchargements/horizon-launcher.AppImage`.
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::modules::Module;

/// The test instance, by name (its id and folder are still `vulture-dev`, from the time the repository held only the
/// vulture: the launcher renames an instance without moving it).
pub const DEFAULT_INSTANCE: &str = "creature-dev";

pub fn run(instance: &str, names: &[String]) -> crate::Result {
    let root = crate::root();
    let home = PathBuf::from(std::env::var("HOME").map_err(|_| "HOME is not set")?);
    let launcher = std::env::var("HORIZON_LAUNCHER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("Téléchargements/horizon-launcher.AppImage"));
    let launcher = launcher.to_string_lossy().into_owned();
    let (id, game) = find_instance(&launcher, instance)?;

    for m in &crate::build::chosen(names)? {
        let built = crate::build::module(m)?;
        copy_in(m, &built, &game)?;
    }
    run_in(
        &root,
        &launcher,
        &[
            "native-build",
            "--instance",
            &id,
            "--trust-native-code",
            "--profile",
            "dev",
        ],
    )?;
    left_out(&home.join(".local/share/horizon-launcher/native/build.log"))
}

/// The launcher still succeeds when it leaves a mod out of the build (an injection it refuses, a helper file with two
/// functions...), and the game then starts without it: its build log says so, which fails the install.
fn left_out(log: &Path) -> crate::Result {
    let Ok(text) = std::fs::read_to_string(log) else {
        return Ok(());
    };
    let problems: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("was left out") || l.contains("\"message\""))
        .map(str::trim)
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the native build left mods out ({}):\n{}",
            log.display(),
            problems.join("\n")
        ))
    }
}

/// Copies one built module into the instance, and turns its content on.
fn copy_in(m: &Module, built: &crate::build::Built, game: &Path) -> crate::Result {
    if let Some(hmod) = &built.hmod {
        copy(hmod, &game.join("horizon-mods"))?;
        turn_on(&game.join("horizon-mods.txt"), &m.id)?;
    }
    if let Some(hnmod) = &built.hnmod {
        copy(hnmod, &game.join("horizon-native-mods"))?;
    }
    Ok(())
}

/// The instance named `instance` (by name or id), from `horizon-launcher instance list --json`: its id and game folder.
fn find_instance(launcher: &str, instance: &str) -> crate::Result<(String, PathBuf)> {
    let out = Command::new(launcher)
        .args(["instance", "list", "--json"])
        .output()
        .map_err(|e| format!("{launcher}: {e}"))?;
    let list: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("{launcher} instance list --json: {e}"))?;
    list.as_array()
        .into_iter()
        .flatten()
        .find(|i| i["name"] == instance || i["id"] == instance)
        .and_then(|i| Some((i["id"].as_str()?.to_owned(), PathBuf::from(i["home"].as_str()?))))
        .ok_or_else(|| format!("no instance {instance} in the launcher"))
}

pub fn run_in(dir: &Path, program: &str, args: &[&str]) -> crate::Result {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed ({status})", args.join(" ")))
    }
}

fn copy(file: &Path, into: &Path) -> crate::Result {
    std::fs::create_dir_all(into).map_err(|e| format!("{}: {e}", into.display()))?;
    let name = file.file_name().ok_or("no file name")?;
    std::fs::copy(file, into.join(name)).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(())
}

/// Turns a content mod on in the instance's `horizon-mods.txt` (`mod.<id>:true`), once.
fn turn_on(settings: &Path, id: &str) -> crate::Result {
    let line = format!("mod.{id}:true");
    let text = std::fs::read_to_string(settings).unwrap_or_default();
    if text.lines().any(|l| l.trim() == line) {
        return Ok(());
    }
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    std::fs::write(settings, format!("{text}{sep}{line}\n")).map_err(|e| format!("{}: {e}", settings.display()))
}
