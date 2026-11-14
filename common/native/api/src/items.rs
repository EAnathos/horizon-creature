//! The creature guide's Items tab (SPECS §7.2): a page for each item of the series, saying how it is got and what it
//! is for.
//!
//! - Each module registers its items' pages (`creature::bestiary::register_item`): who drops it, who gives it, and its
//!   uses, a line each with an icon. The library registers its own (the guide, the glass jar).
//! - The recipes are the modules' own recipe files, read by `cargo creature tab` into `recipes.rs`: a page shows the
//!   recipes that make its item and the items it is used to make, so they can't differ from the game's.
//! - Every page is shown from the start, but only the items of installed mods: a mod registers only its own, and the
//!   glass jar's only shows when a mod uses the jar.
use std::sync::Mutex;

use crate::recipes::RECIPES;

/// What a station makes a recipe in, as its icon shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Station {
    Crafting,
    Furnace,
    Smoker,
    Campfire,
    Brewing,
}

impl Station {
    pub fn icon(self) -> &'static str {
        match self {
            Station::Crafting => "minecraft:crafting_table",
            Station::Furnace => "minecraft:furnace",
            Station::Smoker => "minecraft:smoker",
            Station::Campfire => "minecraft:campfire",
            Station::Brewing => "minecraft:brewing_stand",
        }
    }
}

/// One of the series' recipes (`recipes.rs`): a 3 × 3 grid, each slot the items that fit it (empty: nothing), and what
/// it makes. A cooking recipe's ingredient is in the middle slot, a brewing one's reagent above its potion.
pub struct Recipe {
    pub station: Station,
    pub grid: [&'static [&'static str]; 9],
    pub result: &'static str,
    pub count: u8,
}

/// A line of an item's page: an icon and its text's language key.
pub struct Use {
    pub icon: &'static str,
    pub key: &'static str,
}

/// An item's page, as its module registers it.
pub struct ItemPage {
    pub item: &'static str,
    /// The creatures that drop it (entity ids): "Dropped by" with their spawn egg.
    pub dropped_by: &'static [&'static str],
    /// Other ways to get it (a befriended fairy leaves fairy dust).
    pub given_by: &'static [Use],
    /// What it is for, besides the recipes it goes into.
    pub uses: &'static [Use],
}

static PAGES: Mutex<Vec<&'static ItemPage>> = Mutex::new(Vec::new());

/// From a module's `hooks()`: its item gets a page.
pub fn register(page: &'static ItemPage) {
    let mut pages = PAGES.lock().unwrap();
    if !pages.iter().any(|p| p.item == page.item) {
        pages.push(page);
    }
}

/// The pages shown, in the order registered (the library's first): the glass jar's only with a mod that uses it.
pub fn pages() -> Vec<&'static ItemPage> {
    let jar_used = crate::jar::used();
    PAGES
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|p| p.item != crate::jar::ITEM || jar_used)
        .collect()
}

/// The recipes that make `item`.
pub fn made_by(item: &str) -> Vec<&'static Recipe> {
    RECIPES.iter().filter(|r| r.result == item).collect()
}

/// What `item` is used to make, each once.
pub fn used_in(item: &str) -> Vec<&'static str> {
    let mut results: Vec<&'static str> = Vec::new();
    for r in RECIPES {
        if r.grid.iter().any(|slot| slot.contains(&item)) && !results.contains(&r.result) {
            results.push(r.result);
        }
    }
    results
}

/// `cyclops:club` → `item.cyclops.club`.
pub fn item_key(id: &str) -> String {
    format!("item.{}", id.replacen(':', ".", 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_recipes_of_an_item_and_what_it_makes() {
        assert!(made_by("cyclops:club").iter().all(|r| r.station == Station::Crafting));
        assert!(used_in("cyclops:cyclops_eye").contains(&"cyclops:shepherds_crook"));
        // A tag's items count: a vulture feather is a feather in the guide's recipe.
        assert!(used_in("vulture:feather").contains(&"creature:guide"));
        assert!(made_by("creature:nothing").is_empty());
    }
}
