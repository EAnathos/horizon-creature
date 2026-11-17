//! Builds modules without installing them: the library always, then the mods asked for and the mods they require (all
//! the buildable ones by default). Each module is packaged (`pack.rs`: staged in `target/stage/<id>/`, zipped to
//! `target/<id>.smod`), checked by `cargo sunrise-mod check` and copied to `target/dist/`. The native Rust itself is
//! compiled by the launcher, in an instance (`install`).
use crate::install::run_in;
use crate::modules::{self, Module};

pub fn run(names: &[String]) -> crate::Result {
    let dist = crate::root().join("target/dist");
    if dist.exists() {
        std::fs::remove_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    }
    std::fs::create_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    for m in &chosen(names)? {
        let packed = module(m)?;
        let name = packed.smod.file_name().ok_or("no file name")?;
        std::fs::copy(&packed.smod, dist.join(name)).map_err(|e| format!("{}: {e}", packed.smod.display()))?;
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

/// Packages one module and checks its package as the registry does.
pub fn module(m: &Module) -> crate::Result<crate::pack::Packed> {
    println!("== {} ({})", m.name, m.id);
    let packed = crate::pack::run(m)?;
    run_in(
        &crate::root(),
        "cargo",
        &["sunrise-mod", "check", &packed.smod.to_string_lossy()],
    )?;
    Ok(packed)
}
