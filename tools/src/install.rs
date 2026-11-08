//! Builds modules and installs them in a launcher instance, as the Mods page's "Build" button would: the library
//! always, then the mods asked for and the mods they require (all the buildable ones by default), each with its content part (`.hmod`) and its
//! native part (`.hnmod`); then the instance's native build. Pressing Play on that instance then starts the game with
//! them.
//!
//! The launcher is `$HORIZON_LAUNCHER`, else `~/Téléchargements/horizon-launcher.AppImage`.
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::modules::{self, Module};

/// The test instance, from the time the repository held only the vulture.
pub const DEFAULT_INSTANCE: &str = "vulture-dev";

pub fn run(instance: &str, names: &[String]) -> crate::Result {
    let root = crate::root();
    let home = PathBuf::from(std::env::var("HOME").map_err(|_| "HOME is not set")?);
    let launcher = std::env::var("HORIZON_LAUNCHER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("Téléchargements/horizon-launcher.AppImage"));
    let game = home.join(".local/share/horizon/instances").join(instance);

    // The shared creative tab lists every mod's items: written before the library is built.
    crate::tab::run()?;
    let mut chosen = vec![modules::library()?];
    if names.is_empty() {
        chosen.extend(modules::mods()?.into_iter().filter(Module::buildable));
    } else {
        // A mod comes with the mods it requires (`requires` in its module.toml), installed before it.
        for m in modules::with_requirements(names)? {
            if !m.buildable() {
                return Err(format!("{} is {}: nothing to build yet", m.id, m.status));
            }
            chosen.push(m);
        }
    }
    for m in &chosen {
        install(&root, m, &game)?;
    }
    let launcher = launcher.to_string_lossy().into_owned();
    run_in(
        &root,
        &launcher,
        &[
            "native-build",
            "--instance",
            instance,
            "--trust-native-code",
            "--profile",
            "dev",
        ],
    )
}

/// Builds and copies one module's parts into the instance, and turns its content on.
fn install(root: &Path, m: &Module, game: &Path) -> crate::Result {
    println!("== {} ({})", m.name, m.id);
    if let Some(content) = m.content() {
        run_in(&content, "cargo", &["horizon-mod", "build"])?;
        copy(
            &content.join(format!("target/horizon-mod/{}.hmod", m.id)),
            &game.join("horizon-mods"),
        )?;
        turn_on(&game.join("horizon-mods.txt"), &m.id)?;
    }
    if m.native().is_some() {
        let hnmod = crate::pack::run(m)?;
        run_in(
            root,
            "cargo",
            &["horizon-mod", "check", &hnmod.to_string_lossy()],
        )?;
        copy(&hnmod, &game.join("horizon-native-mods"))?;
    }
    Ok(())
}

fn run_in(dir: &Path, program: &str, args: &[&str]) -> crate::Result {
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
    std::fs::write(settings, format!("{text}{sep}{line}\n"))
        .map_err(|e| format!("{}: {e}", settings.display()))
}
