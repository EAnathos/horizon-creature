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
    /// Its Rust is on Sunrise's `mod_api`: built with its content. Until then only its content
    /// is packaged. Goes once every module is ported.
    #[serde(default)]
    ported: bool,
    /// Its page in Sunrise's registry: a module with one is published (`cargo creature release`).
    registry: Option<Registry>,
    /// The look its models' pictures are drawn in, by model (`cargo creature preview`).
    #[serde(default)]
    preview: std::collections::BTreeMap<String, Look>,
}

/// The look a model's pictures are drawn in, for a model the game colours itself (a greyscale texture tinted by a
/// morph, `creature::colours`): its base colour and, over it, a pattern texture in its own colour.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// The base colour the texture is multiplied by, `#rrggbb`.
    pub tint: String,
    /// A pattern texture over the base, by its path from the module's root.
    pub pattern: Option<String>,
    /// The pattern's colour, `#rrggbb` (white by default).
    pub pattern_tint: Option<String>,
}

/// A published module's page in Sunrise's registry (sunrise-org/sunrise-mods, `mods/<id>.toml`), besides what its
/// manifest says (name, authors, licence) and its README (the description).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// One line for the lists, at most 160 characters.
    pub summary: String,
    /// 1 to 3 of a mod's categories (`crate::release::CATEGORIES`).
    pub categories: Vec<String>,
    /// Up to 10 lowercase tags, each at most 24 characters.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Its icon: one of its models (its picture in `docs/models/`), or a PNG by its path from the repository's root.
    pub icon: String,
    /// Up to 8 pictures for its page, by their paths from the repository's root (PNG, JPEG, or WebP made a JPEG):
    /// the registry shows them; a page's text shows no image.
    #[serde(default)]
    pub screenshots: Vec<String>,
}

#[derive(Debug)]
pub struct Module {
    pub id: String,
    pub name: String,
    pub status: String,
    pub icon: Option<String>,
    pub creative: Vec<String>,
    pub requires: Vec<String>,
    pub ported: bool,
    pub registry: Option<Registry>,
    pub preview: std::collections::BTreeMap<String, Look>,
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
            ported: f.ported,
            registry: f.registry,
            preview: f.preview,
            dir: dir.to_path_buf(),
        })
    }

    /// Its package's manifest (`sunrise-mod.toml`, at the module's root), when it has one.
    pub fn manifest(&self) -> Option<PathBuf> {
        Some(self.dir.join("sunrise-mod.toml")).filter(|p| p.is_file())
    }

    /// Its Rust crate (`Cargo.toml` and `src/` at the module's root), when it is built: ported to `mod_api`.
    pub fn native(&self) -> Option<PathBuf> {
        Some(self.dir.clone()).filter(|d| self.ported && d.join("Cargo.toml").is_file())
    }

    /// Has something to build: a package.
    pub fn buildable(&self) -> bool {
        self.manifest().is_some()
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
