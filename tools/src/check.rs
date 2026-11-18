//! Checks the repository's consistency, without building anything (docs/CONVENTIONS.md). Run by CI.
//!
//! - The shared creative tab is up to date with the modules' `module.toml` (`cargo creature tab`).
//! - Each module: its `sunrise-mod.toml` carries its id, the version of its `Cargo.toml`, and the library's Sunrise and
//!   mixins pins (`[native]`, as CI's `SUNRISE_REV`); its `Cargo.toml` takes the API from the pinned SDK; a mod
//!   depends on the library and on every mod its `module.toml` requires.
//! - A mod's tab items are declared items; its `requires` name mods of the series, without cycles.
//! - Every JSON file parses; each module's `en_us` and `fr_fr` lang files have the same keys.
//! - Generated files are up to date: each model's Rust (`cargo creature model`), and each model has its pictures in
//!   `docs/models/` (`cargo creature preview`).
//! - Sounds: every sound a module's `sounds.json` names is an `.ogg` of it, and every `.ogg` is named there; every
//!   subtitle is translated; when the module has a `sources/sounds.toml`, its clips are exactly its `.ogg` files.
//! - Recipes: each crafting or cooking recipe has its recipe book advancement (`advancement/recipes/<tab>/<name>.json`,
//!   rewarding it), or the recipe book never shows it. Brewing recipes have none, as vanilla's.
//! - The Markdown docs use no em dash.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::modules::{self, LIBRARY, Module};

pub fn run() -> crate::Result {
    let mut errors = Vec::new();
    let root = crate::root();

    let (tab, text, _) = crate::tab::render()?;
    if std::fs::read_to_string(&tab).ok().as_deref() != Some(&text) {
        errors.push(format!("{}: out of date, run `cargo creature tab`", rel(&tab)));
    }
    let (table, text, _) = crate::recipes::render()?;
    if std::fs::read_to_string(&table).ok().as_deref() != Some(&text) {
        errors.push(format!("{}: out of date, run `cargo creature tab`", rel(&table)));
    }

    let library = modules::library()?;
    let mods = modules::mods()?;
    let pins = library
        .manifest()
        .and_then(|p| manifest(&p).ok())
        .and_then(|t| native_pins(&t));
    let Some(pins) = pins else {
        return Err("common/sunrise-mod.toml pins no [native] sunrise and mixins".into());
    };
    let ci = root.join(".github/workflows/ci.yml");
    if std::fs::read_to_string(&ci).is_ok_and(|t| !t.contains(&format!("SUNRISE_REV: {}", pins.0))) {
        errors.push(format!(
            "{}: SUNRISE_REV is not the series' revision {}",
            rel(&ci),
            pins.0
        ));
    }
    let revision = &pins;
    for m in std::iter::once(&library).chain(&mods) {
        module(m, &mods, revision, &mut errors);
    }
    for m in std::iter::once(&library).chain(&mods) {
        models(m, &mut errors);
        sounds(m, &mut errors);
        recipes(m, &mut errors);
    }
    let ids: Vec<String> = mods.iter().map(|m| m.id.clone()).collect();
    if let Err(e) = modules::with_requirements(&ids) {
        errors.push(e);
    }

    let mut files = Vec::new();
    walk(&root, &mut files);
    for file in &files {
        match file.extension().and_then(|e| e.to_str()) {
            Some("json") => {
                if let Err(e) = read_json(file) {
                    errors.push(e);
                }
            }
            Some("md") => {
                let text = std::fs::read_to_string(file).unwrap_or_default();
                for (i, line) in text.lines().enumerate() {
                    if line.contains('\u{2014}') {
                        errors.push(format!("{}:{}: em dash (docs/CONVENTIONS.md)", rel(file), i + 1));
                    }
                }
            }
            _ => {}
        }
    }

    if errors.is_empty() {
        println!("ok: {} modules, {} files", mods.len() + 1, files.len());
        return Ok(());
    }
    for e in &errors {
        eprintln!("{e}");
    }
    Err(format!("{} problems", errors.len()))
}

/// A manifest's `[native]` pins: the Sunrise commit and the horizon-mixins commit.
fn native_pins(t: &toml::Table) -> Option<(String, String)> {
    let native = t.get("native")?;
    Some((
        native.get("sunrise")?.as_str()?.to_owned(),
        native.get("mixins")?.as_str()?.to_owned(),
    ))
}

/// The `Cargo.toml` dependency line every module's crate takes the API from.
const API: &str = "sunrise-mod-api = { path = \".sunrise/sdk/api/crates/sunrise-mod-api\" }";

fn module(m: &Module, mods: &[Module], revision: &(String, String), errors: &mut Vec<String>) {
    let mut push = |e: String| errors.push(e);
    let mut versions = Vec::new();
    if let Some(path) = m.manifest() {
        match manifest(&path) {
            Ok(t) => {
                if t.get("id").and_then(|v| v.as_str()) != Some(&m.id) {
                    push(format!("{}: id is not {}", rel(&path), m.id));
                }
                versions.extend(t.get("version").and_then(|v| v.as_str()).map(str::to_owned));
                if native_pins(&t).as_ref() != Some(revision) {
                    push(format!(
                        "{}: [native] is not pinned to the series' Sunrise {} and mixins {}",
                        rel(&path),
                        revision.0,
                        revision.1
                    ));
                }
                if m.id != LIBRARY {
                    let deps = t.get("dependencies").and_then(|d| d.as_table());
                    for dep in std::iter::once(LIBRARY).chain(m.requires.iter().map(String::as_str)) {
                        if !deps.is_some_and(|d| d.contains_key(dep)) {
                            push(format!("{}: [dependencies] lacks {dep}", rel(&path)));
                        }
                    }
                }
            }
            Err(e) => push(e),
        }
        lang(&m.dir.join(format!("assets/{}/lang", m.id)), &mut push);
        for item in &m.creative {
            let name = item.split_once(':').map_or(item.as_str(), |(_, n)| n);
            let data = m.dir.join(format!("data/{}/sunrise", m.id));
            if !data.join(format!("item/{name}.json")).is_file() && !data.join(format!("block/{name}.json")).is_file() {
                push(format!(
                    "{}: creative item {item} is declared nowhere",
                    rel(&m.dir.join("module.toml"))
                ));
            }
        }
        let cargo = m.dir.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&cargo) {
            if !text.contains(API) {
                push(format!("{}: the API is not the pinned SDK's ({API})", rel(&cargo)));
            }
            versions.extend(
                text.lines()
                    .find_map(|l| l.trim().strip_prefix("version = \"")?.strip_suffix('"'))
                    .map(str::to_owned),
            );
        }
    } else if !m.creative.is_empty() {
        push(format!(
            "{}: creative items but no package",
            rel(&m.dir.join("module.toml"))
        ));
    }
    if versions.windows(2).any(|v| v[0] != v[1]) {
        push(format!(
            "{}: sunrise-mod.toml and Cargo.toml versions differ ({})",
            m.id,
            versions.join(", ")
        ));
    }
    for r in &m.requires {
        if r == &m.id || !mods.iter().any(|o| &o.id == r) {
            push(format!(
                "{}: requires {r}, which is not another mod of the series",
                rel(&m.dir.join("module.toml"))
            ));
        }
    }
}

/// Each of the module's `.bbmodel` files: its Rust as `cargo creature model` makes it, and its pictures.
fn models(m: &Module, errors: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(m.dir.join("sources/models")) else {
        return;
    };
    let docs = crate::root().join("docs");
    for path in entries.flatten().map(|e| e.path()) {
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".bbmodel"))
        else {
            continue;
        };
        match crate::model::render(&m.id, Some(name)) {
            Ok((out, rust, _)) => {
                if std::fs::read_to_string(&out).ok().as_deref() != Some(&rust) {
                    let args = if name == m.id {
                        m.id.clone()
                    } else {
                        format!("{} {name}", m.id)
                    };
                    errors.push(format!("{}: out of date, run `cargo creature model {args}`", rel(&out)));
                }
            }
            Err(e) => errors.push(e),
        }
        let pictures = ["front", "back"].map(|side| docs.join(format!("models/{}-{name}-{side}.png", m.id)));
        if !pictures.iter().all(|p| p.is_file()) {
            errors.push(format!(
                "{}: no pictures in docs/, run `cargo creature preview`",
                rel(&path)
            ));
        }
    }
}

/// The module's sounds: `sounds.json` and the `.ogg` files agree, subtitles are translated, `sounds.toml`'s clips are
/// the `.ogg` files.
fn sounds(m: &Module, errors: &mut Vec<String>) {
    let Some(content) = m.manifest().map(|_| m.dir.clone()) else {
        return;
    };
    let assets = content.join(format!("assets/{}", m.id));
    let json = assets.join("sounds.json");
    let oggs = ogg_names(&assets.join("sounds"));
    let toml_path = m.dir.join("sources/sounds.toml");
    if !json.is_file() {
        if !oggs.is_empty() || toml_path.is_file() {
            errors.push(format!("{}: sound files but no sounds.json", rel(&assets)));
        }
        return;
    }
    let Ok(events) = read_json(&json) else {
        return; // reported with the JSON files
    };
    let keys: BTreeSet<String> = read_json(&assets.join("lang/en_us.json"))
        .ok()
        .and_then(|v| Some(v.as_object()?.keys().cloned().collect()))
        .unwrap_or_default();
    let (named, subtitles) = sound_events(&events);
    for name in &named {
        match name.split_once(':') {
            Some((ns, path)) if ns == m.id && oggs.contains(path) => {}
            _ => errors.push(format!("{}: {name} has no .ogg in this module", rel(&json))),
        }
    }
    for ogg in &oggs {
        if !named.contains(&format!("{}:{ogg}", m.id)) {
            errors.push(format!("{}: sounds/{ogg}.ogg is in no event", rel(&json)));
        }
    }
    for key in subtitles.iter().filter(|k| !keys.contains(*k)) {
        errors.push(format!("{}: subtitle {key} is not translated", rel(&json)));
    }
    if let Ok(table) = manifest(&toml_path) {
        let out = table.get("out").and_then(|o| o.as_str()).unwrap_or_default();
        let out = out.strip_prefix("sounds/").unwrap_or(out);
        let clips: BTreeSet<String> = table
            .get("clip")
            .and_then(|c| c.as_array())
            .into_iter()
            .flatten()
            .filter_map(|c| Some(format!("{out}/{}", c.get("name")?.as_str()?)))
            .collect();
        for missing in clips.symmetric_difference(&oggs) {
            errors.push(format!(
                "{}: {missing} is {} (run `cargo creature sounds {}`)",
                rel(&toml_path),
                if clips.contains(missing) {
                    "a clip with no .ogg"
                } else {
                    "an .ogg with no clip"
                },
                m.id
            ));
        }
    }
}

/// The `.ogg` files under `dir`, as a sound event names them: their path from `dir`, without the extension.
fn ogg_names(dir: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    walk(dir, &mut files);
    files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "ogg"))
        .filter_map(|f| {
            Some(
                f.strip_prefix(dir)
                    .ok()?
                    .with_extension("")
                    .to_str()?
                    .replace('\\', "/"),
            )
        })
        .collect()
}

/// The sounds a `sounds.json` names (`ns:path`) and its subtitle keys.
fn sound_events(events: &serde_json::Value) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut named = BTreeSet::new();
    let mut subtitles = BTreeSet::new();
    for event in events.as_object().into_iter().flat_map(|o| o.values()) {
        if let Some(s) = event["subtitle"].as_str() {
            subtitles.insert(s.to_owned());
        }
        for entry in event["sounds"].as_array().into_iter().flatten() {
            if let Some(id) = entry.as_str().or_else(|| entry["name"].as_str()) {
                named.insert(id.to_owned());
            }
        }
    }
    (named, subtitles)
}

/// The module's crafting and cooking recipes each have a recipe book advancement rewarding them.
fn recipes(m: &Module, errors: &mut Vec<String>) {
    let Some(content) = m.manifest().map(|_| m.dir.clone()) else {
        return;
    };
    let Ok(namespaces) = std::fs::read_dir(content.join("data")) else {
        return;
    };
    for ns in namespaces.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        let name = ns.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_owned();
        let Ok(files) = std::fs::read_dir(ns.join("recipe")) else {
            continue;
        };
        let recipes: Vec<String> = files
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter_map(|p| Some(format!("{name}:{}", p.file_stem()?.to_str()?)))
            .collect();
        let mut advancements = Vec::new();
        walk(&ns.join("advancement/recipes"), &mut advancements);
        let rewarded: BTreeSet<String> = advancements
            .iter()
            .filter_map(|a| read_json(a).ok())
            .flat_map(|v| rewarded_recipes(&v))
            .collect();
        for recipe in missing_advancements(&recipes, &rewarded) {
            errors.push(format!(
                "{}: recipe {recipe} has no recipe book advancement (advancement/recipes/<tab>/, docs/CONVENTIONS.md)",
                rel(&ns)
            ));
        }
    }
}

/// The recipes an advancement rewards.
fn rewarded_recipes(advancement: &serde_json::Value) -> Vec<String> {
    advancement["rewards"]["recipes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.as_str().map(str::to_owned))
        .collect()
}

/// The `recipes` no advancement rewards.
fn missing_advancements<'a>(recipes: &'a [String], rewarded: &BTreeSet<String>) -> Vec<&'a String> {
    recipes.iter().filter(|r| !rewarded.contains(*r)).collect()
}

/// `en_us.json` and `fr_fr.json` have the same keys.
fn lang(dir: &Path, push: &mut impl FnMut(String)) {
    let keys = |name: &str| -> Option<BTreeSet<String>> {
        let v = read_json(&dir.join(name)).ok()?;
        Some(v.as_object()?.keys().cloned().collect())
    };
    match (keys("en_us.json"), keys("fr_fr.json")) {
        (Some(en), Some(fr)) => {
            for k in en.symmetric_difference(&fr) {
                let missing = if en.contains(k) { "fr_fr" } else { "en_us" };
                push(format!("{}: {k} is missing from {missing}.json", rel(dir)));
            }
        }
        _ => push(format!("{}: needs en_us.json and fr_fr.json", rel(dir))),
    }
}

fn manifest(path: &Path) -> crate::Result<toml::Table> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", rel(path)))?;
    toml::from_str(&text).map_err(|e| format!("{}: {e}", rel(path)))
}

fn read_json(path: &Path) -> crate::Result<serde_json::Value> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", rel(path)))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", rel(path)))
}

/// The repository's files, without `.git`, `target` and other hidden or build folders.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !name.starts_with('.') && name != "target" && name != "sounds-src" {
                walk(&path, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn rel(path: &Path) -> String {
    path.strip_prefix(crate::root()).unwrap_or(path).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_sounds_and_subtitles_of_sound_events() {
        let events = serde_json::json!({
            "entity.fairy.hurt": { "subtitle": "subtitles.entity.fairy.hurt", "sounds": ["fairies:fairies/hurt"] },
            "entity.fairy.ambient": { "sounds": [{ "name": "fairies:fairies/chime1", "volume": 0.5 }] }
        });
        let (named, subtitles) = sound_events(&events);
        assert_eq!(
            named.into_iter().collect::<Vec<_>>(),
            ["fairies:fairies/chime1", "fairies:fairies/hurt"]
        );
        assert_eq!(
            subtitles.into_iter().collect::<Vec<_>>(),
            ["subtitles.entity.fairy.hurt"]
        );
    }

    #[test]
    fn finds_recipes_with_no_advancement() {
        let advancement = serde_json::json!({ "rewards": { "recipes": ["cyclops:club"] } });
        let rewarded: BTreeSet<String> = rewarded_recipes(&advancement).into_iter().collect();
        let recipes = ["cyclops:club".to_owned(), "cyclops:shepherds_crook".to_owned()];
        assert_eq!(missing_advancements(&recipes, &rewarded), [&recipes[1]]);
    }
}
