//! Build tools for the Sunrise Creatures series (docs/ARCHITECTURE.md). Run from anywhere in the repository:
//!
//! ```sh
//! cargo creature list                         # the library and the mods, with their status
//! cargo creature tab                          # writes the series' single creative tab and recipe table (library)
//! cargo creature model <mod> [model]          # mods/<mod>/sources/models/<model>.bbmodel -> the mod's src/
//! cargo creature preview                     # every mod's .bbmodel -> pictures in docs/models/
//! cargo creature sounds <mod> [folder]        # freesound downloads (default sounds-src/) -> the mod's sounds (vulture)
//! cargo creature pack <module>                # a module's package -> target/stage/, target/<id>.smod
//! cargo creature build [mod...]               # builds the library and the mods (default: all) into target/dist/
//! cargo creature check                        # the repository's consistency (tab, manifests, pins, lang, JSON, docs)
//! cargo creature native-check [sdk]           # compiles the ported crates (clippy, tests) on the pinned Sunrise SDK
//! cargo creature vanilla [jar]               # Minecraft's own data -> the Vanilla Bestiary's game_data.rs
//! cargo creature release [module...]         # the published modules (default: all) -> target/release/, registry/
//! cargo creature pages [module...]           # their registry pages again (text, icon, screenshots), same versions
//! cargo creature install [--instance <name>] [mod...]
//!                                             # builds the library and the mods (default: all), installs them in the
//!                                             # instance (default creature-dev), runs the launcher's native build
//! ```
//!
//! A module is the library (`creature`) or a mod's folder name under `mods/`. `cargo creature` is an alias
//! (`.cargo/config.toml`) for `cargo run --release --manifest-path tools/Cargo.toml --`.
mod build;
mod check;
mod icon;
mod install;
mod launcher;
mod model;
mod modules;
mod native_check;
mod pack;
mod preview;
mod recipes;
mod release;
mod sounds;
mod tab;
mod vanilla;

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

const USAGE: &str = "usage: cargo creature list | tab | model <mod> [model] | preview | sounds <mod> [folder] | pack <module> \
                     | build [mod...] | check | native-check [sdk] | vanilla [jar] \
                     | release [module...] | pages [module...] | install [--instance <name>] [mod...]";

fn list() -> Result {
    let all: Vec<_> = std::iter::once(modules::library()?).chain(modules::mods()?).collect();
    // Columns as wide as their longest entry.
    let width = |f: fn(&modules::Module) -> &str| all.iter().map(|m| f(m).chars().count()).max().unwrap_or(0);
    let (id_w, name_w, status_w) = (width(|m| &m.id), width(|m| &m.name), width(|m| &m.status));
    for m in &all {
        let parts: Vec<&str> = [("content", m.manifest().is_some()), ("native", m.native().is_some())]
            .into_iter()
            .filter_map(|(name, has)| has.then_some(name))
            .collect();
        let parts = if parts.is_empty() {
            "spec only".to_owned()
        } else {
            parts.join(" + ")
        };
        let requires = if m.requires.is_empty() {
            String::new()
        } else {
            format!(", requires {}", m.requires.join(", "))
        };
        println!(
            "{:<id_w$}  {:<name_w$}  {:<status_w$}  {parts}{requires}",
            m.id, m.name, m.status
        );
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
        Some("preview") => preview::run(),
        Some("sounds") => arg(1).ok_or(USAGE.into()).and_then(|m| sounds::run(m, arg(2))),
        Some("pack") => arg(1)
            .ok_or(USAGE.into())
            .and_then(modules::find)
            .and_then(|m| pack::run(&m).map(|_| ())),
        Some("build") => build::run(&args[1..]),
        Some("release") => release::run(&args[1..]),
        Some("pages") => release::pages(&args[1..]),
        Some("check") => check::run(),
        Some("native-check") => native_check::run(arg(1)),
        Some("vanilla") => vanilla::run(arg(1)),
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
