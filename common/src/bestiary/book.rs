//! The creature guide's book on the client: opened with the player's bestiary (`open`, `received`), drawn as a mod
//! screen (`draw`: the Creatures and Items tabs, a creature's or an item's page, the 3D view), clicked and scrolled.
use super::*;

/// From a module's `hooks()`: its item gets a page in the Items tab (`items`).
pub fn register_item(page: &'static ItemPage) {
    items::register(page);
}

/// From the library's `use_item`, the guide in hand: sends `player` their bestiary, their client opens it.
pub fn open(host: &mut dyn ServerHost, player: u128) {
    open_for_player(host, player);
}

/// Open for an explicit server player, also used by `/bestiary` as a fallback when the held guide's use is swallowed
/// by another item or input hook.
pub fn open_for_player(host: &mut dyn ServerHost, player: u128) {
    let text: String = load(host, player)
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
    send_chunked(host, player, &text);
}

/// A payload's string holds at most 4096 bytes (`PAYLOAD_MAX_STRING_BYTES`): the book is sent in chunks under `CHUNK`
/// bytes (with the flag byte), split on line (species) boundaries, each with a flag
/// byte; the client (`received`) accumulates them and opens on the last. (`B` one and only, `S` start, `M` more, `E`
/// end.)
const CHUNK: usize = mod_api::PAYLOAD_MAX_STRING_BYTES - 1;
fn send_chunked(host: &mut dyn ServerHost, player: u128, text: &str) {
    let mut chunks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for line in text.split_inclusive('\n') {
        if !cur.is_empty() && cur.len() + line.len() > CHUNK {
            chunks.push(std::mem::take(&mut cur));
        }
        cur.push_str(line);
    }
    if !cur.is_empty() || chunks.is_empty() {
        chunks.push(cur);
    }
    let n = chunks.len();
    for (i, chunk) in chunks.iter().enumerate() {
        let flag = match i {
            _ if n == 1 => b'B',
            0 => b'S',
            _ if i == n - 1 => b'E',
            _ => b'M',
        };
        let mut packet = Vec::with_capacity(1 + chunk.len());
        packet.push(flag);
        packet.extend_from_slice(chunk.as_bytes());
        super::send(host, player, packet);
    }
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

/// Accumulates the book's chunks (`send_chunked`) between the start and end packets.
static ACCUM: Mutex<String> = Mutex::new(String::new());

/// From the library's payload handler: the bestiary arrives in chunks; on the last the book opens (on the tab last
/// read).
pub fn received(host: &mut dyn ClientHost, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let (flag, chunk) = (data[0], String::from_utf8_lossy(&data[1..]));
    let text = {
        let mut accum = ACCUM.lock().unwrap();
        if flag == b'S' || flag == b'B' {
            accum.clear();
        }
        accum.push_str(&chunk);
        // Not the last packet yet: wait for the rest before opening.
        if flag != b'E' && flag != b'B' {
            return;
        }
        std::mem::take(&mut *accum)
    };
    let list = |s: &str| -> Vec<String> { s.split(',').filter(|x| !x.is_empty()).map(String::from).collect() };
    let entries = text
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
const VIEW_PITCH: f32 = 22.0;
/// The height of our creatures' template (`minecraft:wandering_trader`), by which a screen centres them.
const TEMPLATE_HEIGHT: f32 = 1.95;
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

/// The text of translation `key` in the player's language (the key itself when no language has it).
fn tr(host: &dyn ClientHost, key: &str) -> String {
    host.translate(key).unwrap_or_else(|| key.to_owned())
}

/// The book's top left corner, centred on the screen.
fn origin(rect: [f32; 4]) -> [f32; 2] {
    [rect[0] + (rect[2] - WIDTH) / 2.0, rect[1] + (rect[3] - HEIGHT) / 2.0]
}

fn is_mine(host: &dyn ClientHost) -> bool {
    host.mod_screen() == Some(MOD_ID)
}

/// `text` cut into lines at most `width` wide.
fn wrap(host: &dyn ClientHost, text: &str, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if tw(host, &candidate) > width && !line.is_empty() {
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

/// Draws `subject`, turning, in the box `rect`: a creature itself (its own renderer, in its look picked), an item's
/// icon, large.
fn view(host: &dyn ClientHost, batch: &mut dyn DrawContext, rect: [f32; 4], subject: Subject) {
    let yaw = (millis() % SPIN) as f32 / SPIN as f32 * 360.0;
    match subject {
        // A creature drawn itself, as in the world: a vanilla one about 1.4 blocks to the box's two thirds, ours by
        // their own box (its height, or half its width for a creature seen side on). Sunrise centres a screen's
        // entity by its type's height, our template's (a trader's: sunrise-org/sunrise#1134), so ours are moved to
        // stand by their own.
        Subject::Creature(species, look) => {
            colours::set_preview(look);
            let (extent, offset) = match crate::mob::size_of(species.id) {
                Some(s) => (s.height.max(s.width / 2.0), (s.height - TEMPLATE_HEIGHT) / 2.0),
                None => (1.4, 0.0),
            };
            let view = GuiEntityView::new(
                [rect[0], rect[1], rect[0] + rect[2], rect[1] + rect[3]],
                rect[3] * 0.66 / extent,
                offset,
                GuiEntityLook::Turned { yaw, pitch: VIEW_PITCH },
            );
            let _ = batch.entity(&GuiEntity::kind(species.id), view);
        }
        Subject::Item(item) => icon(host, batch, rect, item),
    }
}

/// Item `item`'s icon, large, in the middle of `rect`.
fn icon(host: &dyn Host, batch: &mut dyn DrawContext, rect: [f32; 4], item: &str) {
    let size = rect[3].min(rect[2]) * 0.8;
    if let Some(key) = item_key(host, item) {
        batch.item(
            key,
            [rect[0] + (rect[2] - size) / 2.0, rect[1] + (rect[3] - size) / 2.0],
            size,
        );
    }
}

/// The item `id`'s key, to draw it.
fn item_key(host: &dyn Host, id: &str) -> Option<ItemKey> {
    host.item_key(&Identifier::parse(id).ok()?)
}

/// Draws item `id`'s icon with its top left at `at`, as in a slot, with `count` written on it.
fn gui_item(host: &dyn Host, batch: &mut dyn DrawContext, id: &str, at: [f32; 2], count: Option<&str>) {
    if let Some(key) = item_key(host, id) {
        batch.gui_draw_item_with(&ItemStack::of(key), at, count);
    }
}

/// Text drawn in `colour` (0xRRGGBB, or 0xAARRGGBB): Sunrise draws text with its alpha, our colours have none.
fn ink(colour: u32) -> u32 {
    if colour >> 24 == 0 {
        colour | 0xFF00_0000
    } else {
        colour
    }
}

/// How wide `text` is drawn, in GUI pixels.
fn tw(host: &dyn ClientHost, text: &str) -> f32 {
    host.text_width(&Text::literal(text)).map_or(0.0, |w| w as f32)
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
fn ribbons(host: &dyn ClientHost, x: f32, y: f32) -> [(Tab, [f32; 4], String); 2] {
    let creatures = tr(host, "bestiary.creature.tab.creatures");
    let items = tr(host, "bestiary.creature.tab.items");
    let w1 = tw(host, &creatures) + 8.0;
    let w2 = tw(host, &items) + 8.0;
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

    fn draw(&self, host: &dyn ClientHost, batch: &mut dyn DrawContext, px: f32, at: f32) {
        match self {
            Row::Text(text, colour, indent) => batch.text(text, [px + indent, at], ink(*colour), false),
            Row::Facts(facts) => {
                let mut fx = px;
                for (item, label) in facts {
                    gui_item(host, batch, item, [fx, at], None);
                    batch.text(label, [fx + 18.0, at + 4.0], ink(INK), false);
                    fx += 24.0 + tw(host, label);
                }
            }
            Row::Icons(items) => {
                for (k, item) in items.iter().enumerate() {
                    gui_item(host, batch, item, [px + k as f32 * 18.0, at], None);
                }
            }
            Row::Line(item, text) => {
                gui_item(host, batch, item, [px, at], None);
                batch.text(text, [px + 18.0, at + 4.0], ink(INK), false);
            }
            // Right under the line it follows, whose row is an icon high.
            Row::More(text) => batch.text(text, [px + 18.0, at + 4.0 - (ICON_ROW - LINE)], ink(INK), false),
            Row::Recipe(r, stations) => draw_recipe(host, batch, r, stations, px, at),
        }
    }
}

/// A recipe: its 3 × 3 grid (a slot with several items shows each in turn), its stations, an arrow, what it makes.
fn draw_recipe(host: &dyn ClientHost, batch: &mut dyn DrawContext, r: &Recipe, stations: &[Station], px: f32, at: f32) {
    let turn = (millis() / 1000) as usize;
    for (k, slot) in r.grid.iter().enumerate() {
        let [sx, sy] = [px + (k % 3) as f32 * SLOT, at + (k / 3) as f32 * SLOT];
        batch.fill([sx, sy, SLOT - 1.0, SLOT - 1.0], SLOT_BACK);
        if !slot.is_empty() {
            gui_item(host, batch, slot[turn % slot.len()], [sx, sy], None);
        }
    }
    let ax = px + 3.0 * SLOT + 4.0;
    for (k, station) in stations.iter().enumerate() {
        gui_item(host, batch, station.icon(), [ax + 2.0 + k as f32 * 18.0, at], None);
    }
    batch.text("->", [ax + 4.0, at + SLOT + 5.0], ink(FADED), false);
    let rx = ax + 22.0;
    batch.fill([rx - 1.0, at + SLOT - 1.0, SLOT, SLOT], SLOT_BACK);
    let count = (r.count > 1).then(|| r.count.to_string());
    gui_item(host, batch, r.result, [rx, at + SLOT], count.as_deref());
}

/// Wraps items with their word into rows `width` wide.
fn fact_rows(host: &dyn ClientHost, facts: Vec<(&'static str, String)>, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut row: Vec<(&'static str, String)> = Vec::new();
    let mut used = 0.0;
    for (item, label) in facts {
        let w = 24.0 + tw(host, &label);
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
fn line_rows(host: &dyn ClientHost, icon: &'static str, text: &str, width: f32) -> Vec<Row> {
    let mut lines = wrap(host, text, width - 18.0).into_iter();
    let mut rows = vec![Row::Line(icon, lines.next().unwrap_or_default())];
    rows.extend(lines.map(Row::More));
    rows
}

/// A species' details: the biomes it was met in, what was found out, then its drops under their heading.
fn creature_rows(host: &dyn ClientHost, sp: &Species, entry: &Entry, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    if !entry.biomes.is_empty() {
        let biomes: Vec<String> = entry.biomes.iter().map(|b| tr(host, &biome_key(b))).collect();
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
        facts.iter().map(|f| (f.item, tr(host, f.key))).collect(),
        width,
    ));
    if !drops.is_empty() {
        rows.push(Row::Text(tr(host, "bestiary.creature.drops"), FADED, 0.0));
        rows.extend(icon_rows(drops.iter().map(|f| f.item).collect(), width));
    }
    rows
}

/// An item's details: how it is got (its recipes, who drops it, who gives it), then its uses and what it makes.
fn item_rows(host: &dyn ClientHost, page: &ItemPage, width: f32) -> Vec<Row> {
    let mut rows = Vec::new();
    let made = items::made_by(page.item);
    if !made.is_empty() || !page.dropped_by.is_empty() || !page.given_by.is_empty() {
        rows.push(Row::Text(tr(host, "bestiary.creature.item.get"), FADED, 0.0));
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
        let name = tr(host, &entity_key(creature));
        let text = tr(host, "bestiary.creature.item.dropped_by")
            .replace("%1$s", &name)
            .replace("%s", &name);
        rows.extend(line_rows(host, icon, &text, width));
    }
    for given in page.given_by {
        rows.extend(line_rows(host, given.icon, &tr(host, given.key), width));
    }
    let used_in = items::used_in(page.item);
    if !page.uses.is_empty() || !used_in.is_empty() {
        rows.push(Row::Text(tr(host, "bestiary.creature.item.uses"), FADED, 0.0));
    }
    for u in page.uses {
        rows.extend(line_rows(host, u.icon, &tr(host, u.key), width));
    }
    if !used_in.is_empty() {
        rows.push(Row::Text(tr(host, "bestiary.creature.item.makes"), FADED, 0.0));
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
    host: &dyn ClientHost,
    batch: &mut dyn DrawContext,
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
        batch.fill([bx, top, 2.0, bh], FOLD);
        let thumb = (bh / (last + 1) as f32).max(6.0);
        let at = top + (bh - thumb) * scroll as f32 / last as f32;
        batch.fill([bx, at, 2.0, thumb], 0xFF00_0000 | FADED);
    }
    last
}

/// From the library's `screen_render_post`: draws the open book.
pub fn draw(host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
    if !is_mine(host) {
        return;
    }
    let batch = &mut *event.draw;
    let [x, y] = origin(event.screen.rect);
    let mut book = BOOK.lock().unwrap();
    // The ribbons, the open one in the paper's colour; the cover around two pages and the fold between them.
    for (tab, r, label) in ribbons(host, x, y) {
        let open = tab == book.tab;
        batch.fill([r[0], r[1], r[2], r[3] + 4.0], if open { PAPER } else { RIBBON_SHUT });
        let colour = if open { INK } else { 0xEFE4C8 };
        batch.text(&label, [r[0] + 4.0, r[1] + 3.0], ink(colour), false);
    }
    batch.fill([x - 4.0, y - 4.0, WIDTH + 8.0, HEIGHT + 8.0], COVER);
    batch.fill([x, y, WIDTH, HEIGHT], PAPER);
    batch.fill([x + PAGE - 1.0, y, 2.0, HEIGHT], FOLD);
    match book.tab {
        Tab::Creatures => draw_creatures(host, batch, &mut book, x, y),
        Tab::Items => draw_items(host, batch, &mut book, x, y),
    }
}

/// The right page's scrollbar, when there are more rows of icons than shown.
fn grid_scrollbar(_: &dyn ClientHost, batch: &mut dyn DrawContext, x: f32, y: f32, total: usize, scroll: usize) {
    if total > GRID_ROWS {
        let (bx, by, bh) = (x + WIDTH - 5.0, y + GRID_TOP, GRID_ROWS as f32 * CELL - 2.0);
        batch.fill([bx, by, 2.0, bh], FOLD);
        let thumb = bh * GRID_ROWS as f32 / total as f32;
        let top = by + (bh - thumb) * scroll as f32 / (total - GRID_ROWS) as f32;
        batch.fill([bx, top, 2.0, thumb], 0xFF00_0000 | FADED);
    }
}

/// The Creatures tab: the species met on the right, the one picked on the left.
fn draw_creatures(host: &dyn ClientHost, batch: &mut dyn DrawContext, book: &mut Book, x: f32, y: f32) {
    let rx = x + PAGE + MARGIN;
    batch.text(&tr(host, "bestiary.creature.title"), [rx, y + MARGIN], ink(INK), false);
    if book.entries.is_empty() {
        for (k, line) in wrap(host, &tr(host, "bestiary.creature.empty"), PAGE - 2.0 * MARGIN)
            .iter()
            .enumerate()
        {
            batch.text(line, [rx, y + GRID_TOP + k as f32 * LINE], ink(FADED), false);
        }
        return;
    }
    let met = format!("{}: {}", tr(host, "bestiary.creature.met"), book.entries.len());
    batch.text(&met, [rx, y + MARGIN + 12.0], ink(FADED), false);
    let (cells, line) = grid(&book.entries);
    for &(k, row, col) in &cells {
        let Some([cx, cy]) = cell(x, y, row, col, book.grid_scroll) else {
            continue;
        };
        if k == book.selected {
            batch.fill([cx - 1.0, cy - 1.0, CELL - 2.0, CELL - 2.0], HIGHLIGHT);
        }
        if let Some(s) = species(&book.entries[k].species) {
            gui_item(host, batch, s.icon, [cx + 1.0, cy + 1.0], None);
        }
    }
    // The line above Minecraft's own creatures, when the series' are above it.
    if let Some(row) = line.filter(|r| *r > 0)
        && let Some([_, ly]) = cell(x, y, row, 0, book.grid_scroll)
    {
        batch.fill([rx, ly - 2.0, PAGE - 2.0 * MARGIN, 1.0], FOLD);
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
    batch.text(&tr(host, &entity_key(&entry.species)), [px, y + 8.0], ink(INK), false);
    // How rare it is, on the right of its name.
    let rarity = tr(host, sp.rarity.key());
    let rx_end = x + PAGE - MARGIN - tw(host, &rarity);
    batch.text(&rarity, [rx_end, y + 8.0], ink(sp.rarity.colour()), false);
    let morphs = sorted_morphs(sp, entry);
    let look = sp.scheme.map(|_| morphs.get(book.morph).copied().unwrap_or_default());
    // A species with looks has its swatches below: its view is shorter, to leave its details room.
    let view_height = if sp.scheme.is_some() {
        VIEW_WITH_LOOKS_HEIGHT
    } else {
        VIEW[3]
    };
    let view_box = [x + VIEW[0], y + VIEW[1], VIEW[2], view_height];
    batch.fill(view_box, VIEW_BACK);
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
                batch.fill([sx - 1.0, sy - 1.0, SWATCH + 2.0, SWATCH + 2.0], PICKED);
            }
            batch.fill([sx, sy, SWATCH, SWATCH], rgb(m.base));
            if scheme.patterns.get(m.pattern).is_some_and(|p| p.texture.is_some()) {
                batch.fill([sx + 3.0, sy + 3.0, SWATCH - 6.0, SWATCH - 6.0], rgb(m.pattern_colour));
            }
        }
        if let Some(m) = morphs.get(book.morph) {
            let ns = entry.species.split(':').next().unwrap_or("");
            let name = |kind: &str, id: &str| tr(host, &format!("look.{ns}.{kind}.{id}"));
            let pattern = scheme.patterns.get(m.pattern);
            let mut text = name("colour", scheme.colours[m.base].id);
            if let Some(p) = pattern.filter(|p| p.texture.is_some()) {
                text = format!(
                    "{text}, {} ({})",
                    name("pattern", p.id),
                    name("colour", scheme.colours[m.pattern_colour].id)
                );
            }
            batch.text(
                &text,
                [px, y + SWATCHES_TOP + ROWS_SHOWN as f32 * STEP],
                ink(FADED),
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
            batch.fill([bx, by, 2.0, bh], FOLD);
            let thumb = bh * ROWS_SHOWN as f32 / total as f32;
            let top = by + (bh - thumb) * book.scroll as f32 / (total - ROWS_SHOWN) as f32;
            batch.fill([bx, top, 2.0, thumb], 0xFF00_0000 | FADED);
        }
    }
}

/// The Items tab: the series' items on the right, the one picked on the left.
fn draw_items(host: &dyn ClientHost, batch: &mut dyn DrawContext, book: &mut Book, x: f32, y: f32) {
    let rx = x + PAGE + MARGIN;
    batch.text(&tr(host, "bestiary.creature.items"), [rx, y + MARGIN], ink(INK), false);
    let pages = items::pages();
    let count = format!("{}: {}", tr(host, "bestiary.creature.items_count"), pages.len());
    batch.text(&count, [rx, y + MARGIN + 12.0], ink(FADED), false);
    for (k, row, col) in item_grid(pages.len()) {
        let Some([cx, cy]) = cell(x, y, row, col, book.item_scroll) else {
            continue;
        };
        if k == book.item {
            batch.fill([cx - 1.0, cy - 1.0, CELL - 2.0, CELL - 2.0], HIGHLIGHT);
        }
        gui_item(host, batch, pages[k].item, [cx + 1.0, cy + 1.0], None);
    }
    grid_scrollbar(host, batch, x, y, pages.len().div_ceil(PER_ROW), book.item_scroll);
    // Left page: the item picked, turning, then how it is got and what it is for.
    let Some(page) = pages.get(book.item) else {
        return;
    };
    let (px, width) = (x + MARGIN, PAGE - 2.0 * MARGIN);
    batch.text(&tr(host, &items::item_key(page.item)), [px, y + 8.0], ink(INK), false);
    let view_box = [x + VIEW[0], y + VIEW[1], VIEW[2], ITEM_VIEW_HEIGHT];
    batch.fill(view_box, VIEW_BACK);
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
pub fn clicked(host: &dyn ClientHost, event: &ScreenMouseButtonPressedPreEvent<'_>) -> bool {
    if !is_mine(host) || event.button != 0 {
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
pub fn scrolled(host: &dyn ClientHost, screen: &ScreenView<'_>, scroll: f64) -> bool {
    if !is_mine(host) {
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

/// `minecraft:plains` → `biome.minecraft.plains`.
fn biome_key(id: &str) -> String {
    format!("biome.{}", id.replacen(':', ".", 1))
}
