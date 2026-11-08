//! Build tools for the Horizon Creature series (docs/ARCHITECTURE.md). Run from anywhere in the repository:
//!
//! ```sh
//! cargo creature list                         # the library and the mods, with their status
//! cargo creature tab                          # writes the series' single creative tab (library)
//! cargo creature model <mod> [model]          # mods/<mod>/assets/models/<model>.bbmodel -> the mod's native/api/src/
//! cargo creature sounds <mod> [folder]        # freesound downloads (default sounds-src/) -> the mod's sounds (vulture)
//! cargo creature pack <module>                # a module's native/ -> target/<id>.hnmod
//! cargo creature install [--instance <name>] [mod...]
//!                                             # builds the library and the mods (default: all), installs them in the
//!                                             # instance (default vulture-dev), runs its native build
//! ```
//!
//! A module is the library (`creature`) or a mod's folder name under `mods/`. `cargo creature` is an alias
//! (`.cargo/config.toml`) for `cargo run --release --manifest-path tools/Cargo.toml --`.
mod install;
mod model;
mod modules;
mod pack;
mod sounds;
mod tab;

use std::path::PathBuf;
use std::process::ExitCode;

/// The repository root: the parent of this crate's folder.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools/ is inside the repository")
        .to_path_buf()
}

pub type Result<T = ()> = std::result::Result<T, String>;

const USAGE: &str = "usage: cargo creature list | tab | model <mod> [model] | sounds <mod> [folder] | pack <module> \
                     | install [--instance <name>] [mod...]";

fn list() -> Result {
    for m in std::iter::once(modules::library()?).chain(modules::mods()?) {
        let parts: Vec<&str> = [("content", m.content().is_some()), ("native", m.native().is_some())]
            .into_iter()
            .filter_map(|(name, has)| has.then_some(name))
            .collect();
        let parts = if parts.is_empty() { "spec only".to_owned() } else { parts.join(" + ") };
        let requires = if m.requires.is_empty() {
            String::new()
        } else {
            format!(", requires {}", m.requires.join(", "))
        };
        println!("{:<12} {:<18} {:<10} {parts}{requires}", m.id, m.name, m.status);
    }
    Ok(())
}

fn install(args: &[String]) -> Result {
    let mut instance = install::DEFAULT_INSTANCE.to_owned();
    let mut names = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--instance" {
            instance = args.next().ok_or("--instance needs a name")?.clone();
        } else {
            names.push(arg.clone());
        }
    }
    install::run(&instance, &names)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str);
    let done = match arg(0) {
        Some("list") => list(),
        Some("tab") => tab::run(),
        Some("model") => arg(1).ok_or(USAGE.into()).and_then(|m| model::run(m, arg(2))),
        Some("sounds") => arg(1).ok_or(USAGE.into()).and_then(|m| sounds::run(m, arg(2))),
        Some("pack") => arg(1)
            .ok_or(USAGE.into())
            .and_then(modules::find)
            .and_then(|m| pack::run(&m).map(|_| ())),
        Some("install") => install(&args[1..]),
        _ => Err(USAGE.into()),
    };
    match done {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
