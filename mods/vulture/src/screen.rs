//! The vulture's screen and its chests' contents (SPECS §6.1, §6.3).
//!
//! The contents of its chests are saved in the `vulture:chests` attachment (each slot's item as its NBT, read back
//! with a host: `ItemStack::from_nbt`); its death gives everything back on the ground, with its saddle and chests.
//!
//! Its screen (sneak + right-click by its owner: the saddle, sit / follow, its chests' slots) was one of Horizon's
//! server menus; on Sunrise it becomes a mod menu type, still to port (docs/SUNRISE-PORT.md): until then the click
//! says so (`ask`).
use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, ItemStack, PlayerRef, Registrar, ServerHost,
    Tag, Text,
};

use crate::state;

/// Slots per chest.
pub const PER_CHEST: usize = 15;

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

/// Sneak + right-click on its own vulture (`interaction.rs` checked it is the owner): its screen, not ported yet.
pub fn ask(host: &mut dyn ServerHost, player: PlayerRef, _: u32) {
    let _ = player.action_bar(host, Text::translate("message.vulture.screen_unavailable", vec![]));
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
    let contents = CHESTS.get(host, Holder::Entity(id)).unwrap_or_default();
    let mut stacks: Vec<ItemStack> = contents
        .iter()
        .flatten()
        .filter_map(|c| ItemStack::from_nbt(&*host, c))
        .collect();
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
