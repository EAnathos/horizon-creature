//! Builds the mod and installs it in a launcher instance, as the Mods page's "Build" button would: the content part
//! (`.hmod`) and the native part (`.hnmod`), then the instance's native build. Pressing Play on that instance then
//! starts the game with the vulture.
//!
//! The launcher is `$HORIZON_LAUNCHER`, else `~/Téléchargements/horizon-launcher.AppImage`.
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(instance: &str) -> crate::Result {
    let root = crate::root();
    let home = PathBuf::from(std::env::var("HOME").map_err(|_| "HOME is not set")?);
    let launcher = std::env::var("HORIZON_LAUNCHER")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("Téléchargements/horizon-launcher.AppImage"));
    let game = home.join(".local/share/horizon/instances").join(instance);

    run_in(&root.join("content"), "cargo", &["horizon-mod", "build"])?;
    let hnmod = crate::pack::run()?;
    run_in(
        &root,
        "cargo",
        &["horizon-mod", "check", &hnmod.to_string_lossy()],
    )?;
    copy(
        &root.join("content/target/horizon-mod/vulture.hmod"),
        &game.join("horizon-mods"),
    )?;
    copy(&hnmod, &game.join("horizon-native-mods"))?;
    turn_on(&game.join("horizon-mods.txt"))?;
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

/// Turns the mod on in the instance's `horizon-mods.txt` (`mod.vulture:true`), once.
fn turn_on(settings: &Path) -> crate::Result {
    let text = std::fs::read_to_string(settings).unwrap_or_default();
    if text.lines().any(|l| l.trim() == "mod.vulture:true") {
        return Ok(());
    }
    let sep = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    std::fs::write(settings, format!("{text}{sep}mod.vulture:true\n"))
        .map_err(|e| format!("{}: {e}", settings.display()))
}
