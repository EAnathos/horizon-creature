//! Compiles every ported module's crate without the launcher, for CI and quick local checks: a workspace in
//! `target/native-check/` whose crates are ours (their `src/` in place), each on the API of the pinned Sunrise SDK
//! (`target/sunrise-sdk/`, fetched by `cargo sunrise-mod` in the library's staged package), the mods on the library as the
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
        None => fetch(&library)?,
    };
    let api = sdk.join("api/crates/sunrise-mod-api");
    if !api.join("Cargo.toml").is_file() {
        return Err(format!(
            "{}: no api/crates/sunrise-mod-api: not a Sunrise SDK",
            sdk.display()
        ));
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

/// The pinned SDK, in `target/sunrise-sdk/`: when missing or built from another commit than the library's pin,
/// fetched by `cargo sunrise-mod check` in the library's staged package (the check itself may fail, on a target we
/// don't build: only the fetch matters).
fn fetch(library: &Module) -> crate::Result<std::path::PathBuf> {
    let sdk = crate::root().join("target/sunrise-sdk");
    let manifest = std::fs::read_to_string(library.dir.join("sunrise-mod.toml")).unwrap_or_default();
    let pin = crate::pack::toml_value(&manifest, "sunrise");
    if sdk.join("api").is_dir() && pin.is_some() && sdk_commit(&sdk) == pin {
        return Ok(sdk);
    }
    let stage = crate::pack::stage(library)?;
    let _ = run_in(&stage, "cargo", &["sunrise-mod", "check"]);
    let fetched = stage.join(".sunrise/sdk");
    if !fetched.join("api").is_dir() {
        return Err("cargo sunrise-mod fetched no SDK".into());
    }
    if sdk.exists() {
        std::fs::remove_dir_all(&sdk).map_err(|e| format!("{}: {e}", sdk.display()))?;
    }
    std::fs::rename(&fetched, &sdk).map_err(|e| format!("{}: {e}", sdk.display()))?;
    Ok(sdk)
}

/// The Sunrise commit an SDK was built from (its `sdk.json`'s `build.commit`).
fn sdk_commit(sdk: &Path) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sdk.join("sdk.json")).ok()?).ok()?;
    Some(json["build"]["commit"].as_str()?.to_owned())
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
