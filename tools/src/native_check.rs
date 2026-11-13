//! Compiles every module's native `api` crate without the Horizon launcher, for CI and quick local checks: a workspace in
//! `target/native-check/` whose crates are ours (their `src/` in place), each depending on the Horizon source given
//! (`horizon-client` by path, at the series' pinned revision), the mods on the library as the `creature` crate (as the
//! launcher gives it) and on the mods they require. Then `cargo clippy` on it, warnings denied.
//!
//! The injections (`native/*.rs` helpers) are not compiled here: they compile inside Horizon's own crates, in the
//! launcher's build only.
use std::path::Path;

use crate::install::run_in;
use crate::modules::{self, LIBRARY, Module};

pub fn run(horizon: &str) -> crate::Result {
    let horizon = std::fs::canonicalize(horizon).map_err(|e| format!("{horizon}: {e}"))?;
    let client = horizon.join("crates/horizon-client");
    if !client.join("Cargo.toml").is_file() {
        return Err(format!(
            "{}: no crates/horizon-client: not a Horizon source",
            horizon.display()
        ));
    }
    let ws = crate::root().join("target/native-check");
    std::fs::create_dir_all(&ws).map_err(|e| format!("{}: {e}", ws.display()))?;
    let modules: Vec<Module> = std::iter::once(modules::library()?)
        .chain(modules::mods()?)
        .filter(|m| m.native().is_some())
        .collect();
    for m in &modules {
        crate_manifest(&ws, m, &client)?;
    }
    let members: Vec<String> = modules.iter().map(|m| format!("\"{}\"", m.id)).collect();
    let workspace = format!(
        "# Made by `cargo creature native-check`: our native crates against a Horizon source.\n\
         [workspace]\nmembers = [{}]\nresolver = \"2\"\n",
        members.join(", ")
    );
    write(&ws.join("Cargo.toml"), &workspace)?;
    // Horizon's lock file: the same dependency versions as the launcher's build (cargo adds our crates to it).
    let lock = horizon.join("Cargo.lock");
    if lock.is_file() {
        std::fs::copy(&lock, ws.join("Cargo.lock")).map_err(|e| format!("{}: {e}", lock.display()))?;
    }
    run_in(
        &ws,
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    )
}

/// `target/native-check/<id>/Cargo.toml`: the module's `api` crate, by its sources, against `client`.
fn crate_manifest(ws: &Path, m: &Module, client: &Path) -> crate::Result {
    let src = m.native().ok_or("no native part")?.join("api/src/lib.rs");
    let mut deps = format!("horizon-client = {{ path = {:?} }}\n", client.display().to_string());
    if m.id != LIBRARY {
        deps += &format!("creature = {{ package = \"mod-{LIBRARY}\", path = \"../{LIBRARY}\" }}\n");
        for r in &m.requires {
            deps += &format!("{r} = {{ package = \"mod-{r}\", path = \"../{r}\" }}\n");
        }
    }
    let text = format!(
        "[package]\nname = \"mod-{id}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [lib]\npath = {src:?}\n\n[dependencies]\n{deps}",
        id = m.id,
        src = src.display().to_string(),
    );
    let dir = ws.join(&m.id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    write(&dir.join("Cargo.toml"), &text)
}

fn write(path: &Path, text: &str) -> crate::Result {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}
