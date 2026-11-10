//! The bestiary, in the creature guide book (SPECS §9): what each player found out about the series' creatures.
//!
//! - Each mod registers its species and the facts a player can learn about them (`register`), from its `hooks()`.
//! - A player **meets** a species when a wild creature of it comes within `MEET` blocks in one of the biomes it spawns
//!   in: its page opens, with that biome (the library's `entity_tick_post`, every `CHECK_EVERY` ticks per creature).
//! - A player **learns** a fact when they find it out in play: the mod calls `learn` (the vulture: raw chicken eaten,
//!   rotten flesh given). Nothing else is shown: no creature, biome or fact the player hasn't come across.
//! - It is kept in the player's persistent data (`creature:bestiary`), on the server. Right-click with the guide sends
//!   it to the player's client (`creature:bestiary` channel), which opens the book: a mod screen the library draws
//!   (`draw`, `clicked`).
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Compound, Host, ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, Tag, Text};

/// A fact a player can learn about a species: an item drawn beside it and its text's language key.
pub struct Fact {
    pub id: &'static str,
    pub item: &'static str,
    pub key: &'static str,
}

/// A species of the series, by its entity type, with what can be learnt about it.
pub struct Species {
    pub id: &'static str,
    /// The biomes it spawns in: only a creature met in one of them counts, and only a wild one (`wild`, by entity
    /// id): a tamed one following its owner elsewhere tells nothing about where the species lives.
    pub biomes: &'static [&'static str],
    pub wild: fn(u32) -> bool,
    pub facts: &'static [Fact],
}

static SPECIES: Mutex<Vec<&'static Species>> = Mutex::new(Vec::new());

/// From a mod's `hooks()`: its species get pages in the book.
pub fn register(species: &'static Species) {
    let mut all = SPECIES.lock().unwrap();
    if !all.iter().any(|s| s.id == species.id) {
        all.push(species);
    }
}

fn species(id: &str) -> Option<&'static Species> {
    SPECIES.lock().unwrap().iter().copied().find(|s| s.id == id)
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
                    biomes.retain(|b| s.biomes.contains(&b.as_str()));
                }
                Some(Entry {
                    species: species_id,
                    biomes,
                    facts: strings(c, "facts"),
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
pub fn tick(host: &mut dyn Host, dimension: &str, entity: u32, kind: &str) {
    let Some(species) = species(kind) else {
        return;
    };
    {
        let mut ticks = TICKS.lock().unwrap();
        let t = ticks.entry(entity).or_insert(entity % CHECK_EVERY);
        *t += 1;
        if *t < CHECK_EVERY {
            return;
        }
        *t = 0;
    }
    if !(species.wild)(entity) {
        return;
    }
    let Some(at) = host.entity(entity).and_then(|e| host.entity_position(e)) else {
        TICKS.lock().unwrap().remove(&entity);
        return;
    };
    let Some(biome) = host.biome([at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32]) else {
        return;
    };
    if !species.biomes.contains(&biome.as_str()) {
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
    for player in near {
        update(host, player, kind, |e| {
            if !e.biomes.contains(&biome) {
                e.biomes.push(biome.clone());
            }
        });
    }
}

/// From the library's `use_item`, the guide in hand: sends the acting player their bestiary, their client opens it.
pub fn open(host: &mut dyn Host) {
    let Some(player) = host.acting_player() else {
        return;
    };
    let text: String = load(host, player.uuid)
        .iter()
        .map(|e| format!("{}|{}|{}\n", e.species, e.biomes.join(","), e.facts.join(",")))
        .collect();
    let _ = host.net_send_to_player(player.uuid, CHANNEL, text.as_bytes());
}

/// The book as the client got it, and the species shown.
struct Book {
    entries: Vec<Entry>,
    selected: usize,
}

static BOOK: Mutex<Book> = Mutex::new(Book {
    entries: Vec::new(),
    selected: 0,
});

/// From the library's `net_from_server`: the bestiary arrived, the book opens.
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
            })
        })
        .collect();
    {
        let mut book = BOOK.lock().unwrap();
        book.entries = entries;
        book.selected = book.selected.min(book.entries.len().saturating_sub(1));
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
/// The species list on the left page: where it starts, a row's height.
const LIST_TOP: f32 = 38.0;
const ROW: f32 = 12.0;

const COVER: u32 = 0xFF5A3A22;
const PAPER: u32 = 0xFFEFE4C8;
const FOLD: u32 = 0xFFD8C9A4;
const HIGHLIGHT: u32 = 0xFFDCCB9C;
const INK: u32 = 0x3B2A1A;
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

/// From the library's `screen_render_post`: draws the open book.
pub fn draw(host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
    if !is_mine(host, event.screen.id) {
        return;
    }
    let batch = &mut *event.batch;
    let [x, y] = origin(event.screen.rect);
    let book = BOOK.lock().unwrap();
    // The cover around two pages and the fold between them.
    host.fill(batch, [x - 4.0, y - 4.0, WIDTH + 8.0, HEIGHT + 8.0], COVER);
    host.fill(batch, [x, y, WIDTH, HEIGHT], PAPER);
    host.fill(batch, [x + PAGE - 1.0, y, 2.0, HEIGHT], FOLD);
    // Left page: the title, how many species met, their names.
    host.text(
        batch,
        &tr("bestiary.creature.title"),
        [x + MARGIN, y + MARGIN],
        INK,
        false,
    );
    if book.entries.is_empty() {
        for (k, line) in wrap(host, &tr("bestiary.creature.empty"), PAGE - 2.0 * MARGIN)
            .iter()
            .enumerate()
        {
            host.text(batch, line, [x + MARGIN, y + LIST_TOP + k as f32 * LINE], FADED, false);
        }
        return;
    }
    let met = format!("{}: {}", tr("bestiary.creature.met"), book.entries.len());
    host.text(batch, &met, [x + MARGIN, y + MARGIN + 12.0], FADED, false);
    for (k, entry) in book.entries.iter().enumerate() {
        let top = y + LIST_TOP + k as f32 * ROW;
        if k == book.selected {
            host.fill(
                batch,
                [x + MARGIN - 3.0, top - 2.0, PAGE - 2.0 * MARGIN + 6.0, ROW],
                HIGHLIGHT,
            );
        }
        host.text(batch, &tr(&entity_key(&entry.species)), [x + MARGIN, top], INK, false);
    }
    // Right page: the species shown, the biomes it was met in, the facts found out.
    let Some(entry) = book.entries.get(book.selected) else {
        return;
    };
    let (px, width) = (x + PAGE + MARGIN, PAGE - 2.0 * MARGIN);
    let mut at = y + MARGIN;
    host.text(batch, &tr(&entity_key(&entry.species)), [px, at], INK, false);
    at += 16.0;
    if !entry.biomes.is_empty() {
        host.text(batch, &tr("bestiary.creature.seen_in"), [px, at], FADED, false);
        at += LINE + 2.0;
        let biomes: Vec<String> = entry.biomes.iter().map(|b| tr(&biome_key(b))).collect();
        for line in wrap(host, &biomes.join(", "), width) {
            host.text(batch, &line, [px, at], INK, false);
            at += LINE;
        }
        at += 6.0;
    }
    let facts: Vec<&Fact> = species(&entry.species)
        .map(|s| {
            s.facts
                .iter()
                .filter(|f| entry.facts.iter().any(|known| known == f.id))
                .collect()
        })
        .unwrap_or_default();
    if facts.is_empty() {
        for line in wrap(host, &tr("bestiary.creature.nothing_more"), width) {
            host.text(batch, &line, [px, at], FADED, false);
            at += LINE;
        }
        return;
    }
    for fact in facts {
        host.gui_draw_item(batch, &ItemStack::new(fact.item, 1), [px, at]);
        let lines = wrap(host, &tr(fact.key), width - 20.0);
        let rows = lines.len().max(2) as f32;
        // One line sits beside the item's middle.
        let top = if lines.len() == 1 { at + 4.0 } else { at };
        for (k, line) in lines.iter().enumerate() {
            host.text(batch, line, [px + 20.0, top + k as f32 * LINE], INK, false);
        }
        at += rows * LINE + 4.0;
    }
}

/// From the library's `screen_mouse_button_pressed_pre`: a click on a species' name shows its page.
pub fn clicked(host: &dyn Host, event: &ScreenMouseButtonPressedPreEvent<'_>) -> bool {
    if !is_mine(host, event.screen.id) || event.button != 0 {
        return false;
    }
    let [x, y] = origin(event.screen.rect);
    let [mx, my] = event.screen.mouse;
    let mut book = BOOK.lock().unwrap();
    if mx >= x && mx < x + PAGE && my >= y + LIST_TOP - 2.0 {
        let row = ((my - (y + LIST_TOP - 2.0)) / ROW) as usize;
        if row < book.entries.len() {
            book.selected = row;
        }
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
