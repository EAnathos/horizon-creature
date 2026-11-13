//! Writes the series' single creative tab, `common/content/data/creature/horizon/creative_tab/creature.json`
//! (docs/ARCHITECTURE.md, "Creative inventory").
//!
//! A content mod can't add items to another mod's tab, but a tab may list items of other mods: the game leaves out
//! those nobody registers. So the library owns the one tab and lists the items of every module (`creative` in its
//! `module.toml`), in order (common/SPECS.md §3.3): every spawn egg first, then the library's items, then each mod's
//! other items; mods in alphabetical order (their folder names), each one's items in its own order. A mod that is not
//! installed simply has its items left out.
use std::path::PathBuf;

use crate::modules::{self, LIBRARY};

pub fn run() -> crate::Result {
    let (out, text, count) = render()?;
    std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}: {count} items", out.display());
    Ok(())
}

/// The tab file's path, its content and its number of items, without writing it.
pub fn render() -> crate::Result<(PathBuf, String, usize)> {
    let library = modules::library()?;
    let icon = library.icon.clone().ok_or("common/module.toml has no icon")?;
    let mut items = library.creative.clone();
    for m in modules::mods()? {
        for item in m.creative {
            if !item.starts_with(&format!("{}:", m.id)) {
                return Err(format!("{}: creative item {item} is not in its namespace", m.id));
            }
            items.push(item);
        }
    }
    // The spawn eggs first, keeping the order otherwise (a stable sort).
    items.sort_by_key(|item| !item.ends_with("_spawn_egg"));
    let json = serde_json::json!({ "title": format!("itemGroup.{LIBRARY}"), "icon": icon, "items": items });
    let out = library
        .dir
        .join(format!("content/data/{LIBRARY}/horizon/creative_tab/{LIBRARY}.json"));
    let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())? + "\n";
    Ok((out, text, items.len()))
}
