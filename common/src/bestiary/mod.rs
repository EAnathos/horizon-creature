//! The bestiary, in the creature guide book (SPECS §7): what each player found out about the series' creatures.
//!
//! - Each mod registers its species and the facts a player can learn about them (`register`), from its `hooks()`.
//! - A player **meets** a species when a wild creature of it comes within `MEET` blocks in one of the biomes it spawns
//!   in: its page opens, with that biome (the library's `entity_tick_post`, every `CHECK_EVERY` ticks per creature).
//! - A player **learns** a fact when they find it out in play: the mod calls `learn` (the vulture: raw chicken eaten,
//!   rotten flesh given). Nothing else is shown: no creature, biome or fact the player hasn't come across.
//! - For a species with looks (the library's colours and patterns), each **morph** met is noted too.
//! - It is kept in the player's persistent data (`creature:bestiary`), on the server. Right-click with the guide sends
//!   it to the player's client (`creature:bestiary` channel), which opens the book: a mod screen the library draws
//!   (`draw`, `clicked`). On the right page the species met, as icons; on the left the one picked: the creature turning
//!   in 3D, the biomes it was met in, what was found out, and its morphs met (each a swatch; picking one shows it).
//! - The 3D view is a scene (`Host::draw_scene`, as Create's Ponder) of a moss block and an invisible block (a barrier) on it with a block
//!   entity, whose drawing (`specimen`, from the library's `block_entity_submit`) is the species' own (`Species::draw`).
//!   A species with no model of ours (a vanilla creature, mods/vanilla_bestiary) draws its icon there (`draw_icon`).
//! - The right page lists the series' species first and, below a line, Minecraft's own (`minecraft:` ids, from the
//!   Vanilla Bestiary); the wheel scrolls it when they don't fit.
//! - Operators' commands, to check the pages (`perform`): `/bestiary unlock <species>` writes down everything about a
//!   species for the player who runs it (every biome it spawns in, every fact, every look), `/bestiary unlock all`
//!   about every species, `/bestiary reset` forgets everything.
mod book;
mod command;

pub use book::{clicked, draw, draw_icon, open, received, register_item, scrolled, specimen};
pub use command::{commands, complete, perform};

use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::containers::{ModBlockEntity, ModBlockEntityFields};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{
    BlockEntityDraw, BlockEntitySubmitEvent, BlockStateId, Compound, Host, NativeCommand, NativeSceneBlock,
    NativeSceneView, PerformCommandEvent, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, Suggestion,
    TabCompleteV2Event, Tag, Text,
};

use crate::colours::{self, Look, Scheme};
use crate::items::{self, Recipe, Station};
pub use crate::items::{ItemPage, Use};

/// A fact a player can learn about a species: an item drawn beside it and its text's language key (empty: a drop, its
/// item alone, on the lines below the other facts under the heading "Drops").
pub struct Fact {
    pub id: &'static str,
    pub item: &'static str,
    pub key: &'static str,
}

/// A species of the series, by its entity type, with what can be learnt about it.
pub struct Species {
    pub id: &'static str,
    /// The biomes it spawns in: only a creature met in one of them counts, and only a wild one (`wild`, by entity
    /// id): a tamed one following its owner elsewhere tells nothing about where the species lives. `@` and a
    /// dimension (`@minecraft:the_nether`) is any biome of that dimension, for creatures of structures and bosses.
    pub biomes: &'static [&'static str],
    pub wild: fn(u32) -> bool,
    pub facts: &'static [Fact],
    /// Its icon on the book's right page (an item: its spawn egg).
    pub icon: &'static str,
    /// Its looks, when it has some (the morphs met are noted and shown).
    pub scheme: Option<&'static Scheme>,
    /// How rare it is, shown on its page (indicative: how often a player comes across one).
    pub rarity: Rarity,
    /// Draws it in the book's 3D view, standing on the block's floor, centred on the block, about a block across (the
    /// mod scales its model to fit), in `look` when it has looks.
    pub draw: fn(&dyn Host, &mut BlockEntityDraw, Option<Look>),
}

/// How rare a species is, as its page shows it (in vanilla's rarity colours, darkened for the paper).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    VeryRare,
}

impl Rarity {
    fn key(self) -> &'static str {
        match self {
            Rarity::Common => "bestiary.creature.rarity.common",
            Rarity::Uncommon => "bestiary.creature.rarity.uncommon",
            Rarity::Rare => "bestiary.creature.rarity.rare",
            Rarity::VeryRare => "bestiary.creature.rarity.very_rare",
        }
    }
    fn colour(self) -> u32 {
        match self {
            Rarity::Common => 0xFF6B6455,
            Rarity::Uncommon => 0xFFA87C00,
            Rarity::Rare => 0xFF1E7FA0,
            Rarity::VeryRare => 0xFF9A2FA8,
        }
    }
}

static SPECIES: Mutex<Vec<&'static Species>> = Mutex::new(Vec::new());

/// From a mod's `hooks()`: its species get pages in the book.
pub fn register(species: &'static Species) {
    let mut all = SPECIES.lock().unwrap();
    if !all.iter().any(|s| s.id == species.id) {
        all.push(species);
    }
}

/// A registered species, by its entity type.
pub fn species_of(id: &str) -> Option<&'static Species> {
    species(id)
}

fn species(id: &str) -> Option<&'static Species> {
    SPECIES.lock().unwrap().iter().copied().find(|s| s.id == id)
}

/// Whether `species` spawns in `biome` of `dimension` (`None`: in any dimension it spawns in).
fn lives_in(species: &Species, dimension: Option<&str>, biome: &str) -> bool {
    species.biomes.iter().any(|b| match b.strip_prefix('@') {
        Some(d) => dimension.is_none_or(|dim| dim == d),
        None => *b == biome,
    })
}

/// Minecraft's own creatures, listed after the series' (the Vanilla Bestiary's).
fn is_vanilla(species: &str) -> bool {
    species.starts_with("minecraft:")
}

/// The key in the player's persistent data.
const KEY: &str = "creature:bestiary";
const CHANNEL: &str = "creature:bestiary";
/// A player meets a creature within this many blocks.
const MEET: f64 = 16.0;
const CHECK_EVERY: u32 = 40;

/// What one player knows of one species.
#[derive(Clone, Debug, Default, PartialEq)]
struct Entry {
    species: String,
    biomes: Vec<String>,
    facts: Vec<String>,
    /// The looks met (`colours::key`).
    morphs: Vec<String>,
}

fn strings(c: &Compound, key: &str) -> Vec<String> {
    match c.get(&key.into()) {
        Some(Tag::List(list)) => list
            .iter()
            .filter_map(|t| match t {
                Tag::String(s) => Some(s.to_string_lossy()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn string_list(items: &[String]) -> Tag {
    Tag::List(items.iter().map(|s| Tag::String(s.as_str().into())).collect())
}

/// The player's bestiary, in the order the species were met.
fn load(host: &dyn Host, player: u128) -> Vec<Entry> {
    let Some(Tag::Compound(data)) = host.player_persistent_data(player) else {
        return Vec::new();
    };
    let Some(Tag::List(list)) = data.get(&KEY.into()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|t| match t {
            Tag::Compound(c) => {
                let species_id = strings(c, "species").into_iter().next()?;
                let mut biomes = strings(c, "biomes");
                // Only the biomes it spawns in (older saves counted any biome, tamed creatures included).
                if let Some(s) = species(&species_id) {
                    biomes.retain(|b| lives_in(s, None, b));
                }
                Some(Entry {
                    species: species_id,
                    biomes,
                    facts: strings(c, "facts"),
                    morphs: strings(c, "morphs"),
                })
            }
            _ => None,
        })
        .collect()
}

fn store(host: &mut dyn Host, player: u128, entries: &[Entry]) {
    let Some(Tag::Compound(mut data)) = host.player_persistent_data(player) else {
        return;
    };
    let list = entries
        .iter()
        .map(|e| {
            let mut c = Compound::new();
            c.insert("species".into(), string_list(std::slice::from_ref(&e.species)));
            c.insert("biomes".into(), string_list(&e.biomes));
            c.insert("facts".into(), string_list(&e.facts));
            c.insert("morphs".into(), string_list(&e.morphs));
            Tag::Compound(c)
        })
        .collect();
    data.insert(KEY.into(), Tag::List(list));
    host.set_player_persistent_data(player, Tag::Compound(data));
}

/// Changes `player`'s page of `species` (opening it if needed), saved only when something changed.
fn update(host: &mut dyn Host, player: u128, species: &str, change: impl FnOnce(&mut Entry)) {
    let mut entries = load(host, player);
    let (index, new) = match entries.iter().position(|e| e.species == species) {
        Some(i) => (i, false),
        None => {
            entries.push(Entry {
                species: species.into(),
                ..Entry::default()
            });
            (entries.len() - 1, true)
        }
    };
    let before = entries[index].clone();
    change(&mut entries[index]);
    if new || entries[index] != before {
        store(host, player, &entries);
    }
}

/// From a mod: `player` found out `fact` about `species` (one of its registered facts).
pub fn learn(host: &mut dyn Host, player: u128, species: &str, fact: &str) {
    update(host, player, species, |e| {
        if !e.facts.iter().any(|f| f == fact) {
            e.facts.push(fact.into());
        }
    });
}

/// Ticks since each creature's last check.
static TICKS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// From the library's `entity_tick_post`: the players near a creature of a registered species meet it, in its biome.
/// Every entity ticks, and with the Vanilla Bestiary most kinds are species: the species is looked up only every
/// `CHECK_EVERY` ticks of an entity.
pub fn tick(host: &mut dyn Host, dimension: &str, entity: u32, kind: &str) {
    {
        let mut ticks = TICKS.lock().unwrap();
        let t = ticks.entry(entity).or_insert(entity % CHECK_EVERY);
        *t += 1;
        if *t < CHECK_EVERY {
            return;
        }
        *t = 0;
    }
    let Some(species) = species(kind) else {
        return;
    };
    if !(species.wild)(entity) || !crate::on(host, kind) {
        return;
    }
    let Some(at) = host.entity(entity).and_then(|e| host.entity_position(e)) else {
        return;
    };
    let Some(biome) = host.biome([at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32]) else {
        return;
    };
    if !lives_in(species, Some(dimension), &biome) {
        return;
    }
    let near: Vec<u128> = host
        .server_players()
        .into_iter()
        .filter(|p| p.dimension == dimension)
        .filter(|p| (0..3).map(|k| (p.position[k] - at[k]).powi(2)).sum::<f64>() <= MEET * MEET)
        .map(|p| p.uuid)
        .collect();
    if near.is_empty() {
        return;
    }
    let morph = species
        .scheme
        .and_then(|scheme| colours::look(entity).map(|look| colours::key(scheme, look)));
    for player in near {
        update(host, player, kind, |e| {
            if !e.biomes.contains(&biome) {
                e.biomes.push(biome.clone());
            }
            if let Some(m) = &morph
                && !e.morphs.contains(m)
            {
                e.morphs.push(m.clone());
            }
        });
    }
}

/// From the library's `entity_leave_level`: an entity gone is no longer counted.
pub fn forget(entity: u32) {
    TICKS.lock().unwrap().remove(&entity);
}

/// The players within `MEET` blocks of `at` in `dimension`, for a mod's facts seen happen (a creeper exploding).
pub fn players_near(host: &dyn Host, dimension: &str, at: [f64; 3]) -> Vec<u128> {
    host.server_players()
        .into_iter()
        .filter(|p| p.dimension == dimension)
        .filter(|p| (0..3).map(|k| (p.position[k] - at[k]).powi(2)).sum::<f64>() <= MEET * MEET)
        .map(|p| p.uuid)
        .collect()
}

/// `vulture:vulture` → `entity.vulture.vulture`.
fn entity_key(id: &str) -> String {
    format!("entity.{}", id.replacen(':', ".", 1))
}
