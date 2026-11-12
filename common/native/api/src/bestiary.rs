//! The bestiary, in the creature guide book (SPECS §9): what each player found out about the series' creatures.
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
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::containers::{ModBlockEntity, ModBlockEntityFields};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{
    BlockEntityDraw, BlockEntitySubmitEvent, BlockStateId, Compound, Host, NativeSceneBlock, NativeSceneView,
    ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, Tag, Text,
};

use crate::colours::{self, Look, Scheme};

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
    /// Its icon on the book's right page (an item: its spawn egg).
    pub icon: &'static str,
    /// Its looks, when it has some (the morphs met are noted and shown).
    pub scheme: Option<&'static Scheme>,
    /// Draws it in the book's 3D view, standing on the block's floor, centred on the block, about a block across (the
    /// mod scales its model to fit), in `look` when it has looks.
    pub draw: fn(&dyn Host, &mut BlockEntityDraw, Option<Look>),
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
                    biomes.retain(|b| s.biomes.contains(&b.as_str()));
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

/// The book as the client got it, the species shown and the morph picked.
struct Book {
    entries: Vec<Entry>,
    selected: usize,
    /// The morph picked (in the sorted order) and the first swatch row seen.
    morph: usize,
    scroll: usize,
}

static BOOK: Mutex<Book> = Mutex::new(Book {
    entries: Vec::new(),
    selected: 0,
    morph: 0,
    scroll: 0,
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
/// The right page's icons: where the grid starts, a cell's size, how many to a row.
const GRID_TOP: f32 = 36.0;
const CELL: f32 = 22.0;
const PER_ROW: usize = 5;
/// The left page's 3D view: its box (from the page's corner) and how it is seen.
const VIEW: [f32; 4] = [MARGIN, 20.0, PAGE - 2.0 * MARGIN, 66.0];
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

/// The creature the 3D view draws this frame, for `specimen` (`draw_scene` asks for it within the call).
static SUBJECT: Mutex<Option<(&'static Species, Option<Look>)>> = Mutex::new(None);

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

/// From the library's `block_entity_submit`: in the book's scene, draws the creature shown.
pub fn specimen(host: &dyn Host, event: &mut BlockEntitySubmitEvent<'_>) {
    let Some((species, look)) = *SUBJECT.lock().unwrap() else {
        return;
    };
    (species.draw)(host, &mut *event.draw, look);
}

/// Draws `species` in `look`, turning, in the box `rect`.
fn view(
    host: &dyn Host,
    batch: &mut horizon_client::mod_api::HudBatch,
    rect: [f32; 4],
    species: &'static Species,
    look: Option<Look>,
) {
    let Some((barrier, moss)) = scene_states(host) else {
        return;
    };
    let entity = ModBlockEntity::new(ModBlockEntityFields {
        id: "creature:specimen".into(),
        ..ModBlockEntityFields::default()
    });
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let yaw = (millis % SPIN) as f32 / SPIN as f32 * 360.0;
    *SUBJECT.lock().unwrap() = Some((species, look));
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
            scale: VIEW_SCALE,
        },
    );
    *SUBJECT.lock().unwrap() = None;
}

/// The right page's cell `k`: its top left corner.
fn cell(x: f32, y: f32, k: usize) -> [f32; 2] {
    [
        x + PAGE + MARGIN + (k % PER_ROW) as f32 * CELL,
        y + GRID_TOP + (k / PER_ROW) as f32 * CELL,
    ]
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
    // Right page: the title, how many species met, their icons.
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
    for (k, entry) in book.entries.iter().enumerate() {
        let [cx, cy] = cell(x, y, k);
        if k == book.selected {
            host.fill(batch, [cx - 1.0, cy - 1.0, CELL - 2.0, CELL - 2.0], HIGHLIGHT);
        }
        if let Some(s) = species(&entry.species) {
            host.gui_draw_item(batch, &ItemStack::new(s.icon, 1), [cx + 1.0, cy + 1.0]);
        }
    }
    // Left page: the creature picked.
    let Some(entry) = book.entries.get(book.selected) else {
        return;
    };
    let Some(sp) = species(&entry.species) else {
        return;
    };
    let (px, width) = (x + MARGIN, PAGE - 2.0 * MARGIN);
    host.text(batch, &tr(&entity_key(&entry.species)), [px, y + 8.0], INK, false);
    let morphs = sorted_morphs(sp, entry);
    let look = sp.scheme.map(|_| morphs.get(book.morph).copied().unwrap_or_default());
    let view_box = [x + VIEW[0], y + VIEW[1], VIEW[2], VIEW[3]];
    host.fill(batch, view_box, VIEW_BACK);
    view(host, batch, view_box, sp, look);
    let mut at = y + VIEW[1] + VIEW[3] + 4.0;
    // The biomes it was met in.
    if !entry.biomes.is_empty() {
        let biomes: Vec<String> = entry.biomes.iter().map(|b| tr(&biome_key(b))).collect();
        for line in wrap(host, &biomes.join(", "), width).iter().take(2) {
            host.text(batch, line, [px, at], FADED, false);
            at += LINE;
        }
    }
    // What was found out: an item and a word each, side by side.
    let facts: Vec<&Fact> = sp
        .facts
        .iter()
        .filter(|f| entry.facts.iter().any(|known| known == f.id))
        .collect();
    let mut fx = px;
    for fact in facts {
        let label = tr(fact.key);
        host.gui_draw_item(batch, &ItemStack::new(fact.item, 1), [fx, at]);
        host.text(batch, &label, [fx + 18.0, at + 4.0], INK, false);
        fx += 24.0 + host.text_width(&label);
    }
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

/// From the library's `screen_mouse_button_pressed_pre`: an icon opens its species' page, a swatch shows its morph.
pub fn clicked(host: &dyn Host, event: &ScreenMouseButtonPressedPreEvent<'_>) -> bool {
    if !is_mine(host, event.screen.id) || event.button != 0 {
        return false;
    }
    let [x, y] = origin(event.screen.rect);
    let [mx, my] = event.screen.mouse;
    let mut book = BOOK.lock().unwrap();
    for k in 0..book.entries.len() {
        let [cx, cy] = cell(x, y, k);
        if mx >= cx && mx < cx + CELL && my >= cy && my < cy + CELL {
            book.selected = k;
            book.morph = 0;
            book.scroll = 0;
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

/// From the library's `screen_mouse_scrolled_pre`: the wheel over the swatches scrolls them a row at a time. Whether it
/// was taken.
pub fn scrolled(host: &dyn Host, screen: &horizon_client::mod_api::NativeScreenView<'_>, scroll: f64) -> bool {
    if !is_mine(host, screen.id) {
        return false;
    }
    let [x, y] = origin(screen.rect);
    let [mx, my] = screen.mouse;
    let (gx, gy) = (x + MARGIN, y + SWATCHES_TOP);
    if !(mx >= gx && mx < gx + PER_LINE as f32 * STEP + 4.0 && my >= gy && my < gy + ROWS_SHOWN as f32 * STEP) {
        return true;
    }
    let mut book = BOOK.lock().unwrap();
    let count = book.entries.get(book.selected).map_or(0, |e| e.morphs.len());
    let last = rows(count).saturating_sub(ROWS_SHOWN);
    if scroll > 0.0 {
        book.scroll = book.scroll.saturating_sub(1);
    } else if scroll < 0.0 {
        book.scroll = (book.scroll + 1).min(last);
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
