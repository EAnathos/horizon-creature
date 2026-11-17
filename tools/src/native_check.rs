//! Compiles every ported module's crate without the launcher, for CI and quick local checks: a workspace in
//! `target/native-check/` whose crates are ours (their `src/` in place), each on the API of the pinned Sunrise SDK
//! (`common/.sunrise/sdk/`, which any `cargo sunrise-mod` command in `common/` fetches), the mods on the library as the
//! `creature` crate (as the launcher gives it) and on the mods they require. Then `cargo clippy` on it, warnings denied,
//! and their unit tests (`cargo test`: pure logic, nothing that needs a running game).
//!
//! Every crate must use the one SDK copy: two copies of its crates in one build collide.
use std::path::Path;

use crate::install::run_in;
use crate::modules::{self, LIBRARY, Module};

pub fn run(sdk: Option<&str>) -> crate::Result {
    let library = modules::library()?;
    let sdk = match sdk {
        Some(dir) => std::fs::canonicalize(dir).map_err(|e| format!("{dir}: {e}"))?,
        None => {
            let dir = library.dir.join(".sunrise/sdk");
            if !dir.is_dir() {
                // Fetches the pinned build's SDK into common/.sunrise/sdk/.
                run_in(&library.dir, "cargo", &["sunrise-mod", "check"])?;
            }
            dir
        }
    };
    let api = sdk.join("api/crates/sunrise-mod-api");
    if !api.join("Cargo.toml").is_file() {
        return Err(format!("{}: no api/crates/sunrise-mod-api: not a Sunrise SDK", sdk.display()));
    }
    let ws = crate::root().join("target/native-check");
    std::fs::create_dir_all(&ws).map_err(|e| format!("{}: {e}", ws.display()))?;
    let modules: Vec<Module> = std::iter::once(library)
        .chain(modules::mods()?)
        .filter(|m| m.native().is_some())
        .collect();
    for m in &modules {
        crate_manifest(&ws, m, &api)?;
    }
    let members: Vec<String> = modules.iter().map(|m| format!("\"{}\"", m.id)).collect();
    let workspace = format!(
        "# Made by `cargo creature native-check`: our crates against the pinned Sunrise SDK.\n\
         [workspace]\nmembers = [{}]\nresolver = \"2\"\n",
        members.join(", ")
    );
    write(&ws.join("Cargo.toml"), &workspace)?;
    run_in(
        &ws,
        "cargo",
        &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    )?;
    run_in(&ws, "cargo", &["test", "--workspace"])
}

/// `target/native-check/<id>/Cargo.toml`: the module's crate, by its sources, on the SDK's API at `api`.
fn crate_manifest(ws: &Path, m: &Module, api: &Path) -> crate::Result {
    let src = m.dir.join("src/lib.rs");
    let mut deps = format!("sunrise-mod-api = {{ path = {:?} }}\n", api.display().to_string());
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
