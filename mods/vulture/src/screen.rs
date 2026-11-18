//! The vulture's screen and its chests' contents (SPECS §6.1, §6.3).
//!
//! Its owner opens it with sneak + right-click (`interaction.rs`, opened on its next tick). It is a chest menu a mod opens (`open_menu`): the
//! top row is the vulture's (its saddle, sit / follow, a button per chest), then 15 slots per chest. Such a menu
//! moves nothing by itself: its clicks (`clicked`) are the screen's. A click (left, right or with shift) on a stack
//! sends it across, from a chest to the inventory or back, as much of it as fits; other clicks (a drag, a number key)
//! do nothing. Items can't go on the cursor: writing to a player's slots or cursor through `ItemPlace` from the click
//! hook moves nothing on this revision, so the stacks are written directly (`set_inventory_slot`, `menu_set_items`).
//!
//! The contents of its chests are saved in the `vulture:chests` attachment after each click (each slot's item as its
//! NBT, read back with a host: `ItemStack::from_nbt`); a chest taken off, the saddle taken off (only without chests)
//! or the vulture's death gives everything back (to the owner, or on the ground).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, ItemStack, MenuClickedEvent, MenuClosedEvent,
    MenuReplace, PlayerRef, Registrar, ServerHost, Tag, Text,
};

use crate::state::{self, Change};

/// Slots per chest.
pub const PER_CHEST: usize = 15;
const ROW: usize = 9;
/// The top row's buttons.
const HARNESS: usize = 0;
const SIT: usize = 2;
const CHEST_LEFT: usize = 6;
const CHEST_RIGHT: usize = 7;
/// The player's inventory after the menu's own slots: 27 slots and the hotbar's 9.
const INVENTORY: usize = 36;
/// Vanilla's click types (`ClickType`): a click and a shift-click.
const PICKUP: i32 = 0;
const QUICK_MOVE: i32 = 1;

/// The contents of both chests, 30 slots (the right chest's are the last 15), each item's NBT.
static CHESTS: AttachmentType<Vec<Option<Compound>>> = AttachmentType {
    namespace: "vulture",
    name: "chests",
    default: || vec![None; 2 * PER_CHEST],
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the mod's `register`: its attachments.
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("vulture:chests", false);
}

// `&Vec`: the attachment's codec takes `fn(&T)` with `T` its type, a `Vec`.
#[allow(clippy::ptr_arg)]
fn encode(items: &Vec<Option<Compound>>) -> Result<Compound, AttachmentError> {
    let list = items
        .iter()
        .enumerate()
        .filter_map(|(k, s)| {
            let mut c = s.clone()?;
            c.insert("Slot".into(), Tag::Byte(k as i8));
            Some(Tag::Compound(c))
        })
        .collect();
    Ok([("Items".into(), Tag::List(list))].into())
}

fn decode(data: &Compound) -> Result<Vec<Option<Compound>>, AttachmentError> {
    let mut items = vec![None; 2 * PER_CHEST];
    if let Some(Tag::List(list)) = data.get(&"Items".into()) {
        for tag in list {
            let Tag::Compound(c) = tag else { continue };
            let Some(Tag::Byte(slot)) = c.get(&"Slot".into()) else {
                continue;
            };
            if let Some(cell) = items.get_mut(*slot as usize) {
                let mut item = c.clone();
                item.remove(&"Slot".into());
                *cell = Some(item);
            }
        }
    }
    Ok(items)
}

/// The screen each player has open: the vulture, the menu's window and how many chest slots it shows.
static OPEN: Mutex<BTreeMap<u128, Open>> = Mutex::new(BTreeMap::new());

#[derive(Clone, Copy, Debug)]
struct Open {
    vulture: u32,
    window: i32,
    storage: usize,
}

/// The menu's slots: the top row, then the chests' contents, then panes up to the end of the last row.
fn layout(host: &dyn ServerHost, saved: &state::Saved, contents: &[Option<ItemStack>]) -> (u8, Vec<Option<ItemStack>>) {
    let storage = PER_CHEST * usize::from(saved.chests);
    let rows = 1 + storage.div_ceil(ROW);
    let pane = stack(host, "minecraft:gray_stained_glass_pane", 1);
    let mut items = vec![pane; rows * ROW];
    items[HARNESS] = if saved.harness {
        stack(host, crate::interaction::SADDLE, 1)
    } else {
        None
    };
    items[SIT] = stack(
        host,
        if saved.sitting {
            "minecraft:lead"
        } else {
            "minecraft:feather"
        },
        1,
    );
    if saved.chests >= 1 {
        items[CHEST_LEFT] = stack(host, "minecraft:chest", 1);
    }
    if saved.chests >= 2 {
        items[CHEST_RIGHT] = stack(host, "minecraft:chest", 1);
    }
    for k in 0..storage {
        items[ROW + k] = contents.get(k).cloned().flatten();
    }
    (rows as u8, items)
}

/// The chests' contents as stacks.
fn contents(host: &mut dyn ServerHost, id: u32) -> Vec<Option<ItemStack>> {
    CHESTS
        .get(host, Holder::Entity(id))
        .unwrap_or_default()
        .iter()
        .map(|c| c.as_ref().and_then(|c| ItemStack::from_nbt(&*host, c)))
        .collect()
}

fn set_contents(host: &mut dyn ServerHost, id: u32, items: &[Option<ItemStack>]) {
    let mut nbt: Vec<Option<Compound>> = items
        .iter()
        .map(|s| s.as_ref().and_then(|s| s.to_nbt(&*host)))
        .collect();
    nbt.resize(2 * PER_CHEST, None);
    let _ = CHESTS.set(host, Holder::Entity(id), nbt);
}

/// Screens asked for by a click, opened on the vulture's next tick: a menu opened while the clicking player's own
/// action is handled doesn't show.
static ASKED: Mutex<BTreeMap<u32, u128>> = Mutex::new(BTreeMap::new());

/// Sneak + right-click on its own vulture (`interaction.rs` checked it is the owner): its screen, next tick.
pub fn ask(player: PlayerRef, id: u32) {
    ASKED.lock().unwrap().insert(id, player.uuid());
}

/// From the vulture's tick: opens the screen asked for it.
pub fn tick(host: &mut dyn ServerHost, id: u32) {
    let asked = ASKED.lock().unwrap().remove(&id);
    if let Some(uuid) = asked {
        open(host, uuid, id, MenuReplace::CloseFirst);
    }
}

fn open(host: &mut dyn ServerHost, uuid: u128, id: u32, replace: MenuReplace) {
    let saved = state::saved(id);
    let contents = contents(host, id);
    let (rows, items) = layout(&*host, &saved, &contents);
    let title = Text::translate("container.vulture.vulture", vec![]);
    if let Ok(Some(window)) = host.open_menu(uuid, rows, title, items, replace) {
        let storage = PER_CHEST * usize::from(saved.chests);
        OPEN.lock().unwrap().insert(
            uuid,
            Open {
                vulture: id,
                window,
                storage,
            },
        );
    }
}

/// From the `menu_closed` hook.
pub fn closed(event: &MenuClosedEvent) {
    let mut open = OPEN.lock().unwrap();
    if open.get(&event.uuid).is_some_and(|o| o.window == event.window) {
        open.remove(&event.uuid);
    }
}

/// From the `menu_clicked` hook: a button of the top row, or a click on the chests' or the inventory's slots.
pub fn clicked(host: &mut dyn ServerHost, event: &MenuClickedEvent) {
    let Some(open) = OPEN.lock().unwrap().get(&event.uuid).copied() else {
        return;
    };
    let Ok(slot) = usize::try_from(event.slot) else {
        return;
    };
    if open.window != event.window || !matches!(event.mode, PICKUP | QUICK_MOVE) {
        return;
    }
    if slot < ROW {
        button(host, event.uuid, open, slot);
        return;
    }
    let Some(player) = host
        .players()
        .server_players()
        .into_iter()
        .find(|p| p.uuid() == event.uuid)
    else {
        return;
    };
    let rows = 1 + open.storage.div_ceil(ROW);
    let mut chests = contents(host, open.vulture);
    chests.resize(2 * PER_CHEST, None);
    let mut inventory: Vec<Option<ItemStack>> = vec![None; INVENTORY];
    for (k, s) in player.inventory_slots(&*host).unwrap_or_default() {
        if let Some(cell) = inventory.get_mut(k as usize) {
            *cell = Some(s);
        }
    }
    let before = inventory.clone();
    // Vanilla's order for a stack leaving a chest: the hotbar from its end, then the inventory from its end.
    let to_inventory: Vec<usize> = (0..9).rev().chain((9..INVENTORY).rev()).collect();
    let to_chests: Vec<usize> = (0..open.storage).collect();
    if let Some(k) = (ROW..ROW + open.storage).contains(&slot).then(|| slot - ROW) {
        let Some(stack) = chests[k].take() else {
            return;
        };
        chests[k] = transfer(&*host, stack, &mut inventory, &to_inventory);
    } else if let Some(k) = slot.checked_sub(rows * ROW).filter(|&k| k < INVENTORY) {
        // The menu shows the inventory's 27 slots, then the hotbar.
        let k = if k < 27 { 9 + k } else { k - 27 };
        let Some(stack) = inventory[k].take() else {
            return;
        };
        inventory[k] = transfer(&*host, stack, &mut chests, &to_chests);
    } else {
        return;
    }
    for (k, (now, was)) in inventory.iter().zip(&before).enumerate() {
        if now.as_ref().map(|s| (s.item(), s.count())) != was.as_ref().map(|s| (s.item(), s.count())) {
            let _ = player.set_inventory_slot(host, k as u32, now.clone());
        }
    }
    set_contents(host, open.vulture, &chests);
    let (_, items) = layout(&*host, &state::saved(open.vulture), &chests);
    let _ = host.menu_set_items(event.uuid, open.window, items);
}

/// Puts `stack` into `to`'s slots in `order`, those holding the same item first, then empty ones: what doesn't fit.
fn transfer(
    host: &dyn ServerHost,
    stack: ItemStack,
    to: &mut [Option<ItemStack>],
    order: &[usize],
) -> Option<ItemStack> {
    let max = host.max_stack_size(stack.item()).unwrap_or(64);
    let mut left = stack.count();
    for &k in order {
        if let Some(s) = to[k].as_mut().filter(|s| s.is_stackable_with(&stack)) {
            let n = left.min(max.saturating_sub(s.count()));
            if n > 0 && s.set_count(s.count() + n).is_ok() {
                left -= n;
            }
        }
    }
    for &k in order {
        if left == 0 {
            break;
        }
        if to[k].is_none() {
            let mut s = stack.clone();
            let n = left.min(max);
            if s.set_count(n).is_ok() {
                to[k] = Some(s);
                left -= n;
            }
        }
    }
    let mut rest = stack;
    (left > 0 && rest.set_count(left).is_ok()).then_some(rest)
}

/// A button of the top row: sit / follow, a chest or the saddle taken off (given back with what the chest held). The
/// screen opens again in place, showing the change.
fn button(host: &mut dyn ServerHost, uuid: u128, screen: Open, slot: usize) {
    let id = screen.vulture;
    let saved = state::saved(id);
    let Some(player) = host.players().server_players().into_iter().find(|p| p.uuid() == uuid) else {
        return;
    };
    match slot {
        SIT => state::request(id, Change::ToggleSit),
        CHEST_LEFT | CHEST_RIGHT if saved.chests > 0 => {
            // The last chest put on comes off first; what it held goes back with it.
            let index = usize::from(saved.chests) - 1;
            let mut items = contents(host, id);
            items.resize(2 * PER_CHEST, None);
            let taken: Vec<ItemStack> = items[index * PER_CHEST..(index + 1) * PER_CHEST]
                .iter_mut()
                .filter_map(Option::take)
                .collect();
            set_contents(host, id, &items);
            for s in taken.into_iter().chain(stack(&*host, "minecraft:chest", 1)) {
                creature::hand::give(host, player, s);
            }
            state::request(id, Change::RemoveChest);
        }
        HARNESS if saved.harness && saved.chests == 0 => {
            if let Some(s) = stack(&*host, crate::interaction::SADDLE, 1) {
                creature::hand::give(host, player, s);
            }
            state::request(id, Change::RemoveHarness);
        }
        _ => return,
    }
    open(host, uuid, id, MenuReplace::InPlace);
}

/// The item `id` (`minecraft:saddle`), `count` of it.
fn stack(host: &dyn ServerHost, id: &str, count: u32) -> Option<ItemStack> {
    host.item_from_snbt(&format!("{{id:\"{id}\",count:{count}}}"))
}

/// Its death: its saddle, chests and their contents fall where it died.
pub fn died(host: &mut dyn ServerHost, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let saved = state::saved(id);
    let mut stacks: Vec<ItemStack> = contents(host, id).into_iter().flatten().collect();
    if saved.harness {
        stacks.extend(stack(&*host, crate::interaction::SADDLE, 1));
    }
    if saved.chests > 0 {
        stacks.extend(stack(&*host, "minecraft:chest", u32::from(saved.chests)));
    }
    for s in stacks {
        creature::hand::drop_stack(host, level, at, s);
    }
}
