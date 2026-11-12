//! The glass jar (SPECS §6.1): catching a small creature and releasing it elsewhere.
//!
//! - A mod registers the creatures a jar can hold (`register`), with the icon of a jar holding one.
//! - **Catch**: right-click a registered creature with an empty `creature:glass_jar` (the library's `interact_entity`,
//!   which runs before the mods'): the creature is taken out of the world and the player gets a filled jar. A filled jar
//!   is the same item, carrying the creature as item data (`minecraft:custom_data`: `creature: {kind, look}`, its look
//!   from `colours`), its name ("Glass Jar of Butterfly") and its icon set on the stack; it doesn't stack.
//! - **Release**: right-click a block with a filled jar (`use_item_on`): the creature comes out on that face, with its
//!   look, and the jar is empty again. A jar whose creature's mod isn't installed any more stays full.
//!
//! In creative, as a bucket, the jar isn't used up: catching keeps the empty jar and releasing keeps the full one.
//! Released on a flower, a creature whose kind settles in a flower patch settles there (`territory`, §6.2).
//!
//! **Display**: sneak + right-click with a filled jar places it as a small block (`creature:jar`, or `creature:lit_jar`
//! giving light for a creature that glows), the creature drawn inside with its species' own drawing (its bestiary 3D
//! view); broken, it gives the filled jar back (not in creative).
use std::sync::Mutex;

use horizon_client::items::ItemStack;
use horizon_client::mod_api::{Compound, Host, InteractEntityEvent, Tag, UseItemOnEvent};

use crate::colours;

pub const ITEM: &str = "creature:glass_jar";
/// The bestiary fact a catch teaches (a species lists it among its facts to show it).
pub const JAR_FACT: &str = "jar";
/// The bestiary fact a release on a flower teaches, when the creature settles there.
pub const FLOWER_FACT: &str = "flower";
/// The bestiary fact a release in a garden teaches, when the creature settles there.
pub const GARDEN_FACT: &str = "garden";
/// The key of the jar's data in `minecraft:custom_data`.
const DATA: &str = "creature";

/// A creature a jar can hold: its entity type, and the item model of a jar holding it (`<mod>:<kind>_jar`).
pub struct Catchable {
    pub kind: &'static str,
    pub model: &'static str,
    /// Where it settles when released (`territory`, SPECS §6.2): on a flower (its patch) or on farmland (its garden).
    pub settles: Option<crate::territory::Rule>,
    /// A jar holding it, placed as a block, gives light (fireflies).
    pub glows: bool,
}

static CATCHABLE: Mutex<Vec<&'static Catchable>> = Mutex::new(Vec::new());

/// From a mod's `hooks()`: its creatures can be caught.
pub fn register(catchable: &'static Catchable) {
    let mut all = CATCHABLE.lock().unwrap();
    if !all.iter().any(|c| c.kind == catchable.kind) {
        all.push(catchable);
    }
}

fn catchable(kind: &str) -> Option<&'static Catchable> {
    CATCHABLE.lock().unwrap().iter().copied().find(|c| c.kind == kind)
}

/// What a filled jar holds: its creature's kind and look.
fn held(stack: &ItemStack) -> Option<(String, Option<Compound>)> {
    let Some(Tag::Compound(data)) = stack.custom_data()?.get(&DATA.into()) else {
        return None;
    };
    let kind = match data.get(&"kind".into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => return None,
    };
    let look = match data.get(&"look".into()) {
        Some(Tag::Compound(c)) => Some(c.clone()),
        _ => None,
    };
    Some((kind, look))
}

/// SNBT for a string.
fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A filled jar holding `kind` with `look`.
fn filled(host: &dyn Host, c: &Catchable, look: Option<&Compound>) -> Option<ItemStack> {
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
    let entity_key = format!("entity.{}", c.kind.replacen(':', ".", 1));
    let snbt = format!(
        "{{id:{item},count:1,components:{{\"minecraft:custom_data\":{{{DATA}:{{kind:{kind}{look_data}}}}},\
         \"minecraft:item_model\":{model},\"minecraft:max_stack_size\":1,\
         \"minecraft:item_name\":{{translate:\"item.creature.glass_jar.filled\",with:[{{translate:{name}}}]}},\
         \"minecraft:lore\":[{lore}]}}}}",
        item = quoted(ITEM),
        kind = quoted(c.kind),
        look_data = look,
        model = quoted(c.model),
        name = quoted(&entity_key),
        lore = lore(c.kind, look_fields),
    );
    host.item_from_snbt(&snbt)
}

/// The filled jar's lines under its name: what it holds (its colour, its pattern and the pattern's colour, in the
/// mod's words: `look.<mod>.colour.<id>`, `look.<mod>.pattern.<id>`), then the series' name in blue, as the creative
/// tab names an item it lists (a filled jar is in no tab, so the game shows no tab line for it).
fn lore(kind: &str, look: Option<[String; 3]>) -> String {
    let namespace = kind.split(':').next().unwrap_or(kind);
    let colour = |id: &str| format!("{{translate:{}}}", quoted(&format!("look.{namespace}.colour.{id}")));
    let pattern = |id: &str| format!("{{translate:{}}}", quoted(&format!("look.{namespace}.pattern.{id}")));
    let mut lines = Vec::new();
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
    lines.push("{translate:\"itemGroup.creature\",color:\"blue\",italic:false}".into());
    lines.join(",")
}

/// The acting player's slot holding `stack` (the hotbar and inventory, then the off hand).
fn slot_of(host: &mut dyn Host, stack: &ItemStack) -> Option<i32> {
    // The selected hotbar slot, when it holds this stack: Horizon doesn't say which slot is selected, but `held` reads
    // it, so each hotbar slot holding the same stack is marked a moment (its count changed, then put back) to see which
    // one `held` sees. Without that, a jar in another slot looking the same was filled in place of the held one.
    for s in 0..9 {
        let Some(count) = host.carried_stack(s).filter(|c| **c == *stack).map(|c| c.count) else {
            continue;
        };
        if let Some(c) = host.carried_stack(s) {
            c.count = count + MARK;
        }
        let selected = host.held().is_some_and(|(_, n, _)| n == count + MARK);
        if let Some(c) = host.carried_stack(s) {
            c.count = count;
        }
        if selected {
            return Some(s);
        }
    }
    (0..36)
        .chain([-106])
        .find(|&s| host.carried_stack(s).is_some_and(|c| *c == *stack))
}

/// A count no stack has, to tell the selected slot (`slot_of`).
const MARK: u32 = 1000;

/// From the library's `interact_entity`: an empty jar catches a registered creature.
pub fn catch(host: &mut dyn Host, event: &InteractEntityEvent<'_>) -> bool {
    let Some(stack) = event.stack.filter(|s| &*s.item == ITEM && held(s).is_none()) else {
        return false;
    };
    let Some(c) = catchable(event.kind) else {
        return false;
    };
    let (Some(player), Ok(id)) = (host.acting_player(), u32::try_from(event.entity)) else {
        return false;
    };
    let look = colours::saved(host, id);
    let Some(jar) = filled(host, c, look.as_ref()) else {
        return false;
    };
    if !host.remove_mob(&player.dimension, id) {
        return false;
    }
    colours::forget(id);
    // The bestiary: they now know a jar catches it.
    crate::bestiary::learn(host, player.uuid, c.kind, JAR_FACT);
    // The only jar in hand becomes the filled one, in its slot; from a stack, one is taken (not in creative, as a
    // bucket) and the filled jar goes to the inventory.
    let slot = slot_of(host, stack);
    if stack.count == 1
        && let Some(s) = slot.and_then(|slot| host.carried_stack(slot))
    {
        *s = jar;
        return true;
    }
    if player.game_mode != 1
        && let Some(s) = slot.and_then(|slot| host.carried_stack(slot))
    {
        s.count -= 1;
    }
    if let Some(rest) = host.give_stack(player.uuid, jar) {
        drop_stack(host, &player.dimension, player.position, &rest);
    }
    true
}

fn drop_stack(host: &mut dyn Host, dimension: &str, at: [f64; 3], stack: &ItemStack) {
    let mut item = Compound::new();
    item.insert("id".into(), Tag::String((&*stack.item).into()));
    item.insert("count".into(), Tag::Int(stack.count as i32));
    if let Some(components) = horizon_client::item_data::encode_components(stack) {
        item.insert("components".into(), components);
    }
    let nbt: Compound = [("Item".into(), Tag::Compound(item))].into();
    host.summon(dimension, "minecraft:item", at, nbt);
}

/// From the library's `interaction_key`, on the window's client, for each hand the use key tries: with a jar in the
/// main hand, the off hand's turn is skipped. The client can't tell that the jar acted (the server's hooks do it), so it
/// would go on to the off hand and place its torch (shown, then undone by the server, and the torch spent); vanilla
/// stops at the hand that acted.
pub fn holds_jar(host: &dyn Host) -> bool {
    host.held().is_some_and(|(item, _, _)| item == ITEM)
}

/// From the library's `use_item_on`: a filled jar releases its creature on the face clicked. Whether the click was the
/// jar's.
pub fn release(host: &mut dyn Host, event: &UseItemOnEvent<'_>) -> bool {
    if &*event.stack.item != ITEM {
        return false;
    }
    let Some((kind, look)) = held(event.stack) else {
        return false;
    };
    let Some(player) = host.acting_player() else {
        return true;
    };
    // Its mod isn't installed: the jar stays full.
    if catchable(&kind).is_none() {
        return true;
    }
    let c = catchable(&kind).expect("checked above");
    // Sneaking: the jar is placed as a block, the creature inside (`display`).
    if player.sneaking {
        place(host, event, &player, c, look.as_ref());
        return true;
    }
    // On a flower (which a click hits itself, a flower being no full block) it comes out in the flower, and settles
    // there if its kind does; elsewhere on the face clicked.
    use crate::territory::Rule;
    let on_flower = crate::territory::is_flower(host, &host.block(event.pos));
    // On farmland (or a crop on it) of a big enough garden, it comes out on the farmland and settles in its garden.
    let garden = match c.settles {
        Some(Rule::Garden) => crate::territory::garden_at(host, event.pos),
        _ => None,
    };
    let block = if let Some(f) = garden {
        [f[0], f[1] + 1, f[2]]
    } else if on_flower {
        event.pos
    } else {
        [
            event.pos[0] + event.face[0],
            event.pos[1] + event.face[1],
            event.pos[2] + event.face[2],
        ]
    };
    let at = [
        f64::from(block[0]) + 0.5,
        f64::from(block[1]),
        f64::from(block[2]) + 0.5,
    ];
    let Some(id) = host.summon(&player.dimension, &kind, at, Compound::new()) else {
        return true;
    };
    if let Some(look) = &look {
        colours::restore(host, id, look);
    }
    if let Some(f) = garden {
        crate::territory::settle(host, id, f);
        crate::bestiary::learn(host, player.uuid, c.kind, GARDEN_FACT);
    } else if on_flower && c.settles == Some(Rule::FlowerPatch) {
        crate::territory::settle(host, id, block);
        crate::bestiary::learn(host, player.uuid, c.kind, FLOWER_FACT);
    }
    if player.game_mode != 1
        && let Some(slot) = slot_of(host, event.stack)
        && let Some(s) = host.carried_stack(slot)
    {
        *s = ItemStack::new(ITEM, 1);
    }
    true
}

/// The jar placed as a block (SPECS §6.1, "display"): plain, or giving light for a creature that glows.
pub const BLOCK: &str = "creature:jar";
pub const LIT_BLOCK: &str = "creature:lit_jar";

fn is_jar_block(name: &str) -> bool {
    name == BLOCK || name == LIT_BLOCK
}

fn state_of(host: &dyn Host, name: &str) -> Option<horizon_client::mod_api::BlockStateId> {
    host.block_states()
        .iter()
        .find(|(_, f)| &*f.name == name)
        .map(|(id, _)| id)
}

/// Sneak + right-click with a filled jar: placed as a block on the face clicked, holding its creature (kind and look in
/// its block entity), the hand's jar used up (not in creative).
fn place(
    host: &mut dyn Host,
    event: &UseItemOnEvent<'_>,
    player: &horizon_client::mod_api::NativeServerPlayer,
    c: &Catchable,
    look: Option<&Compound>,
) {
    let at = [
        event.pos[0] + event.face[0],
        event.pos[1] + event.face[1],
        event.pos[2] + event.face[2],
    ];
    if host.block(at) != "minecraft:air" {
        return;
    }
    let Some(state) = state_of(host, if c.glows { LIT_BLOCK } else { BLOCK }) else {
        return;
    };
    if !host.set_block(&player.dimension, at, state, 3) {
        return;
    }
    let kind = c.kind;
    let look = look.cloned();
    host.change_mod_block_entity(at, &mut |e| {
        e.nbt.insert("Kind".into(), Tag::String(kind.into()));
        if let Some(l) = &look {
            e.nbt.insert("Look".into(), Tag::Compound(l.clone()));
        }
    });
    host.block_entity_sync(at);
    if player.game_mode != 1
        && let Some(slot) = slot_of(host, event.stack)
        && let Some(s) = host.carried_stack(slot)
    {
        s.count = 0;
    }
}

/// What a placed jar holds: from its block entity's data.
fn held_in(nbt: &Compound) -> Option<(String, Option<Compound>)> {
    let kind = match nbt.get(&"Kind".into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => return None,
    };
    let look = match nbt.get(&"Look".into()) {
        Some(Tag::Compound(c)) => Some(c.clone()),
        _ => None,
    };
    Some((kind, look))
}

/// From the library's `block_entity`: a placed jar has a block entity, to hold its creature.
pub fn block_entity(block: &str) -> Option<(String, Compound)> {
    is_jar_block(block).then(|| (BLOCK.to_string(), Compound::new()))
}

/// From the library's `block_entity_update_tag`: clients get what a placed jar holds, to draw it.
pub fn update_tag(entity: &horizon_client::containers::ModBlockEntity) -> Option<Compound> {
    (entity.id == BLOCK).then(|| entity.nbt.clone())
}

/// From the library's `destroy_block`: a placed jar broken gives back the filled jar (not in creative, as a block).
pub fn broken(host: &mut dyn Host, pos: [i32; 3]) {
    if !is_jar_block(&host.block(pos)) {
        return;
    }
    let Some(player) = host.acting_player() else {
        return;
    };
    if player.game_mode == 1 {
        return;
    }
    let Some(entity) = host.mod_block_entity(pos) else {
        return;
    };
    let Some((kind, look)) = held_in(&entity.nbt) else {
        return;
    };
    let item = match catchable(&kind) {
        Some(c) => filled(host, c, look.as_ref()),
        None => Some(ItemStack::new(ITEM, 1)),
    };
    if let Some(item) = item {
        let at = [
            f64::from(pos[0]) + 0.5,
            f64::from(pos[1]) + 0.3,
            f64::from(pos[2]) + 0.5,
        ];
        drop_stack(host, &player.dimension, at, &item);
    }
}

/// From the library's `block_entity_submit`: draws the creature inside a placed jar, small, in the middle, with the
/// species' own drawing (its bestiary 3D view).
pub fn draw_inside(host: &dyn Host, event: &mut horizon_client::mod_api::BlockEntitySubmitEvent<'_>) {
    if !is_jar_block(event.block) {
        return;
    }
    let Some((kind, look)) = held_in(&event.entity.nbt) else {
        return;
    };
    let Some(species) = crate::bestiary::species_of(&kind) else {
        return;
    };
    let look = species
        .scheme
        .and_then(|scheme| look.as_ref().and_then(|l| colours::from_compound(scheme, l)));
    let draw = &mut *event.draw;
    draw.push();
    draw.translate([0.5, 0.06, 0.5]);
    draw.scale([INSIDE; 3]);
    draw.translate([-0.5, 0.0, -0.5]);
    (species.draw)(host, draw, look);
    draw.pop();
}

/// The creature's size in a placed jar, against its size in the bestiary (about a block).
const INSIDE: f32 = 0.4;
