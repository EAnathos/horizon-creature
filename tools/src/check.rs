//! Checks the repository's consistency, without building anything (docs/CONVENTIONS.md). Run by CI.
//!
//! - The shared creative tab is up to date with the modules' `module.toml` (`cargo creature tab`).
//! - Each module: its content and native manifests carry its id and the same version; every native part is pinned to
//!   the library's Horizon revision, in `horizon-mixin.toml` and in `api/Cargo.toml` (and CI installs
//!   `cargo-horizon-mod` at it); a mod's native part depends on
//!   the library and on every mod its `module.toml` requires.
//! - A mod's tab items are declared items; its `requires` name mods of the series, without cycles.
//! - Every JSON file parses; each module's `en_us` and `fr_fr` lang files have the same keys.
//! - Generated files are up to date: each model's Rust (`cargo creature model`), and each model has its pictures in
//!   `docs/models/` and its entry in `docs/MODELS.md` (`cargo creature preview`).
//! - The mods that use the glass jar are the same in its recipe's condition and in the creative tab injection
//!   (`common/native/jar_tab.rs`).
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

    let library = modules::library()?;
    let mods = modules::mods()?;
    let revision = library
        .native()
        .and_then(|n| manifest(&n.join("horizon-mixin.toml")).ok())
        .and_then(|t| Some(t.get("compatibility")?.get("horizon_revision")?.as_str()?.to_owned()));
    let Some(revision) = revision else {
        return Err("common/native/horizon-mixin.toml pins no horizon_revision".into());
    };
    let ci = root.join(".github/workflows/ci.yml");
    if std::fs::read_to_string(&ci).is_ok_and(|t| !t.contains(&format!("HORIZON_REV: {revision}"))) {
        errors.push(format!(
            "{}: HORIZON_REV is not the series' revision {revision}",
            rel(&ci)
        ));
    }
    for m in std::iter::once(&library).chain(&mods) {
        module(m, &mods, &revision, &mut errors);
    }
    for m in std::iter::once(&library).chain(&mods) {
        models(m, &mut errors);
    }
    jar_users(&root, &mut errors);
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

fn module(m: &Module, mods: &[Module], revision: &str, errors: &mut Vec<String>) {
    let mut push = |e: String| errors.push(e);
    let mut versions = Vec::new();
    if let Some(content) = m.content() {
        let path = content.join("horizon-mod.toml");
        match manifest(&path) {
            Ok(t) => {
                if t.get("id").and_then(|v| v.as_str()) != Some(&m.id) {
                    push(format!("{}: id is not {}", rel(&path), m.id));
                }
                versions.extend(t.get("version").and_then(|v| v.as_str()).map(str::to_owned));
            }
            Err(e) => push(e),
        }
        lang(&content.join(format!("assets/{}/lang", m.id)), &mut push);
        for item in &m.creative {
            let name = item.split_once(':').map_or(item.as_str(), |(_, n)| n);
            let data = content.join(format!("data/{}/horizon", m.id));
            if !data.join(format!("item/{name}.json")).is_file() && !data.join(format!("block/{name}.json")).is_file() {
                push(format!(
                    "{}: creative item {item} is declared nowhere",
                    rel(&m.dir.join("module.toml"))
                ));
            }
        }
    } else if !m.creative.is_empty() {
        push(format!(
            "{}: creative items but no content part",
            rel(&m.dir.join("module.toml"))
        ));
    }
    if let Some(native) = m.native() {
        let path = native.join("horizon-mixin.toml");
        match manifest(&path) {
            Ok(t) => {
                let get = |table: &str, key: &str| t.get(table)?.get(key)?.as_str().map(str::to_owned);
                if get("mod", "id").as_deref() != Some(&m.id) {
                    push(format!("{}: [mod] id is not {}", rel(&path), m.id));
                }
                versions.extend(get("mod", "version"));
                if get("compatibility", "horizon_revision").as_deref() != Some(revision) {
                    push(format!(
                        "{}: not pinned to the series' Horizon revision {revision}",
                        rel(&path)
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
        let cargo = native.join("api/Cargo.toml");
        let text = std::fs::read_to_string(&cargo).unwrap_or_default();
        if !text.contains(&format!("rev = \"{revision}\"")) {
            push(format!(
                "{}: horizon-client is not at the series' revision {revision}",
                rel(&cargo)
            ));
        }
    }
    if versions.windows(2).any(|v| v[0] != v[1]) {
        push(format!(
            "{}: content and native versions differ ({})",
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

/// Each of the module's `.bbmodel` files: its Rust as `cargo creature model` makes it, its pictures and its entry in
/// `docs/MODELS.md`.
fn models(m: &Module, errors: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(m.dir.join("assets/models")) else {
        return;
    };
    let docs = crate::root().join("docs");
    let index = std::fs::read_to_string(docs.join("MODELS.md")).unwrap_or_default();
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
        if !pictures.iter().all(|p| p.is_file()) || !index.contains(&rel(&path)) {
            errors.push(format!(
                "{}: no pictures in docs/, run `cargo creature preview`",
                rel(&path)
            ));
        }
    }
}

/// The mods the glass jar's recipe waits for (`mod_loaded`) are the ones the creative tab injection keeps it for.
fn jar_users(root: &Path, errors: &mut Vec<String>) {
    let recipe = root.join("common/content/data/creature/recipe/glass_jar.json");
    let injection = root.join("common/native/jar_tab.rs");
    let mut in_recipe = BTreeSet::new();
    if let Ok(v) = read_json(&recipe) {
        collect_modids(&v, &mut in_recipe);
    }
    let text = std::fs::read_to_string(&injection).unwrap_or_default();
    let in_injection: BTreeSet<String> = text
        .lines()
        .find(|l| l.contains("const USERS"))
        .map(|l| {
            l.split('"')
                .skip(1)
                .step_by(2)
                .map(|s| s.trim_end_matches(':').to_owned())
                .collect()
        })
        .unwrap_or_default();
    if in_recipe.is_empty() || in_recipe != in_injection {
        errors.push(format!(
            "{} and {}: the jar's mods differ ({} / {})",
            rel(&recipe),
            rel(&injection),
            in_recipe.into_iter().collect::<Vec<_>>().join(", "),
            in_injection.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
}

fn collect_modids(v: &serde_json::Value, out: &mut BTreeSet<String>) {
    match v {
        serde_json::Value::Object(o) => {
            if o.get("type").and_then(|t| t.as_str()) == Some("neoforge:mod_loaded")
                && let Some(id) = o.get("modid").and_then(|i| i.as_str())
            {
                out.insert(id.to_owned());
            }
            o.values().for_each(|v| collect_modids(v, out));
        }
        serde_json::Value::Array(a) => a.iter().for_each(|v| collect_modids(v, out)),
        _ => {}
    }
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
