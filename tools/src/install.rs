//! Builds modules (`build.rs`) and installs them in a launcher instance: the library always, then the mods asked for
//! and the mods they require (all the buildable ones by default). A module with Rust has its `.smod` installed in the
//! instance's `sunrise-native-mods/`, as the launcher's mod browser installs one, and the launcher builds the
//! instance's native mods (`sunrise-launcher native-build --instance`), which also makes that build the one the
//! instance plays: Play checks that each mod in the build is that installed package (its version and digest). A
//! content-only module's `.smod` goes in the instance's `sunrise-mods/`, turned on in `sunrise-mods.txt` (a native one too: the game loads only the mods turned on there). Pressing Play
//! on that instance then starts the game with them.
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

    let mut native = false;
    let installed = game.join(NATIVE_DIR);
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
            copy(&packed.smod, &installed)?;
            // The game loads only the mods its profile turns on: a new module has no line there yet.
            turn_on(&game.join("sunrise-mods.txt"), &m.id)?;
        } else {
            copy(&packed.smod, &content)?;
            turn_on(&game.join("sunrise-mods.txt"), &m.id)?;
        }
    }
    if !native {
        return Ok(());
    }
    // The launcher builds every native mod installed in the instance at once: those not asked for this time keep the
    // package installed before.
    run_in(
        &root,
        &launcher,
        &["native-build", "--instance", &id, "--trust-native-code"],
    )
}

/// The instance's folder of installed native mods (the launcher's `mod_list::NATIVE_DIR`).
const NATIVE_DIR: &str = "sunrise-native-mods";

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

/// Turns a mod on in the instance's `sunrise-mods.txt` (`mod.<id>:true`), in place of any line it had (one turned off).
fn turn_on(settings: &Path, id: &str) -> crate::Result {
    let key = format!("mod.{id}:");
    let line = format!("{key}true");
    let text = std::fs::read_to_string(settings).unwrap_or_default();
    if text.lines().any(|l| l.trim() == line) && text.lines().filter(|l| l.trim().starts_with(&key)).count() == 1 {
        return Ok(());
    }
    let mut lines: Vec<&str> = text.lines().filter(|l| !l.trim().starts_with(&key)).collect();
    lines.push(&line);
    std::fs::write(settings, lines.join("\n") + "\n").map_err(|e| format!("{}: {e}", settings.display()))
}
