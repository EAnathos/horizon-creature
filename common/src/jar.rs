//! The glass jar (SPECS §4.1): catching a small creature and releasing it elsewhere.
//!
//! - A mod registers the creatures a jar can hold (`register`), with the icon of a jar holding one.
//! - **Catch**: right-click a registered creature with an empty `creature:glass_jar` (the library's `entity_interact`,
//!   which runs before the mods'): the creature is taken out of the world and the player gets a filled jar. A filled jar
//!   is the same item, carrying the creature as item data (`minecraft:custom_data`: `creature: {kind, look}`, its look
//!   from `colours`), its name ("Glass Jar of Butterfly") and its icon set on the stack; it doesn't stack.
//! - **Release**: right-click a block with a filled jar (`use_item_on`): the creature comes out on that face, with its
//!   look, and the jar is empty again. A jar whose creature's mod isn't installed any more stays full.
//!
//! A befriended creature (`friend`) is only caught by its friend; the jar keeps its friend, and it is still theirs once
//! released. A named creature keeps its name (shown under the jar's), from the catch to the release.
//!
//! In creative, as a bucket, the jar isn't used up: catching keeps the empty jar and releasing keeps the full one.
//! Released on a flower, a creature whose kind settles in a flower patch settles there (`territory`, §4.2).
//!
//! **Display**: sneak + right-click with a filled jar places it as a small block (`creature:jar`, or `creature:lit_jar`
//! giving light for a creature that glows), the creature drawn inside with its species' own drawing (its bestiary 3D
//! view); broken, it gives the filled jar back (not in creative). Its block entity (type `creature:jar`, `JarEntity`)
//! holds the creature, sent to clients to draw it. Pistons can't move it (`push_reaction` in its block file).
use std::sync::Mutex;

use mod_api::{
    BlockEntityDrawContext, BlockEntityPart, BlockEntitySubmitEvent, BlockEntityUpdateTagEvent, BlockPart,
    BlockUpdateFlags, ClientHost, Compound, EntityInteractEvent, Hand, Host, HostSide, InteractionResult, ItemPart,
    ItemStack, LevelRef, LevelSide, PlayerRef, Registrar, ServerHost, Tag, UseOnEvent,
};

use crate::colours;
use crate::hand;
use crate::territory::Ground;

pub const ITEM: &str = "creature:glass_jar";
/// The bestiary fact a catch teaches (a species lists it among its facts to show it).
pub const JAR_FACT: &str = "jar";
/// The bestiary fact a release on a flower teaches, when the creature settles there.
pub const FLOWER_FACT: &str = "flower";
/// The bestiary fact a release in a garden teaches, when the creature settles there.
pub const GARDEN_FACT: &str = "garden";
/// The bestiary fact a release on a sandy patch teaches, when the creature settles there.
pub const SAND_FACT: &str = "sand";
/// The key of the jar's data in `minecraft:custom_data`.
const DATA: &str = "creature";

/// A creature a jar can hold: its entity type, and the item model of a jar holding it (`<mod>:<kind>_jar`).
pub struct Catchable {
    pub kind: &'static str,
    pub model: &'static str,
    /// Where it settles when released (`territory`, SPECS §4.2): on a flower (its patch) or on farmland (its garden).
    pub settles: Option<crate::territory::Rule>,
    /// A jar holding it, placed as a block, gives light (fireflies, fairies).
    pub glows: bool,
}

static CATCHABLE: Mutex<Vec<&'static Catchable>> = Mutex::new(Vec::new());

/// From a mod's `register`: its creatures can be caught.
pub fn register(catchable: &'static Catchable) {
    let mut all = CATCHABLE.lock().unwrap();
    if !all.iter().any(|c| c.kind == catchable.kind) {
        all.push(catchable);
    }
}

/// Whether a mod uses the jar (one registered a catchable creature): the guide shows its page only then.
pub fn used() -> bool {
    !CATCHABLE.lock().unwrap().is_empty()
}

fn catchable(kind: &str) -> Option<&'static Catchable> {
    CATCHABLE.lock().unwrap().iter().copied().find(|c| c.kind == kind)
}

/// What a jar holds: its creature's kind, look, friend and name.
struct Held {
    kind: String,
    look: Option<Compound>,
    friend: Option<[i32; 4]>,
    name: Option<String>,
}

/// What a filled jar holds, from its item data (`kind`, `look`, `friend`, `name`) or a placed jar's block entity
/// (`Kind`, `Look`, `Friend`, `Name`).
fn read(data: &Compound, [kind, look, friend, name]: [&str; 4]) -> Option<Held> {
    let kind = match data.get(&kind.into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => return None,
    };
    let look = match data.get(&look.into()) {
        Some(Tag::Compound(c)) => Some(c.clone()),
        _ => None,
    };
    let friend = match data.get(&friend.into()) {
        Some(Tag::IntArray(a)) if a.len() == 4 => Some([a[0], a[1], a[2], a[3]]),
        _ => None,
    };
    let name = match data.get(&name.into()) {
        Some(Tag::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    };
    Some(Held {
        kind,
        look,
        friend,
        name,
    })
}

/// What a filled jar holds.
fn held(stack: &ItemStack) -> Option<Held> {
    let Some(Tag::Compound(custom)) = stack.components()?.as_nbt().get(&"minecraft:custom_data".into()) else {
        return None;
    };
    let Some(Tag::Compound(data)) = custom.get(&DATA.into()) else {
        return None;
    };
    read(data, ["kind", "look", "friend", "name"])
}

/// An empty jar.
fn empty(host: &dyn Host) -> Option<ItemStack> {
    host.item_from_snbt(&format!("{{id:{},count:1}}", quoted(ITEM)))
}

/// SNBT for a string.
fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A filled jar holding `kind` with `look`, the friend of `friend`, named `name`.
fn filled(
    host: &dyn Host,
    c: &Catchable,
    look: Option<&Compound>,
    friend: Option<[i32; 4]>,
    name: Option<&str>,
) -> Option<ItemStack> {
    let look_fields = look.map(|l| {
        let text = |key: &str| match l.get(&key.into()) {
            Some(Tag::String(s)) => s.to_string_lossy(),
            _ => String::new(),
        };
        [text("Base"), text("Pattern"), text("PatternColour")]
    });
    let look = look
        .map(|l| {
            let fields: Vec<String> = l
                .iter()
                .filter_map(|(k, v)| match v {
                    Tag::String(s) => Some(format!("{}:{}", k.to_string_lossy(), quoted(&s.to_string_lossy()))),
                    _ => None,
                })
                .collect();
            format!(",look:{{{}}}", fields.join(","))
        })
        .unwrap_or_default();
    let friend_data = friend
        .map(|f| format!(",friend:[I;{},{},{},{}]", f[0], f[1], f[2], f[3]))
        .unwrap_or_default();
    let name_data = name.map(|n| format!(",name:{}", quoted(n))).unwrap_or_default();
    let entity_key = format!("entity.{}", c.kind.replacen(':', ".", 1));
    let snbt = format!(
        "{{id:{item},count:1,components:{{\"minecraft:custom_data\":{{{DATA}:{{kind:{kind}{look_data}{friend_data}{name_data}}}}},\
         \"minecraft:item_model\":{model},\"minecraft:max_stack_size\":1,\
         \"minecraft:item_name\":{{translate:\"item.creature.glass_jar.filled\",with:[{{translate:{name}}}]}},\
         \"minecraft:lore\":[{lore}]}}}}",
        item = quoted(ITEM),
        kind = quoted(c.kind),
        look_data = look,
        model = quoted(c.model),
        name = quoted(&entity_key),
        lore = lore(c.kind, look_fields, friend.is_some(), name),
    );
    host.item_from_snbt(&snbt)
}

/// The filled jar's lines under its name: what it holds (its colour, its pattern and the pattern's colour, in the
/// mod's words: `look.<mod>.colour.<id>`, `look.<mod>.pattern.<id>`), then the series' name in blue, as the creative
/// tab names an item it lists (a filled jar is in no tab, so the game shows no tab line for it). A named creature's jar
/// says its name first, a befriended creature's jar says so.
fn lore(kind: &str, look: Option<[String; 3]>, befriended: bool, name: Option<&str>) -> String {
    let namespace = kind.split(':').next().unwrap_or(kind);
    let colour = |id: &str| format!("{{translate:{}}}", quoted(&format!("look.{namespace}.colour.{id}")));
    let pattern = |id: &str| format!("{{translate:{}}}", quoted(&format!("look.{namespace}.pattern.{id}")));
    let mut lines = Vec::new();
    if let Some(name) = name {
        lines.push(format!(
            "{{translate:\"item.creature.glass_jar.name\",with:[{{text:{}}}],color:\"white\",italic:false}}",
            quoted(name)
        ));
    }
    if let Some([base, pat, pat_colour]) = look {
        let line = if pat == "plain" || pat.is_empty() {
            format!(
                "{{translate:\"item.creature.glass_jar.look_plain\",with:[{}]",
                colour(&base)
            )
        } else {
            format!(
                "{{translate:\"item.creature.glass_jar.look\",with:[{},{},{}]",
                colour(&base),
                pattern(&pat),
                colour(&pat_colour)
            )
        };
        lines.push(format!("{line},color:\"gray\",italic:false}}"));
    }
    if befriended {
        lines.push("{translate:\"item.creature.glass_jar.friend\",color:\"light_purple\",italic:false}".into());
    }
    lines.push("{translate:\"itemGroup.creature\",color:\"blue\",italic:false}".into());
    lines.join(",")
}

/// The name a name tag gave creature `id`, if any.
fn custom_name(host: &mut dyn ServerHost, id: u32) -> Option<String> {
    let entity = host.entity(id)?;
    let named = entity
        .nbt(host)
        .is_some_and(|nbt| matches!(nbt.get(&"CustomName".into()), Some(t) if !matches!(t, Tag::End)));
    named.then(|| entity.name(host)).flatten()
}

/// Whether the right-click of `event` is an empty jar on a registered creature (both sides answer it, so the client
/// neither trades nor tries the off hand).
pub fn catches(host: &dyn Host, event: &EntityInteractEvent<'_>) -> bool {
    event
        .stack
        .is_some_and(|s| hand::is(host, s, ITEM) && held(s).is_none())
        && catchable(event.kind).is_some()
}

/// From the library's `entity_interact`, on the server: an empty jar catches a registered creature. Whether it did.
pub fn catch(host: &mut dyn ServerHost, event: &EntityInteractEvent<'_>) -> bool {
    if !catches(&*host, event) {
        return false;
    }
    let Some(c) = catchable(event.kind) else {
        return false;
    };
    let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity)) else {
        return false;
    };
    // A befriended one only comes to its friend.
    let friend = crate::friend::of(id);
    if friend.is_some_and(|f| !f.is(player.uuid())) {
        return false;
    }
    let look = colours::saved(host, id);
    let name = custom_name(host, id);
    let Some(jar) = filled(&*host, c, look.as_ref(), friend.map(|f| f.uuid), name.as_deref()) else {
        return false;
    };
    if !host.entity(id).is_some_and(|e| e.discard(host)) {
        return false;
    }
    colours::forget(id);
    crate::friend::forget(id);
    crate::body::forget(id);
    // The bestiary: they now know a jar catches it.
    crate::bestiary::learn(host, player.uuid(), c.kind, JAR_FACT);
    // The only jar in hand becomes the filled one; from a stack, one is taken (not in creative, as a bucket) and the
    // filled jar goes to the inventory.
    let one = event.stack.is_some_and(|s| s.count() == 1);
    if one {
        hand::set_held(host, player, event.hand, Some(jar));
        return true;
    }
    hand::use_one(host, player, event.hand);
    hand::give(host, player, jar);
    true
}

/// Whether `stack` is a filled jar (its right-click on a block is the jar's).
pub fn is_filled(host: &dyn Host, stack: &ItemStack) -> bool {
    hand::is(host, stack, ITEM) && held(stack).is_some()
}

/// From the jar's `use_on` (`JarItem`): a filled jar releases its creature on the face clicked. Whether the click was
/// the jar's.
pub fn release(host: &mut dyn ServerHost, event: &UseOnEvent<'_>) -> bool {
    let stack = event.stack;
    let Some(Held {
        kind,
        look,
        friend,
        name,
    }) = held(stack)
    else {
        return false;
    };
    let Some(player) = host.player(event.player) else {
        return true;
    };
    let Some(level) = player.level(&*host) else {
        return true;
    };
    // Its mod isn't installed: the jar stays full.
    let Some(c) = catchable(&kind) else {
        return true;
    };
    let (face, pos) = (event.face, event.pos);
    // Sneaking: the jar is placed as a block, the creature inside (`display`).
    if player.entity().is_shift_key_down(&*host).unwrap_or(false) {
        place(
            host,
            level,
            pos,
            face,
            player,
            event.hand,
            c,
            look.as_ref(),
            friend,
            name.as_deref(),
        );
        return true;
    }
    // On a flower (which a click hits itself, a flower being no full block) it comes out in the flower, and settles
    // there if its kind does; elsewhere on the face clicked.
    use crate::territory::Rule;
    let ground = Ground::new(&*host, level);
    let on_flower = ground.flower_at(pos);
    // On farmland of a big enough garden, or a big enough sandy patch, it comes out on that ground and settles there,
    // each rule teaching its own fact.
    let settle = match c.settles {
        Some(Rule::Garden) => crate::territory::garden_at(ground, pos).map(|f| (f, GARDEN_FACT)),
        Some(Rule::Sand) => crate::territory::sand_at(ground, pos).map(|f| (f, SAND_FACT)),
        _ => None,
    };
    let block = if let Some((f, _)) = settle {
        [f[0], f[1] + 1, f[2]]
    } else if on_flower {
        pos
    } else {
        [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]]
    };
    let at = [
        f64::from(block[0]) + 0.5,
        f64::from(block[1]),
        f64::from(block[2]) + 0.5,
    ];
    // Its name back, as a name tag gave it.
    let mut nbt = Compound::new();
    if let Some(n) = &name {
        nbt.insert("CustomName".into(), Tag::String(n.as_str().into()));
    }
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    let Some(id) = host.summon(&dimension, &kind, at, nbt) else {
        return true;
    };
    if let Some(look) = &look {
        colours::restore(host, id, look);
    }
    if let Some(f) = friend {
        crate::friend::befriend(host, id, crate::friend::uuid(f));
    }
    if let Some((f, fact)) = settle {
        crate::territory::settle(host, id, f);
        crate::bestiary::learn(host, player.uuid(), c.kind, fact);
    } else if on_flower && c.settles == Some(Rule::FlowerPatch) {
        crate::territory::settle(host, id, block);
        crate::bestiary::learn(host, player.uuid(), c.kind, FLOWER_FACT);
    }
    if !player.has_infinite_materials(&*host).unwrap_or(false) {
        let jar = empty(&*host);
        hand::set_held(host, player, event.hand, jar);
    }
    true
}

/// The jar placed as a block (SPECS §4.1, "display"): plain, or giving light for a creature that glows.
pub const BLOCK: &str = "creature:jar";
pub const LIT_BLOCK: &str = "creature:lit_jar";

/// From the library's `explosion_detonate`: a blast spares the placed jars (their creature would be lost).
pub fn spare_from_blast(ground: Ground<'_>, blocks: &mut Vec<[i32; 3]>) {
    blocks.retain(|&pos| !is_jar_block(&ground.block(pos)));
}

fn is_jar_block(name: &str) -> bool {
    name == BLOCK || name == LIT_BLOCK
}

/// Sneak + right-click with a filled jar: placed as a block on the face clicked, holding its creature (kind and look in
/// its block entity), the hand's jar used up (not in creative).
#[allow(clippy::too_many_arguments)]
fn place(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    face: [i32; 3],
    player: PlayerRef,
    in_hand: Hand,
    c: &Catchable,
    look: Option<&Compound>,
    friend: Option<[i32; 4]>,
    name: Option<&str>,
) {
    let at = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
    if Ground::new(&*host, level).block(at) != "minecraft:air" {
        return;
    }
    let Some(state) = host.block_state_id(if c.glows { LIT_BLOCK } else { BLOCK }) else {
        return;
    };
    if !level
        .set_block(host, at, state, BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS)
        .unwrap_or(false)
    {
        return;
    }
    let mut nbt = Compound::new();
    nbt.insert("Kind".into(), Tag::String(c.kind.into()));
    if let Some(l) = look {
        nbt.insert("Look".into(), Tag::Compound(l.clone()));
    }
    if let Some(f) = friend {
        nbt.insert("Friend".into(), Tag::IntArray(f.to_vec()));
    }
    if let Some(n) = name {
        nbt.insert("Name".into(), Tag::String(n.into()));
    }
    if let Some(entity) = host.block_entity(level, at) {
        let _ = entity.merge_data(host, nbt);
        let _ = entity.set_changed(host);
    }
    hand::use_one(host, player, in_hand);
}

/// What a placed jar holds: from its block entity's data.
fn held_in(nbt: &Compound) -> Option<Held> {
    read(nbt, ["Kind", "Look", "Friend", "Name"])
}

/// From the library's `break_block`: a placed jar broken gives back the filled jar (not in creative, as a block).
pub fn broken(host: &mut dyn ServerHost, pos: [i32; 3]) {
    let Some(player) = host.players().acting_player() else {
        return;
    };
    let Some(level) = player.level(&*host) else {
        return;
    };
    if !is_jar_block(&Ground::new(&*host, level).block(pos)) || player.has_infinite_materials(&*host).unwrap_or(false) {
        return;
    }
    let Some(data) = host.block_entity(level, pos).and_then(|e| e.data(&*host)) else {
        return;
    };
    let Some(Held {
        kind,
        look,
        friend,
        name,
    }) = held_in(&data)
    else {
        return;
    };
    let item = match catchable(&kind) {
        Some(c) => filled(&*host, c, look.as_ref(), friend, name.as_deref()),
        None => empty(&*host),
    };
    if let Some(item) = item {
        let at = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.3,
            f64::from(pos[2]) + 0.5,
        ];
        hand::drop_stack(host, level, at, item);
    }
}

/// The placed jar's block entity: holds its creature, sent to clients, which draw it inside.
pub struct JarEntity;

#[mod_api::sunrise_part]
impl BlockPart for JarEntity {}

#[mod_api::sunrise_part]
impl BlockEntityPart for JarEntity {
    fn block_entity(&self, _: LevelSide, _: &mut mod_api::BlockEntityEvent<'_>) -> Option<Compound> {
        Some(Compound::new())
    }

    fn block_entity_update_tag(&self, event: &mut BlockEntityUpdateTagEvent<'_>) -> Option<Compound> {
        Some(event.entity.nbt.clone())
    }

    fn block_entity_submit(&self, _: &dyn ClientHost, event: &mut BlockEntitySubmitEvent<'_>) {
        draw_inside(&event.entity.nbt, &mut *event.draw);
    }
}

/// The glass jar item: a filled one used on a block releases its creature.
struct JarItem;

#[mod_api::sunrise_part]
impl ItemPart for JarItem {
    fn use_on(&self, host: HostSide, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        match host {
            HostSide::Server(host) => release(host, event).then_some(InteractionResult::Success),
            HostSide::Client(host) => is_filled(&*host, event.stack).then_some(InteractionResult::Success),
        }
    }
}

/// From the library's `register`: the jar item's part and the placed jars' block entity type, `creature:jar`.
pub fn declare(r: &mut dyn Registrar) {
    let item = r.item_key(ITEM);
    r.item_part(item, JarItem);
    let kind = r.block_entity_type("jar").blocks(&["jar", "lit_jar"]).build();
    let _ = r.block_entity_part(&kind, JarEntity);
}

/// Draws the creature inside a placed jar, small, in the middle, with the species' own drawing (its bestiary 3D view).
fn draw_inside(nbt: &Compound, draw: &mut dyn BlockEntityDrawContext) {
    let Some(Held { kind, look, .. }) = held_in(nbt) else {
        return;
    };
    let Some(species) = crate::bestiary::species_of(&kind) else {
        return;
    };
    let look = species
        .scheme
        .and_then(|scheme| look.as_ref().and_then(|l| colours::from_compound(scheme, l)));
    draw.push();
    draw.translate([0.5, 0.06, 0.5]);
    draw.scale([INSIDE; 3]);
    draw.translate([-0.5, 0.0, -0.5]);
    (species.draw)(draw, look);
    draw.pop();
}

/// The creature's size in a placed jar, against its size in the bestiary (about a block).
const INSIDE: f32 = 0.4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_quoted_safely_in_snbt() {
        assert_eq!(quoted("Lili"), "\"Lili\"");
        assert_eq!(quoted("Bob \"the\" fairy"), "\"Bob \\\"the\\\" fairy\"");
        assert_eq!(quoted("back\\slash"), "\"back\\\\slash\"");
    }

    #[test]
    fn a_named_befriended_jar_says_both_then_the_series() {
        let lines = lore("fairies:fairy", None, true, Some("Lili"));
        let name = lines.find("glass_jar.name").unwrap();
        let friend = lines.find("glass_jar.friend").unwrap();
        let series = lines.find("itemGroup.creature").unwrap();
        assert!(name < friend && friend < series);
        assert!(lines.contains("{text:\"Lili\"}"));
        assert_eq!(lore("worms:worm", None, false, None).matches("translate").count(), 1);
    }
}
