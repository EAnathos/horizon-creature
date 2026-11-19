//! The bestiary, in the creature guide book (SPECS §7): what each player found out about the series' creatures.
//!
//! - Each mod registers its species and the facts a player can learn about them (`register`), from its `register`.
//! - A player **meets** a species when a wild creature of it comes within `MEET` blocks in one of the biomes it spawns
//!   in: its page opens, with that biome (the library's `entity_tick_post`, every `CHECK_EVERY` ticks per creature).
//! - A player **learns** a fact when they find it out in play: the mod calls `learn` (the vulture: raw chicken eaten,
//!   rotten flesh given). Nothing else is shown: no creature, biome or fact the player hasn't come across.
//! - For a species with looks (the library's colours and patterns), each **morph** met is noted too.
//! - It is kept in the player's persistent data (`creature:bestiary`), on the server. Right-click with the guide sends
//!   it to the player's client (`creature:bestiary` payload), which opens the book: a mod screen the library draws
//!   (`draw`, `clicked`). On the right page the species met, as icons; on the left the one picked: the creature turning
//!   in 3D, the biomes it was met in, what was found out, and its morphs met (each a swatch; picking one shows it).
//! - The 3D view draws the creature itself (`DrawContext::entity`: its own renderer, a vanilla creature's model too),
//!   in the look picked (`colours::set_preview`). `Species::draw` draws it in a placed jar.
//! - The right page lists the series' species first and, below a line, Minecraft's own (`minecraft:` ids, from the
//!   Vanilla Bestiary); the wheel scrolls it when they don't fit.
//! - Operators' commands, to check the pages (`perform`): `/bestiary unlock <species>` writes down everything about a
//!   species for the player who runs it (every biome it spawns in, every fact, every look), `/bestiary unlock all`
//!   about every species, `/bestiary reset` forgets everything.
mod book;
mod command;

pub use book::{clicked, draw, open, received, register_item, scrolled};
pub use command::{commands, complete, perform};

use std::collections::BTreeMap;
use std::sync::Mutex;

use std::sync::OnceLock;

use mod_api::{
    BlockEntityDrawContext, ClientHost, Command, CommandArgumentValue, CommandSource, Compound, DrawContext, GuiEntity,
    GuiEntityLook, GuiEntityView, Host, Identifier, ItemKey, ItemStack, PayloadCodec, PayloadId, Registrar,
    RunModCommandEvent, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, ScreenView, ServerHost, Suggestion,
    TabCompleteEvent, Tag, Text, ToClient,
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
    /// Draws its model in a placed jar, in `look` when it has looks: at the origin, the jar placing it (`draw_scale`,
    /// `draw_lift`).
    pub draw: fn(&mut dyn BlockEntityDrawContext, Option<Look>),
    /// Its model's scale to stand about a block across, and how high above the floor it stands (in blocks, before the
    /// jar's own shrinking).
    pub draw_scale: f32,
    pub draw_lift: f32,
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

/// From a mod's `register`: its species get pages in the book.
pub fn register(species: &'static Species) {
    let mut all = SPECIES.lock().unwrap();
    if !all.iter().any(|s| s.id == species.id) {
        all.push(species);
    }
}

/// The size of the creatures the book's view can't take by default (a ghast, the ender dragon, a vulture's wings), by
/// entity type: width and height, in blocks.
static VIEW_SIZES: Mutex<BTreeMap<&'static str, [f32; 2]>> = Mutex::new(BTreeMap::new());

/// From a mod's `register`: creature `kind` is `width` × `height` blocks, for the book's 3D view to fit it: a vanilla
/// one (the game has no read of a type's size), or one of ours whose model is bigger than its box (`crate::mob::ours`).
pub fn view_size(kind: &'static str, width: f32, height: f32) {
    VIEW_SIZES.lock().unwrap().insert(kind, [width, height]);
}

/// The size `view_size` gave `kind`.
pub(crate) fn view_size_of(kind: &str) -> Option<[f32; 2]> {
    VIEW_SIZES.lock().unwrap().get(kind).copied()
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
/// The book sent to a player's client: one chunk (`book::send_chunked`), its first character the chunk's flag.
#[derive(PayloadCodec)]
struct BookChunk {
    text: String,
}

static BOOK_PAYLOAD: OnceLock<ToClient<BookChunk>> = OnceLock::new();

/// From the library's `register`: the book's payload, received by the client (`book::received`).
pub fn declare(r: &mut dyn Registrar) {
    let mut payloads = r.payload_handlers();
    let mut view = payloads.registrar("1");
    let token = view.play_to_client_typed::<BookChunk>(PayloadId("creature:bestiary".into()), |chunk, cx| {
        let host = cx.client_host().map_err(|e| format!("{e:?}"))?;
        book::received(host, chunk.text.as_bytes());
        Ok(())
    });
    let _ = BOOK_PAYLOAD.set(token);
}

/// Sends one chunk of the book to `player`.
fn send(host: &mut dyn ServerHost, player: u128, packet: Vec<u8>) {
    let (Some(token), Some(player)) = (BOOK_PAYLOAD.get(), host.player(player)) else {
        return;
    };
    let chunk = BookChunk {
        text: String::from_utf8_lossy(&packet).into_owned(),
    };
    let _ = token.send(host, player, &chunk);
}
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
    let Some(data) = host.player(player).and_then(|p| p.persistent_data(host)) else {
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

fn store(host: &mut dyn ServerHost, player: u128, entries: &[Entry]) {
    let Some(p) = host.player(player) else {
        return;
    };
    let Some(mut data) = p.persistent_data(host) else {
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
    let _ = p.set_persistent_data(host, data);
}

/// Changes `player`'s page of `species` (opening it if needed), saved only when something changed.
fn update(host: &mut dyn ServerHost, player: u128, species: &str, change: impl FnOnce(&mut Entry)) {
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
pub fn learn(host: &mut dyn ServerHost, player: u128, species: &str, fact: &str) {
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
pub fn tick(host: &mut dyn ServerHost, dimension: &str, entity: u32, kind: &str) {
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
    if !(species.wild)(entity) {
        return;
    }
    let Some(e) = host.entity(entity) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let Some(biome) = level.biome_at(host, [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32]) else {
        return;
    };
    if !lives_in(species, Some(dimension), &biome) {
        return;
    }
    let near = players_near(&*host, dimension, at);
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
pub fn players_near(host: &dyn ServerHost, dimension: &str, at: [f64; 3]) -> Vec<u128> {
    host.players()
        .server_players()
        .into_iter()
        .filter(|p| p.dimension(host).is_some_and(|d| d.to_string() == dimension))
        .filter(|p| {
            p.entity()
                .position(host)
                .is_some_and(|q| crate::body::distance(q, at) <= MEET)
        })
        .map(|p| p.uuid())
        .collect()
}

/// `vulture:vulture` → `entity.vulture.vulture`.
fn entity_key(id: &str) -> String {
    format!("entity.{}", id.replacen(':', ".", 1))
}
