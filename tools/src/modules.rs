//! The series' modules: the library (`common/`) and the mods (`mods/<name>/`), each described by its `module.toml`
//! (docs/ARCHITECTURE.md). The file is the repository's, for these tools: it is not shipped in the mod.
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The library's id: the namespace of the shared content and the `creature` crate of the mods' native parts.
pub const LIBRARY: &str = "creature";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    id: String,
    name: String,
    /// `planned` (a spec only), `prototype`, `alpha`, `beta`, `release`.
    status: String,
    /// The library only: the item on the shared creative tab's button.
    icon: Option<String>,
    /// This module's items in the shared creative tab, in order.
    #[serde(default)]
    creative: Vec<String>,
    /// Other mods of the series this one requires, besides the library (docs/ARCHITECTURE.md §2).
    #[serde(default)]
    requires: Vec<String>,
}

#[derive(Debug)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub status: String,
    pub icon: Option<String>,
    pub creative: Vec<String>,
    pub requires: Vec<String>,
    pub dir: PathBuf,
}

impl Module {
    pub fn read(dir: &Path) -> crate::Result<Module> {
        let path = dir.join("module.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let f: File = toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Module {
            id: f.id,
            name: f.name,
            status: f.status,
            icon: f.icon,
            creative: f.creative,
            requires: f.requires,
            dir: dir.to_path_buf(),
        })
    }

    pub fn content(&self) -> Option<PathBuf> {
        Some(self.dir.join("content")).filter(|d| d.join("horizon-mod.toml").is_file())
    }

    pub fn native(&self) -> Option<PathBuf> {
        Some(self.dir.join("native")).filter(|d| d.join("horizon-mixin.toml").is_file())
    }

    /// Has something to build: a content part or a native part.
    pub fn buildable(&self) -> bool {
        self.content().is_some() || self.native().is_some()
    }
}

pub fn library() -> crate::Result<Module> {
    Module::read(&crate::root().join("common"))
}

/// The mods, by folder name.
pub fn mods() -> crate::Result<Vec<Module>> {
    let dir = crate::root().join("mods");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.join("module.toml").is_file())
        .collect();
    dirs.sort();
    dirs.iter().map(|d| Module::read(d)).collect()
}

/// `creature` (or `common`) is the library, anything else a mod's folder name.
pub fn find(name: &str) -> crate::Result<Module> {
    if name == LIBRARY || name == "common" {
        return library();
    }
    let dir = crate::root().join("mods").join(name);
    if !dir.join("module.toml").is_file() {
        return Err(format!(
            "no module {name}: neither the library ({LIBRARY}) nor a folder of mods/"
        ));
    }
    Module::read(&dir)
}

/// The mods named and, before each, the mods it requires (transitively), each once. A cycle is an error.
pub fn with_requirements(names: &[String]) -> crate::Result<Vec<Module>> {
    fn visit(name: &str, path: &mut Vec<String>, out: &mut Vec<Module>) -> crate::Result {
        if out.iter().any(|m| m.id == name) {
            return Ok(());
        }
        if path.iter().any(|p| p == name) {
            return Err(format!("mods require each other: {} -> {name}", path.join(" -> ")));
        }
        let m = find(name)?;
        if m.id == LIBRARY {
            return Ok(());
        }
        path.push(m.id.clone());
        for r in m.requires.clone() {
            visit(&r, path, out)?;
        }
        path.pop();
        out.push(m);
        Ok(())
    }
    let mut out = Vec::new();
    for name in names {
        visit(name, &mut Vec::new(), &mut out)?;
    }
    Ok(out)
}
