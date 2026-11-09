//! The vulture's screen and its chests' contents (SPECS §6.1, §6.3).
//!
//! Its owner opens it with sneak + right-click (`interaction.rs`, opened on its next tick). It is one of
//! Horizon's server menus (`Host::open_menu_with_input`), shown as a chest: the top row is the vulture's (its harness,
//! sit / follow, a button per chest), then 15 slots per chest that hold items as a chest's do. The contents are saved
//! in the `vulture:chests` attachment when the screen closes; a chest taken off, the harness taken off (only without
//! chests) or the vulture's death gives everything back (to the owner, or on the ground).
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::item_data::encode_components;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, MenuClickedEvent, MenuInputClosedEvent,
    Tag, Text,
};

use crate::state::{self, Change};

/// Slots per chest.
pub const PER_CHEST: usize = 15;
const ROW: usize = 9;
/// The top row's buttons.
const HARNESS: i32 = 0;
const SIT: i32 = 2;
const CHEST_LEFT: i32 = 6;
const CHEST_RIGHT: i32 = 7;

/// The contents of both chests, 30 slots (the right chest's are the last 15).
static CHESTS: AttachmentType<Vec<Option<ItemStack>>> = AttachmentType {
    namespace: "vulture",
    name: "chests",
    default: || vec![None; 2 * PER_CHEST],
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

fn stack_nbt(stack: &ItemStack) -> Compound {
    let mut c = Compound::new();
    c.insert("id".into(), Tag::String((&*stack.item).into()));
    c.insert("count".into(), Tag::Int(stack.count as i32));
    if let Some(components) = encode_components(stack) {
        c.insert("components".into(), components);
    }
    c
}

fn encode(items: &Vec<Option<ItemStack>>) -> Result<Compound, AttachmentError> {
    let list = items
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            let mut c = stack_nbt(s.as_ref()?);
            c.insert("Slot".into(), Tag::Byte(k as i8));
            Some(Tag::Compound(c))
        })
        .collect();
    Ok([("Items".into(), Tag::List(list))].into())
}

fn decode(data: &Compound) -> Result<Vec<Option<ItemStack>>, AttachmentError> {
    let mut items = vec![None; 2 * PER_CHEST];
    if let Some(Tag::List(list)) = data.get(&"Items".into()) {
        for tag in list {
            let Tag::Compound(c) = tag else { continue };
            let Some(Tag::Byte(slot)) = c.get(&"Slot".into()) else {
                continue;
            };
            if let Some(cell) = items.get_mut(*slot as usize) {
                *cell = ItemStack::from_nbt(c);
            }
        }
    }
    Ok(items)
}

/// The screen each player has open: the vulture and the menu's window.
static OPEN: Mutex<BTreeMap<u128, (u32, i32)>> = Mutex::new(BTreeMap::new());

fn button(item: &str) -> Option<ItemStack> {
    Some(ItemStack::new(item, 1))
}

/// The menu's slots: the top row, then the chests' contents, then panes up to the end of the last row.
fn layout(saved: &state::Saved, contents: &[Option<ItemStack>]) -> (u8, Vec<Option<ItemStack>>, usize) {
    let storage = PER_CHEST * usize::from(saved.chests);
    let rows = 1 + storage.div_ceil(ROW);
    let mut items = vec![button("minecraft:gray_stained_glass_pane"); rows * ROW];
    items[HARNESS as usize] = saved.harness.then(|| ItemStack::new("vulture:harness", 1));
    items[SIT as usize] = button(if saved.sitting {
        "minecraft:lead"
    } else {
        "minecraft:feather"
    });
    if saved.chests >= 1 {
        items[CHEST_LEFT as usize] = button("minecraft:chest");
    }
    if saved.chests >= 2 {
        items[CHEST_RIGHT as usize] = button("minecraft:chest");
    }
    for k in 0..storage {
        items[ROW + k] = contents.get(k).cloned().flatten();
    }
    (rows as u8, items, storage)
}

/// Screens asked for by a click, opened on the vulture's next tick: Horizon refuses a menu while the clicking
/// player's own action is being handled.
static ASKED: Mutex<BTreeMap<u32, u128>> = Mutex::new(BTreeMap::new());

/// Sneak + right-click on its own vulture (`interaction.rs` checked it is the owner): its screen, next tick.
pub fn ask(host: &dyn Host, id: u32) {
    if let Some(player) = host.acting_player() {
        ASKED.lock().unwrap().insert(id, player.uuid);
    }
}

/// From `entity_tick_post`: opens the screen asked for this vulture.
pub fn tick(host: &mut dyn Host, id: u32) {
    let Some(player) = ASKED.lock().unwrap().remove(&id) else {
        return;
    };
    let saved = state::saved(id);
    let contents = CHESTS.get(host, Holder::Entity(id)).unwrap_or_default();
    let (rows, items, storage) = layout(&saved, &contents);
    let title = Text::translate("container.vulture.vulture", &[]);
    // Without chests there is nothing to hold: a menu of buttons only.
    let window = if storage == 0 {
        host.open_menu(player, rows, title, items)
    } else {
        host.open_menu_with_input(player, rows, title, items, ROW..ROW + storage)
    };
    if let Some(window) = window {
        OPEN.lock().unwrap().insert(player, (id, window));
    }
}

/// Gives `stacks` to the player, dropping what doesn't fit at the vulture.
fn give_back(host: &mut dyn Host, player: u128, dimension: &str, at: [f64; 3], stacks: Vec<ItemStack>) {
    for stack in stacks {
        if let Some(rest) = host.give_stack(player, stack) {
            drop_at(host, dimension, at, &rest);
        }
    }
}

fn drop_at(host: &mut dyn Host, dimension: &str, at: [f64; 3], stack: &ItemStack) {
    let nbt: Compound = [("Item".into(), Tag::Compound(stack_nbt(stack)))].into();
    host.summon(dimension, "minecraft:item", at, nbt);
}

/// Takes out the contents of chest `index` (0 left, 1 right).
fn empty_chest(host: &mut dyn Host, id: u32, index: usize) -> Vec<ItemStack> {
    let mut contents = CHESTS.get(host, Holder::Entity(id)).unwrap_or_default();
    let taken = contents[index * PER_CHEST..(index + 1) * PER_CHEST]
        .iter_mut()
        .filter_map(Option::take)
        .collect();
    let _ = CHESTS.set(host, Holder::Entity(id), contents);
    taken
}

/// A click on the top row.
pub fn clicked(host: &mut dyn Host, event: &MenuClickedEvent) {
    let Some((id, window)) = OPEN.lock().unwrap().get(&event.uuid).copied() else {
        return;
    };
    if window != event.window {
        return;
    }
    let saved = state::saved(id);
    let Some(player) = host.server_players().into_iter().find(|p| p.uuid == event.uuid) else {
        return;
    };
    let (dimension, at) = (player.dimension.clone(), player.position);
    match event.slot {
        SIT => state::request(id, Change::ToggleSit),
        CHEST_LEFT | CHEST_RIGHT if saved.chests > 0 => {
            // The last chest put on comes off first; what it held goes back with it.
            let index = usize::from(saved.chests) - 1;
            let mut stacks = empty_chest(host, id, index);
            stacks.push(ItemStack::new("minecraft:chest", 1));
            give_back(host, event.uuid, &dimension, at, stacks);
            state::request(id, Change::RemoveChest);
        }
        HARNESS if saved.harness && saved.chests == 0 => {
            give_back(
                host,
                event.uuid,
                &dimension,
                at,
                vec![ItemStack::new("vulture:harness", 1)],
            );
            state::request(id, Change::RemoveHarness);
        }
        _ => return,
    }
    // The buttons changed what the screen shows: close it, the player opens it again.
    host.close_menu(event.uuid);
}

/// The screen closed: what is in the chests' slots is kept.
pub fn closed(host: &mut dyn Host, event: &mut MenuInputClosedEvent<'_>) {
    let Some((id, window)) = OPEN.lock().unwrap().remove(&event.uuid) else {
        return;
    };
    if window != event.window {
        return;
    }
    let storage = PER_CHEST * usize::from(state::saved(id).chests);
    let mut contents = CHESTS.get(host, Holder::Entity(id)).unwrap_or_default();
    for (k, slot) in event.slots.iter_mut().enumerate().take(storage) {
        contents[k] = slot.take();
    }
    let _ = CHESTS.set(host, Holder::Entity(id), contents);
}

/// A screen without chests closed (no input slots, so `closed` doesn't hear it).
pub fn forget(player: u128) {
    OPEN.lock().unwrap().remove(&player);
}

/// Its death: its harness, chests and their contents fall where it died.
pub fn died(host: &mut dyn Host, dimension: &str, id: u32) {
    let Some(at) = host.entity(id).and_then(|e| host.entity_position(e)) else {
        return;
    };
    let saved = state::saved(id);
    let mut stacks: Vec<ItemStack> = CHESTS
        .get(host, Holder::Entity(id))
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect();
    if saved.harness {
        stacks.push(ItemStack::new("vulture:harness", 1));
    }
    if saved.chests > 0 {
        stacks.push(ItemStack::new("minecraft:chest", u32::from(saved.chests)));
    }
    for stack in &stacks {
        drop_at(host, dimension, at, stack);
    }
}
