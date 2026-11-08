//! Builds modules without installing them: the library always, then the mods asked for and the mods they require (all
//! the buildable ones by default). Each module's content part (`cargo horizon-mod build`) and native part (`pack.rs`,
//! then `cargo horizon-mod check`) end up in `target/dist/`. The native Rust itself is compiled by the launcher, in an
//! instance (`install`).
use std::path::PathBuf;

use crate::install::run_in;
use crate::modules::{self, Module};

/// What a module's build made.
pub struct Built {
    pub hmod: Option<PathBuf>,
    pub hnmod: Option<PathBuf>,
}

pub fn run(names: &[String]) -> crate::Result {
    let dist = crate::root().join("target/dist");
    std::fs::create_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    for m in &chosen(names)? {
        let built = module(m)?;
        for file in built.hmod.iter().chain(&built.hnmod) {
            let name = file.file_name().ok_or("no file name")?;
            std::fs::copy(file, dist.join(name)).map_err(|e| format!("{}: {e}", file.display()))?;
        }
    }
    println!("{}", dist.display());
    Ok(())
}

/// The library, then the mods named with the mods they require (all the buildable ones when none is named). Writes the
/// shared creative tab first: it lists every mod's items.
pub fn chosen(names: &[String]) -> crate::Result<Vec<Module>> {
    crate::tab::run()?;
    let mut chosen = vec![modules::library()?];
    if names.is_empty() {
        chosen.extend(modules::mods()?.into_iter().filter(Module::buildable));
    } else {
        for m in modules::with_requirements(names)? {
            if !m.buildable() {
                return Err(format!("{} is {}: nothing to build yet", m.id, m.status));
            }
            chosen.push(m);
        }
    }
    Ok(chosen)
}

/// Builds one module's parts.
pub fn module(m: &Module) -> crate::Result<Built> {
    println!("== {} ({})", m.name, m.id);
    let mut built = Built {
        hmod: None,
        hnmod: None,
    };
    if let Some(content) = m.content() {
        run_in(&content, "cargo", &["horizon-mod", "build"])?;
        built.hmod = Some(content.join(format!("target/horizon-mod/{}.hmod", m.id)));
    }
    if m.native().is_some() {
        let hnmod = crate::pack::run(m)?;
        run_in(
            &crate::root(),
            "cargo",
            &["horizon-mod", "check", &hnmod.to_string_lossy()],
        )?;
        built.hnmod = Some(hnmod);
    }
    Ok(built)
}
