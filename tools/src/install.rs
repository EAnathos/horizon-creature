//! Builds modules (`build.rs`) and installs them in a launcher instance: the library always, then the mods asked for
//! and the mods they require (all the buildable ones by default). A module with Rust is built by the launcher from its
//! staged folder (`sunrise-launcher native-build target/stage/native`), which also makes that build the one the
//! instance plays, with their content; a content-only module's `.smod` goes in the instance's `sunrise-mods/`, turned
//! on in `sunrise-mods.txt`. Pressing Play on that instance then starts the game with them.
//!
//! The launcher and its data folder are found as `launcher.rs` says (`$SUNRISE_LAUNCHER`, the `PATH`, the usual
//! folders).
use std::path::{Path, PathBuf};
use std::process::Command;

/// The test instance, by name.
pub const DEFAULT_INSTANCE: &str = "creature-dev";

pub fn run(instance: &str, names: &[String]) -> crate::Result {
    let root = crate::root();
    let launcher = crate::launcher::binary()?;
    let (id, game) = find_instance(&launcher, instance)?;

    let staged = root.join("target/stage/native");
    if staged.exists() {
        std::fs::remove_dir_all(&staged).map_err(|e| format!("{}: {e}", staged.display()))?;
    }
    let mut native = false;
    for m in &crate::build::chosen(names)? {
        let packed = crate::build::module(m)?;
        let content = game.join("sunrise-mods");
        if m.native().is_some() {
            native = true;
            // Built by the launcher with its content: a content-only copy from before would load it twice.
            let stale = content.join(format!("{}.smod", m.id));
            if stale.exists() {
                std::fs::remove_file(&stale).map_err(|e| format!("{}: {e}", stale.display()))?;
            }
        } else {
            copy(&packed.smod, &content)?;
            turn_on(&game.join("sunrise-mods.txt"), &m.id)?;
        }
    }
    if !native {
        return Ok(());
    }
    run_in(
        &root,
        &launcher,
        &[
            "native-build",
            &staged.to_string_lossy(),
            "--instance",
            &id,
            "--trust-native-code",
        ],
    )
}

/// The instance named `instance` (by name or id), from `sunrise-launcher instance list --json`: its id and game folder.
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

/// Turns a content mod on in the instance's `sunrise-mods.txt` (`mod.<id>:true`), once.
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
