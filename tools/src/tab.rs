//! Writes the series' single creative tab, `common/content/data/creature/horizon/creative_tab/creature.json`
//! (docs/ARCHITECTURE.md, "Creative inventory").
//!
//! A content mod can't add items to another mod's tab, but a tab may list items of other mods: the game leaves out
//! those nobody registers. So the library owns the one tab and lists the items of every module, in order: the
//! library's, then each mod's (`creative` in its `module.toml`), mods sorted by folder name. A mod that is not
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
    let json = serde_json::json!({ "title": format!("itemGroup.{LIBRARY}"), "icon": icon, "items": items });
    let out = library
        .dir
        .join(format!("content/data/{LIBRARY}/horizon/creative_tab/{LIBRARY}.json"));
    let text = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())? + "\n";
    Ok((out, text, items.len()))
}
