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
    if !(species.wild)(entity) {
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

/// The operators' command and its operator level.
const COMMAND: &str = "bestiary";
const COMMAND_LEVEL: u8 = 2;

/// From the library's `register_commands`.
pub fn commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::op(
        &format!("{COMMAND} unlock <species:text?>"),
        COMMAND_LEVEL,
    ));
    commands.push(NativeCommand::op(&format!("{COMMAND} reset"), COMMAND_LEVEL));
}

/// Every look `scheme` can roll, by key: the colours, patterns and pattern colours some variety gives a weight; a
/// pattern with no texture once, its pattern colour unseen.
fn every_morph(scheme: &Scheme) -> Vec<String> {
    let mut keys = Vec::new();
    for v in scheme.varieties {
        let given =
            |list: &[(usize, u32)]| -> Vec<usize> { list.iter().filter(|(_, w)| *w > 0).map(|(i, _)| *i).collect() };
        let pattern_colours = given(v.pattern_colours);
        for &base in &given(v.base) {
            for &pattern in &given(v.patterns) {
                let plain = scheme.patterns.get(pattern).is_none_or(|p| p.texture.is_none());
                let colours = if plain {
                    &pattern_colours[..pattern_colours.len().min(1)]
                } else {
                    &pattern_colours[..]
                };
                for &pattern_colour in colours {
                    let key = colours::key(
                        scheme,
                        Look {
                            base,
                            pattern,
                            pattern_colour,
                        },
                    );
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            }
        }
    }
    keys
}

/// Writes down everything about `species` for `player`: every biome it spawns in (not a whole dimension's, which has
/// no list), every fact and every look.
fn unlock(host: &mut dyn Host, player: u128, species: &Species) {
    let morphs = species.scheme.map(every_morph).unwrap_or_default();
    update(host, player, species.id, |e| {
        for b in species.biomes.iter().filter(|b| !b.starts_with('@')) {
            if !e.biomes.iter().any(|known| known == b) {
                e.biomes.push((*b).to_owned());
            }
        }
        for f in species.facts {
            if !e.facts.iter().any(|known| known == f.id) {
                e.facts.push(f.id.to_owned());
            }
        }
        for m in morphs {
            if !e.morphs.contains(&m) {
                e.morphs.push(m);
            }
        }
    });
}

/// The species a command names: its id (`minecraft:cow`), or the part after the colon when one species has it
/// (`cow`).
fn named(name: &str) -> Option<&'static Species> {
    let all = SPECIES.lock().unwrap();
    if let Some(s) = all.iter().find(|s| s.id == name) {
        return Some(s);
    }
    let mut short = all
        .iter()
        .filter(|s| s.id.split_once(':').is_some_and(|(_, path)| path == name));
    match (short.next(), short.next()) {
        (Some(s), None) => Some(s),
        _ => None,
    }
}

fn answer(host: &mut dyn Host, key: &str, args: &[Text]) {
    host.feedback(Text::translate(key, args));
}

/// From the library's `perform_command`: the operators' `/bestiary`, for the player who runs it. Whether it was ours.
pub fn perform(host: &mut dyn Host, event: &PerformCommandEvent<'_>) -> bool {
    let mut words = event.command.split_whitespace();
    if words.next() != Some(COMMAND) {
        return false;
    }
    let player = event.player.uuid;
    match (words.next(), words.next()) {
        (Some("reset"), None) => {
            store(host, player, &[]);
            answer(host, "commands.creature.bestiary.reset", &[]);
        }
        (Some("unlock"), Some("all")) => {
            let all: Vec<&'static Species> = SPECIES.lock().unwrap().clone();
            for s in &all {
                unlock(host, player, s);
            }
            answer(
                host,
                "commands.creature.bestiary.unlocked_all",
                &[Text::literal(all.len().to_string())],
            );
        }
        (Some("unlock"), Some(name)) => match named(name) {
            Some(s) => {
                unlock(host, player, s);
                answer(
                    host,
                    "commands.creature.bestiary.unlocked",
                    &[Text::translate(&entity_key(s.id), &[])],
                );
            }
            None => answer(host, "commands.creature.bestiary.unknown", &[Text::literal(name)]),
        },
        _ => answer(host, "commands.creature.bestiary.usage", &[]),
    }
    true
}

/// From the library's `tab_complete_v2`: `/bestiary unlock` completes `all` and the species' ids.
pub fn complete(event: &mut TabCompleteV2Event<'_>) {
    if event.label != COMMAND || event.arguments.len() != 2 || event.arguments[0] != "unlock" {
        return;
    }
    let typed = event.arguments[1].as_str();
    let ids: Vec<&'static str> = SPECIES.lock().unwrap().iter().map(|s| s.id).collect();
    for id in std::iter::once("all").chain(ids) {
        let short = id.split_once(':').map_or(id, |(_, path)| path);
        if id.starts_with(typed) || short.starts_with(typed) {
            event.suggestions.push(Suggestion {
                range: (event.range[0], event.range[1]),
                text: id.to_owned(),
                tooltip: None,
                int_value: None,
            });
        }
    }
}

/// From a module's `hooks()`: its item gets a page in the Items tab (`items`).
pub fn register_item(page: &'static ItemPage) {
    items::register(page);
}

/// From the library's `use_item`, the guide in hand: sends the acting player their bestiary, their client opens it.
pub fn open(host: &mut dyn Host) {
    let Some(player) = host.acting_player() else {
        return;
    };
    let text: String = load(host, player.uuid)
        .iter()
        .map(|e| {
            format!(
                "{}|{}|{}|{}\n",
                e.species,
                e.biomes.join(","),
                e.facts.join(","),
                e.morphs.join(",")
            )
        })
        .collect();
    let _ = host.net_send_to_player(player.uuid, CHANNEL, text.as_bytes());
}

/// The book's tabs, ribbons on its edge (SPECS §7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tab {
    Creatures,
    Items,
}

/// The book as the client got it: the tab open, the species and the item shown, and how far each list is scrolled.
struct Book {
    tab: Tab,
    entries: Vec<Entry>,
    selected: usize,
    /// The morph picked (in the sorted order) and the first swatch row seen.
    morph: usize,
    scroll: usize,
    /// The first row of the right page's icons seen, on each tab.
    grid_scroll: usize,
    item: usize,
    item_scroll: usize,
    /// The first row of the left page's details seen (biomes, facts, drops; an item's recipes and uses), and the last
    /// first row there can be (from the last frame drawn).
    detail_scroll: usize,
    detail_last: usize,
}

static BOOK: Mutex<Book> = Mutex::new(Book {
    tab: Tab::Creatures,
    entries: Vec::new(),
    selected: 0,
    morph: 0,
    scroll: 0,
    grid_scroll: 0,
    item: 0,
    item_scroll: 0,
    detail_scroll: 0,
    detail_last: 0,
});

/// The morphs `entry` met, sorted by colour, then pattern, then pattern colour (the scheme's order).
fn sorted_morphs(sp: &Species, entry: &Entry) -> Vec<Look> {
    let Some(scheme) = sp.scheme else {
        return Vec::new();
    };
    let mut morphs: Vec<Look> = entry
        .morphs
        .iter()
        .filter_map(|m| colours::from_key(scheme, m))
        .collect();
    morphs.sort_by_key(|m| (m.base, m.pattern, m.pattern_colour));
    morphs
}

/// The swatch grid's rows for `count` morphs.
fn rows(count: usize) -> usize {
    count.div_ceil(PER_LINE)
}

/// From the library's `net_from_server`: the bestiary arrived, the book opens (on the tab last read).
pub fn received(host: &mut dyn Host, channel: &str, data: &[u8]) {
    if channel != CHANNEL {
        return;
    }
    let list = |s: &str| -> Vec<String> { s.split(',').filter(|x| !x.is_empty()).map(String::from).collect() };
    let entries = String::from_utf8_lossy(data)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('|');
            Some(Entry {
                species: parts.next()?.into(),
                biomes: list(parts.next().unwrap_or("")),
                facts: list(parts.next().unwrap_or("")),
                morphs: list(parts.next().unwrap_or("")),
            })
        })
        .collect();
    {
        let mut book = BOOK.lock().unwrap();
        book.entries = entries;
        book.selected = book.selected.min(book.entries.len().saturating_sub(1));
        book.morph = 0;
        book.scroll = 0;
        book.grid_scroll = book.grid_scroll.min(grid_rows(&book.entries).saturating_sub(GRID_ROWS));
        book.detail_scroll = 0;
    }
    host.mod_screen_open(MOD_ID);
}

const MOD_ID: &str = "creature";

/// The book's size and layout, in GUI pixels.
const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 172.0;
const PAGE: f32 = 150.0;
const MARGIN: f32 = 10.0;
const LINE: f32 = 10.0;
/// The ribbons above the right page: their height and how far apart.
const RIBBON: f32 = 13.0;
const RIBBON_GAP: f32 = 4.0;
/// The right page's icons: where the grid starts, a cell's size, how many to a row.
const GRID_TOP: f32 = 36.0;
const CELL: f32 = 22.0;
const PER_ROW: usize = 5;
/// How many rows of icons the right page shows (the wheel scrolls the others).
const GRID_ROWS: usize = 5;
/// A row of icons on the left page (facts, drops, a line with its icon), and a recipe's 3 × 3 grid.
const ICON_ROW: f32 = 18.0;
const SLOT: f32 = 18.0;
const RECIPE_ROW: f32 = 3.0 * SLOT + 4.0;
/// The left page's 3D view: its box (from the page's corner) and how it is seen; an item's is shorter.
const VIEW: [f32; 4] = [MARGIN, 20.0, PAGE - 2.0 * MARGIN, 66.0];
const ITEM_VIEW_HEIGHT: f32 = 40.0;
const VIEW_WITH_LOOKS_HEIGHT: f32 = 52.0;
const VIEW_SCALE: f32 = 48.0;
const VIEW_PITCH: f32 = 22.0;
/// One turn of the creature, in milliseconds.
const SPIN: u128 = 12_000;
/// The morph swatches at the left page's bottom: a grid of `PER_LINE` a row, `ROWS_SHOWN` rows seen at once (the wheel
/// scrolls the others), the name of the one picked under it.
const SWATCH: f32 = 10.0;
const STEP: f32 = SWATCH + 3.0;
const PER_LINE: usize = 10;
const ROWS_SHOWN: usize = 2;
const SWATCHES_TOP: f32 = HEIGHT - MARGIN - LINE - ROWS_SHOWN as f32 * STEP + 1.0;

const COVER: u32 = 0xFF5A3A22;
const PAPER: u32 = 0xFFEFE4C8;
const FOLD: u32 = 0xFFD8C9A4;
const HIGHLIGHT: u32 = 0xFFDCCB9C;
const VIEW_BACK: u32 = 0xFFE6D9B8;
const SLOT_BACK: u32 = 0xFFE2D4B0;
/// A ribbon not open: the cover's leather, lighter.
const RIBBON_SHUT: u32 = 0xFF8A6440;
const INK: u32 = 0x3B2A1A;
/// The picked morph's outline: gold, seen around a black swatch as a white one.
const PICKED: u32 = 0xFFE0A030;
const FADED: u32 = 0x7A6650;

fn tr(key: &str) -> String {
    Text::translate(key, &[]).plain()
}

/// The book's top left corner, centred on the screen.
fn origin(rect: [f32; 4]) -> [f32; 2] {
    [rect[0] + (rect[2] - WIDTH) / 2.0, rect[1] + (rect[3] - HEIGHT) / 2.0]
}

fn is_mine(host: &dyn Host, id: &str) -> bool {
    id == "mod" && host.mod_screen() == Some(MOD_ID)
}

/// `text` cut into lines at most `width` wide.
fn wrap(host: &dyn Host, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if host.text_width(&candidate) > width && !line.is_empty() {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// What the 3D view draws this frame, for `specimen` (`draw_scene` asks for it within the call).
#[derive(Clone, Copy)]
enum Subject {
    Creature(&'static Species, Option<Look>),
    Item(&'static str),
}

static SUBJECT: Mutex<Option<Subject>> = Mutex::new(None);

/// The states of the scene's blocks: a barrier (invisible) to carry the block entity, and a moss block under it (grass would
/// need a biome for its green).
fn scene_states(host: &dyn Host) -> Option<(BlockStateId, BlockStateId)> {
    static STATES: Mutex<Option<(BlockStateId, BlockStateId)>> = Mutex::new(None);
    let mut cached = STATES.lock().unwrap();
    if cached.is_none() {
        let states = host.block_states();
        let find = |name: &str| states.iter().find(|(_, f)| &*f.name == name).map(|(id, _)| id);
        *cached = find("minecraft:barrier").zip(find("minecraft:moss_block"));
    }
    *cached
}

/// From the library's `block_entity_submit`: in the book's scene, draws the creature or the item shown.
pub fn specimen(host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
    let subject = *SUBJECT.lock().unwrap();
    match subject {
        Some(Subject::Creature(species, look)) => (species.draw)(host, &mut *event.draw, look),
        Some(Subject::Item(item)) => draw_item(host, &mut *event.draw, item),
        None => {}
    }
}

/// `item`, large, standing on the moss as an item in a frame.
fn draw_item(host: &dyn Host, draw: &mut BlockEntityDraw, item: &str) {
    draw.push();
    draw.translate([0.5, 0.5, 0.5]);
    draw.scale([0.9; 3]);
    host.draw_item(
        draw,
        &ItemStack::new(item, 1),
        horizon_client::mod_api::DisplayContext::Fixed,
        None,
        0,
    );
    draw.pop();
}

/// A `Species::draw` for a species with no model of ours (a vanilla creature): its icon, large, standing on the moss
/// as an item in a frame.
pub fn draw_icon(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<Look>) {
    let subject = *SUBJECT.lock().unwrap();
    if let Some(Subject::Creature(species, _)) = subject {
        draw_item(host, draw, species.icon);
    }
}

/// Draws `subject`, turning, in the box `rect`.
fn view(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, rect: [f32; 4], subject: Subject) {
    let Some((barrier, moss)) = scene_states(host) else {
        return;
    };
    let entity = ModBlockEntity::new(ModBlockEntityFields {
        id: "creature:specimen".into(),
        ..ModBlockEntityFields::default()
    });
    let yaw = (millis() % SPIN) as f32 / SPIN as f32 * 360.0;
    *SUBJECT.lock().unwrap() = Some(subject);
    // Seen smaller in a shorter box.
    let scale = VIEW_SCALE * rect[3] / VIEW[3];
    // The scene's box is its two corners, not a corner and a size.
    host.draw_scene(
        batch,
        [rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]],
        &[
            NativeSceneBlock {
                pos: [0, 0, 0],
                state: barrier,
                entity: Some(&entity),
            },
            NativeSceneBlock {
                pos: [0, -1, 0],
                state: moss,
                entity: None,
            },
        ],
        NativeSceneView {
            center: [0.5, 0.25, 0.5],
            yaw,
            pitch: VIEW_PITCH,
            scale,
        },
    );
    *SUBJECT.lock().unwrap() = None;
}

fn millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// The right page's icons: each entry's index with its row and column, the series' first, then, on a new row, the
/// vanilla ones, each part in the order met. The first vanilla row, if any.
fn grid(entries: &[Entry]) -> (Vec<(usize, usize, usize)>, Option<usize>) {
    let series = entries.iter().enumerate().filter(|(_, e)| !is_vanilla(&e.species));
    let vanilla = entries.iter().enumerate().filter(|(_, e)| is_vanilla(&e.species));
    let mut cells: Vec<(usize, usize, usize)> = series
        .enumerate()
        .map(|(k, (i, _))| (i, k / PER_ROW, k % PER_ROW))
        .collect();
    let first = cells.last().map_or(0, |c| c.1 + 1);
    let mut line = None;
    for (k, (i, _)) in vanilla.enumerate() {
        line.get_or_insert(first);
        cells.push((i, first + k / PER_ROW, k % PER_ROW));
    }
    (cells, line)
}

/// How many rows the right page's icons take.
fn grid_rows(entries: &[Entry]) -> usize {
    grid(entries).0.last().map_or(0, |c| c.1 + 1)
}

/// The items' icons on the right page: each page's index with its row and column.
fn item_grid(count: usize) -> Vec<(usize, usize, usize)> {
    (0..count).map(|k| (k, k / PER_ROW, k % PER_ROW)).collect()
}

/// The top left corner of the right page's icon at `row` and `col`, `scroll` rows scrolled: none when out of sight.
fn cell(x: f32, y: f32, row: usize, col: usize, scroll: usize) -> Option<[f32; 2]> {
    let shown = row.checked_sub(scroll).filter(|r| *r < GRID_ROWS)?;
    Some([
        x + PAGE + MARGIN + col as f32 * CELL,
        y + GRID_TOP + shown as f32 * CELL,
    ])
}

/// The ribbons above the right page: each tab's box and its label.
fn ribbons(host: &dyn Host, x: f32, y: f32) -> [(Tab, [f32; 4], String); 2] {
    let creatures = tr("bestiary.creature.tab.creatures");
    let items = tr("bestiary.creature.tab.items");
    let w1 = host.text_width(&creatures) + 8.0;
    let w2 = host.text_width(&items) + 8.0;
    let left = x + PAGE + MARGIN;
    [
        (Tab::Creatures, [left, y - 4.0 - RIBBON, w1, RIBBON], creatures),
        (
            Tab::Items,
            [left + w1 + RIBBON_GAP, y - 4.0 - RIBBON, w2, RIBBON],
            items,
        ),
    ]
}

fn inside(m: [f32; 2], r: [f32; 4]) -> bool {
    m[0] >= r[0] && m[0] < r[0] + r[2] && m[1] >= r[1] && m[1] < r[1] + r[3]
}

/// A row of the left page's details, which scroll as a whole when they don't fit.
enum Row {
    /// A line of text, its colour, how far in.
    Text(String, u32, f32),
    /// Items with their word, side by side.
    Facts(Vec<(&'static str, String)>),
    /// Items alone, side by side (drops, what an item makes).
    Icons(Vec<&'static str>),
    /// An item and a line of text beside it, and the lines after it when the text wraps.
    Line(&'static str, String),
    More(String),
    /// A recipe, with every station that makes it from the same grid (a furnace, a smoker, a campfire).
    Recipe(&'static Recipe, Vec<Station>),
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Text(..) | Row::More(_) => LINE,
            Row::Recipe(..) => RECIPE_ROW,
            _ => ICON_ROW,
        }
    }

    fn draw(&self, host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, px: f32, at: f32) {
        match self {
            Row::Text(text, colour, indent) => host.text(batch, text, [px + indent, at], *colour, false),
            Row::Facts(facts) => {
                let mut fx = px;
                for (item, label) in facts {
                    host.gui_draw_item(batch, &ItemStack::new(item, 1), [fx, at]);
                    host.text(batch, label, [fx + 18.0, at + 4.0], INK, false);
                    fx += 24.0 + host.text_width(label);
                }
            }
            Row::Icons(items) => {
                for (k, item) in items.iter().enumerate() {
                    host.gui_draw_item(batch, &ItemStack::new(item, 1), [px + k as f32 * 18.0, at]);
                }
            }
            Row::Line(item, text) => {
                host.gui_draw_item(batch, &ItemStack::new(item, 1), [px, at]);
                host.text(batch, text, [px + 18.0, at + 4.0], INK, false);
            }
            // Right under the line it follows, whose row is an icon high.
            Row::More(text) => host.text(batch, text, [px + 18.0, at + 4.0 - (ICON_ROW - LINE)], INK, false),
            Row::Recipe(r, stations) => draw_recipe(host, batch, r, stations, px, at),
        }
    }
}

/// A recipe: its 3 × 3 grid (a slot with several items shows each in turn), its stations, an arrow, what it makes.
fn draw_recipe(
    host: &dyn Host,
    batch: &mut horizon_client::mod_api::HudBatch,
    r: &Recipe,
    stations: &[Station],
    px: f32,
    at: f32,
) {
    let turn = (millis() / 1000) as usize;
    for (k, slot) in r.grid.iter().enumerate() {
        let [sx, sy] = [px + (k % 3) as f32 * SLOT, at + (k / 3) as f32 * SLOT];
        host.fill(batch, [sx, sy, SLOT - 1.0, SLOT - 1.0], SLOT_BACK);
        if !slot.is_empty() {
            host.gui_draw_item(batch, &ItemStack::new(slot[turn % slot.len()], 1), [sx, sy]);
        }
    }
    let ax = px + 3.0 * SLOT + 4.0;
    for (k, station) in stations.iter().enumerate() {
        host.gui_draw_item(
            batch,
            &ItemStack::new(station.icon(), 1),
            [ax + 2.0 + k as f32 * 18.0, at],
        );
    }
    host.text(batch, "->", [ax + 4.0, at + SLOT + 5.0], FADED, false);
    let rx = ax + 22.0;
    host.fill(batch, [rx - 1.0, at + SLOT - 1.0, SLOT, SLOT], SLOT_BACK);
    let count = (r.count > 1).then(|| r.count.to_string());
    host.gui_draw_item_with(batch, &ItemStack::new(r.result, 1), [rx, at + SLOT], count.as_deref());
}

/// Wraps items with their word into rows `width` wide.
fn fact_rows(host: &dyn Host, facts: Vec<(&'static str, String)>, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut row: Vec<(&'static str, String)> = Vec::new();
    let mut used = 0.0;
    for (item, label) in facts {
        let w = 24.0 + host.text_width(&label);
        if !row.is_empty() && used + w > width + 6.0 {
            rows.push(Row::Facts(std::mem::take(&mut row)));
            used = 0.0;
        }
        used += w;
        row.push((item, label));
    }
    if !row.is_empty() {
        rows.push(Row::Facts(row));
    }
    rows
}

/// Items alone, as many to a row as `width` holds.
fn icon_rows(items: Vec<&'static str>, width: f32) -> Vec<Row> {
    let per_row = (((width + 2.0) / 18.0) as usize).max(1);
    items.chunks(per_row).map(|c| Row::Icons(c.to_vec())).collect()
}

/// A line with its icon, the text wrapped beside it.
fn line_rows(host: &dyn Host, icon: &'static str, text: &str, width: f32) -> Vec<Row> {
    let mut lines = wrap(host, text, width - 18.0).into_iter();
    let mut rows = vec![Row::Line(icon, lines.next().unwrap_or_default())];
    rows.extend(lines.map(Row::More));
    rows
}

/// A species' details: the biomes it was met in, what was found out, then its drops under their heading.
fn creature_rows(host: &dyn Host, sp: &Species, entry: &Entry, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    if !entry.biomes.is_empty() {
        let biomes: Vec<String> = entry.biomes.iter().map(|b| tr(&biome_key(b))).collect();
        rows.extend(
            wrap(host, &biomes.join(", "), width)
                .into_iter()
                .map(|l| Row::Text(l, FADED, 0.0)),
        );
    }
    let known = |f: &&Fact| entry.facts.iter().any(|k| k == f.id);
    let (drops, facts): (Vec<&Fact>, Vec<&Fact>) = sp.facts.iter().filter(known).partition(|f| f.key.is_empty());
    rows.extend(fact_rows(
        host,
        facts.iter().map(|f| (f.item, tr(f.key))).collect(),
        width,
    ));
    if !drops.is_empty() {
        rows.push(Row::Text(tr("bestiary.creature.drops"), FADED, 0.0));
        rows.extend(icon_rows(drops.iter().map(|f| f.item).collect(), width));
    }
    rows
}

/// An item's details: how it is got (its recipes, who drops it, who gives it), then its uses and what it makes.
fn item_rows(host: &dyn Host, page: &ItemPage, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let made = items::made_by(page.item);
    if !made.is_empty() || !page.dropped_by.is_empty() || !page.given_by.is_empty() {
        rows.push(Row::Text(tr("bestiary.creature.item.get"), FADED, 0.0));
    }
    // Recipes from the same grid, in other stations, share one grid.
    let mut recipes: Vec<(&'static Recipe, Vec<Station>)> = Vec::new();
    for r in made {
        match recipes.iter_mut().find(|(o, _)| o.grid == r.grid && o.count == r.count) {
            Some((_, stations)) => stations.push(r.station),
            None => recipes.push((r, vec![r.station])),
        }
    }
    rows.extend(recipes.into_iter().map(|(r, stations)| Row::Recipe(r, stations)));
    for creature in page.dropped_by {
        let icon = species(creature).map_or_else(|| egg(creature), |s| s.icon);
        let text = Text::translate(
            "bestiary.creature.item.dropped_by",
            &[Text::translate(&entity_key(creature), &[])],
        )
        .plain();
        rows.extend(line_rows(host, icon, &text, width));
    }
    for given in page.given_by {
        rows.extend(line_rows(host, given.icon, &tr(given.key), width));
    }
    let used_in = items::used_in(page.item);
    if !page.uses.is_empty() || !used_in.is_empty() {
        rows.push(Row::Text(tr("bestiary.creature.item.uses"), FADED, 0.0));
    }
    for u in page.uses {
        rows.extend(line_rows(host, u.icon, &tr(u.key), width));
    }
    if !used_in.is_empty() {
        rows.push(Row::Text(tr("bestiary.creature.item.makes"), FADED, 0.0));
        rows.extend(icon_rows(used_in, width));
    }
    rows
}

/// `fairies:fairy` → `fairies:fairy_spawn_egg`, for a creature with no page.
fn egg(creature: &str) -> &'static str {
    Box::leak(format!("{creature}_spawn_egg").into_boxed_str())
}

/// Draws `rows` from the `scroll`th on, in the band from `top` to `bottom`, with a scrollbar when they don't all fit.
/// The last first row there can be, for the wheel.
fn draw_rows(
    host: &dyn Host,
    batch: &mut horizon_client::mod_api::HudBatch,
    rows: &[Row],
    px: f32,
    [top, bottom]: [f32; 2],
    scroll: usize,
) -> usize {
    // The last first row: from it to the end, they all fit.
    let mut last = rows.len();
    let mut fill = 0.0;
    while last > 0 && fill + rows[last - 1].height() <= bottom - top {
        last -= 1;
        fill += rows[last].height();
    }
    let scroll = scroll.min(last);
    let mut at = top;
    for row in &rows[scroll..] {
        if at + row.height() > bottom {
            break;
        }
        row.draw(host, batch, px, at);
        at += row.height();
    }
    if last > 0 {
        let (bx, bh) = (px + PAGE - 2.0 * MARGIN + 4.0, bottom - top);
        host.fill(batch, [bx, top, 2.0, bh], FOLD);
        let thumb = (bh / (last + 1) as f32).max(6.0);
        let at = top + (bh - thumb) * scroll as f32 / last as f32;
        host.fill(batch, [bx, at, 2.0, thumb], 0xFF00_0000 | FADED);
    }
    last
}

/// From the library's `screen_render_post`: draws the open book.
pub fn draw(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    if !is_mine(host, event.screen.id) {
        return;
    }
    let batch = &mut *event.batch;
    let [x, y] = origin(event.screen.rect);
    let mut book = BOOK.lock().unwrap();
    // The ribbons, the open one in the paper's colour; the cover around two pages and the fold between them.
    for (tab, r, label) in ribbons(host, x, y) {
        let open = tab == book.tab;
        host.fill(
            batch,
            [r[0], r[1], r[2], r[3] + 4.0],
            if open { PAPER } else { RIBBON_SHUT },
        );
        let colour = if open { INK } else { 0xEFE4C8 };
        host.text(batch, &label, [r[0] + 4.0, r[1] + 3.0], colour, false);
    }
    host.fill(batch, [x - 4.0, y - 4.0, WIDTH + 8.0, HEIGHT + 8.0], COVER);
    host.fill(batch, [x, y, WIDTH, HEIGHT], PAPER);
    host.fill(batch, [x + PAGE - 1.0, y, 2.0, HEIGHT], FOLD);
    match book.tab {
        Tab::Creatures => draw_creatures(host, batch, &mut book, x, y),
        Tab::Items => draw_items(host, batch, &mut book, x, y),
    }
}

/// The right page's scrollbar, when there are more rows of icons than shown.
fn grid_scrollbar(
    host: &dyn Host,
    batch: &mut horizon_client::mod_api::HudBatch,
    x: f32,
    y: f32,
    total: usize,
    scroll: usize,
) {
    if total > GRID_ROWS {
        let (bx, by, bh) = (x + WIDTH - 5.0, y + GRID_TOP, GRID_ROWS as f32 * CELL - 2.0);
        host.fill(batch, [bx, by, 2.0, bh], FOLD);
        let thumb = bh * GRID_ROWS as f32 / total as f32;
        let top = by + (bh - thumb) * scroll as f32 / (total - GRID_ROWS) as f32;
        host.fill(batch, [bx, top, 2.0, thumb], 0xFF00_0000 | FADED);
    }
}

/// The Creatures tab: the species met on the right, the one picked on the left.
fn draw_creatures(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, book: &mut Book, x: f32, y: f32) {
    let rx = x + PAGE + MARGIN;
    host.text(batch, &tr("bestiary.creature.title"), [rx, y + MARGIN], INK, false);
    if book.entries.is_empty() {
        for (k, line) in wrap(host, &tr("bestiary.creature.empty"), PAGE - 2.0 * MARGIN)
            .iter()
            .enumerate()
        {
            host.text(batch, line, [rx, y + GRID_TOP + k as f32 * LINE], FADED, false);
        }
        return;
    }
    let met = format!("{}: {}", tr("bestiary.creature.met"), book.entries.len());
    host.text(batch, &met, [rx, y + MARGIN + 12.0], FADED, false);
    let (cells, line) = grid(&book.entries);
    for &(k, row, col) in &cells {
        let Some([cx, cy]) = cell(x, y, row, col, book.grid_scroll) else {
            continue;
        };
        if k == book.selected {
            host.fill(batch, [cx - 1.0, cy - 1.0, CELL - 2.0, CELL - 2.0], HIGHLIGHT);
        }
        if let Some(s) = species(&book.entries[k].species) {
            host.gui_draw_item(batch, &ItemStack::new(s.icon, 1), [cx + 1.0, cy + 1.0]);
        }
    }
    // The line above Minecraft's own creatures, when the series' are above it.
    if let Some(row) = line.filter(|r| *r > 0)
        && let Some([_, ly]) = cell(x, y, row, 0, book.grid_scroll)
    {
        host.fill(batch, [rx, ly - 2.0, PAGE - 2.0 * MARGIN, 1.0], FOLD);
    }
    grid_scrollbar(host, batch, x, y, grid_rows(&book.entries), book.grid_scroll);
    // Left page: the creature picked.
    let Some(entry) = book.entries.get(book.selected) else {
        return;
    };
    let Some(sp) = species(&entry.species) else {
        return;
    };
    let (px, width) = (x + MARGIN, PAGE - 2.0 * MARGIN);
    host.text(batch, &tr(&entity_key(&entry.species)), [px, y + 8.0], INK, false);
    // How rare it is, on the right of its name.
    let rarity = tr(sp.rarity.key());
    let rx_end = x + PAGE - MARGIN - host.text_width(&rarity);
    host.text(batch, &rarity, [rx_end, y + 8.0], sp.rarity.colour(), false);
    let morphs = sorted_morphs(sp, entry);
    let look = sp.scheme.map(|_| morphs.get(book.morph).copied().unwrap_or_default());
    // A species with looks has its swatches below: its view is shorter, to leave its details room.
    let view_height = if sp.scheme.is_some() {
        VIEW_WITH_LOOKS_HEIGHT
    } else {
        VIEW[3]
    };
    let view_box = [x + VIEW[0], y + VIEW[1], VIEW[2], view_height];
    host.fill(batch, view_box, VIEW_BACK);
    view(host, batch, view_box, Subject::Creature(sp, look));
    // Its biomes, facts and drops, scrolled by the wheel above the swatches.
    let details = creature_rows(host, sp, entry, width);
    let top = y + VIEW[1] + view_height + 4.0;
    let bottom = if sp.scheme.is_some() {
        y + SWATCHES_TOP - 3.0
    } else {
        y + HEIGHT - MARGIN + 4.0
    };
    book.detail_last = draw_rows(host, batch, &details, px, [top, bottom], book.detail_scroll);
    // Its morphs met: a swatch each (its colour, the pattern's colour in the middle); the one picked is outlined.
    if let Some(scheme) = sp.scheme {
        let rgb = |i: usize| 0xFF00_0000 | scheme.colours.get(i).map_or(0xFFFFFF, |c| c.rgb);
        let first = book.scroll * PER_LINE;
        for (k, m) in morphs.iter().enumerate().skip(first).take(ROWS_SHOWN * PER_LINE) {
            let shown = k - first;
            let sx = px + (shown % PER_LINE) as f32 * STEP;
            let sy = y + SWATCHES_TOP + (shown / PER_LINE) as f32 * STEP;
            if k == book.morph {
                host.fill(batch, [sx - 1.0, sy - 1.0, SWATCH + 2.0, SWATCH + 2.0], PICKED);
            }
            host.fill(batch, [sx, sy, SWATCH, SWATCH], rgb(m.base));
            if scheme.patterns.get(m.pattern).is_some_and(|p| p.texture.is_some()) {
                host.fill(
                    batch,
                    [sx + 3.0, sy + 3.0, SWATCH - 6.0, SWATCH - 6.0],
                    rgb(m.pattern_colour),
                );
            }
        }
        if let Some(m) = morphs.get(book.morph) {
            let ns = entry.species.split(':').next().unwrap_or("");
            let name = |kind: &str, id: &str| tr(&format!("look.{ns}.{kind}.{id}"));
            let pattern = scheme.patterns.get(m.pattern);
            let mut text = name("colour", scheme.colours[m.base].id);
            if let Some(p) = pattern.filter(|p| p.texture.is_some()) {
                text = format!(
                    "{text}, {} ({})",
                    name("pattern", p.id),
                    name("colour", scheme.colours[m.pattern_colour].id)
                );
            }
            host.text(
                batch,
                &text,
                [px, y + SWATCHES_TOP + ROWS_SHOWN as f32 * STEP],
                FADED,
                false,
            );
        }
        // More rows than shown: a thin scrollbar beside the grid.
        let total = rows(morphs.len());
        if total > ROWS_SHOWN {
            let (bx, by, bh) = (
                px + PER_LINE as f32 * STEP,
                y + SWATCHES_TOP,
                ROWS_SHOWN as f32 * STEP - 3.0,
            );
            host.fill(batch, [bx, by, 2.0, bh], FOLD);
            let thumb = bh * ROWS_SHOWN as f32 / total as f32;
            let top = by + (bh - thumb) * book.scroll as f32 / (total - ROWS_SHOWN) as f32;
            host.fill(batch, [bx, top, 2.0, thumb], 0xFF00_0000 | FADED);
        }
    }
}

/// The Items tab: the series' items on the right, the one picked on the left.
fn draw_items(host: &dyn Host, batch: &mut horizon_client::mod_api::HudBatch, book: &mut Book, x: f32, y: f32) {
    let rx = x + PAGE + MARGIN;
    host.text(batch, &tr("bestiary.creature.items"), [rx, y + MARGIN], INK, false);
    let pages = items::pages();
    let count = format!("{}: {}", tr("bestiary.creature.items_count"), pages.len());
    host.text(batch, &count, [rx, y + MARGIN + 12.0], FADED, false);
    for (k, row, col) in item_grid(pages.len()) {
        let Some([cx, cy]) = cell(x, y, row, col, book.item_scroll) else {
            continue;
        };
        if k == book.item {
            host.fill(batch, [cx - 1.0, cy - 1.0, CELL - 2.0, CELL - 2.0], HIGHLIGHT);
        }
        host.gui_draw_item(batch, &ItemStack::new(pages[k].item, 1), [cx + 1.0, cy + 1.0]);
    }
    grid_scrollbar(host, batch, x, y, pages.len().div_ceil(PER_ROW), book.item_scroll);
    // Left page: the item picked, turning, then how it is got and what it is for.
    let Some(page) = pages.get(book.item) else {
        return;
    };
    let (px, width) = (x + MARGIN, PAGE - 2.0 * MARGIN);
    host.text(batch, &tr(&items::item_key(page.item)), [px, y + 8.0], INK, false);
    let view_box = [x + VIEW[0], y + VIEW[1], VIEW[2], ITEM_VIEW_HEIGHT];
    host.fill(batch, view_box, VIEW_BACK);
    view(host, batch, view_box, Subject::Item(page.item));
    let details = item_rows(host, page, width);
    let top = y + VIEW[1] + ITEM_VIEW_HEIGHT + 4.0;
    book.detail_last = draw_rows(
        host,
        batch,
        &details,
        px,
        [top, y + HEIGHT - MARGIN + 4.0],
        book.detail_scroll,
    );
}

/// From the library's `screen_mouse_button_pressed_pre`: a ribbon opens its tab, an icon its page, a swatch shows its
/// morph.
pub fn clicked(host: &dyn Host, event: &ScreenMouseButtonPressedPreEvent<'_>) -> bool {
    if !is_mine(host, event.screen.id) || event.button != 0 {
        return false;
    }
    let [x, y] = origin(event.screen.rect);
    let mouse = event.screen.mouse;
    let [mx, my] = mouse;
    let mut book = BOOK.lock().unwrap();
    for (tab, r, _) in ribbons(host, x, y) {
        if inside(mouse, r) {
            if book.tab != tab {
                book.tab = tab;
                book.detail_scroll = 0;
            }
            return true;
        }
    }
    if book.tab == Tab::Items {
        for (k, row, col) in item_grid(items::pages().len()) {
            if let Some([cx, cy]) = cell(x, y, row, col, book.item_scroll)
                && inside(mouse, [cx, cy, CELL, CELL])
            {
                book.item = k;
                book.detail_scroll = 0;
                return true;
            }
        }
        return true;
    }
    for (k, row, col) in grid(&book.entries).0 {
        let Some([cx, cy]) = cell(x, y, row, col, book.grid_scroll) else {
            continue;
        };
        if inside(mouse, [cx, cy, CELL, CELL]) {
            book.selected = k;
            book.morph = 0;
            book.scroll = 0;
            book.detail_scroll = 0;
            return true;
        }
    }
    let (gx, gy) = (x + MARGIN, y + SWATCHES_TOP);
    if mx >= gx && mx < gx + PER_LINE as f32 * STEP && my >= gy && my < gy + ROWS_SHOWN as f32 * STEP {
        let (col, row) = (((mx - gx) / STEP) as usize, ((my - gy) / STEP) as usize);
        let k = (book.scroll + row) * PER_LINE + col;
        let count = book.entries.get(book.selected).map_or(0, |e| e.morphs.len());
        if k < count {
            book.morph = k;
        }
    }
    true
}

/// One step of the wheel on `scroll`, up to `last`.
fn step(scroll: &mut usize, wheel: f64, last: usize) {
    if wheel > 0.0 {
        *scroll = scroll.saturating_sub(1);
    } else if wheel < 0.0 {
        *scroll = (*scroll + 1).min(last);
    }
}

/// From the library's `screen_mouse_scrolled_pre`: the wheel scrolls, a row at a time, the right page's icons, the
/// morph swatches, or the left page's details, whichever it is over. Whether it was taken.
pub fn scrolled(host: &dyn Host, screen: &horizon_client::mod_api::NativeScreenView<'_>, scroll: f64) -> bool {
    if !is_mine(host, screen.id) {
        return false;
    }
    let [x, y] = origin(screen.rect);
    let [mx, my] = screen.mouse;
    let mut book = BOOK.lock().unwrap();
    if mx >= x + PAGE && mx < x + WIDTH && my >= y && my < y + HEIGHT {
        if book.tab == Tab::Items {
            let last = items::pages().len().div_ceil(PER_ROW).saturating_sub(GRID_ROWS);
            step(&mut book.item_scroll, scroll, last);
        } else {
            let last = grid_rows(&book.entries).saturating_sub(GRID_ROWS);
            step(&mut book.grid_scroll, scroll, last);
        }
        return true;
    }
    if !(mx >= x && mx < x + PAGE && my >= y && my < y + HEIGHT) {
        return true;
    }
    let has_swatches = book.tab == Tab::Creatures
        && book
            .entries
            .get(book.selected)
            .and_then(|e| species(&e.species))
            .is_some_and(|s| s.scheme.is_some());
    if has_swatches && my >= y + SWATCHES_TOP {
        let count = book.entries.get(book.selected).map_or(0, |e| e.morphs.len());
        let last = rows(count).saturating_sub(ROWS_SHOWN);
        step(&mut book.scroll, scroll, last);
    } else {
        let last = book.detail_last;
        step(&mut book.detail_scroll, scroll, last);
    }
    true
}

/// `vulture:vulture` → `entity.vulture.vulture`.
fn entity_key(id: &str) -> String {
    format!("entity.{}", id.replacen(':', ".", 1))
}

/// `minecraft:plains` → `biome.minecraft.plains`.
fn biome_key(id: &str) -> String {
    format!("biome.{}", id.replacen(':', ".", 1))
}
