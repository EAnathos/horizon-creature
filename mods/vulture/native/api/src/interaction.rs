//! Right-click on a vulture (SPECS §6), on the server, with the acting player's hand (`interact_entity` hook).
//!
//! The `interact.rs` injection, which runs first, only mounts: it sees a copy of the mob that the game refreshes from
//! the one its AI ticks, so what it would change there is lost. Every other click comes here, which changes the mob
//! through `state::request` (applied by its goal on its next tick) and always takes the click (no trading screen).
//!
//! Only its owner gets anything from it:
//! - sneaking: its screen (`screen.rs`);
//! - holding the vulture harness: puts it on (not on a baby); a chest: adds one (two at most, harness needed);
//! - holding rotten flesh, grown and not resting from breeding: in love for 30 s (SPECS §8);
//! - holding raw meat: a baby grows a minute older; a hurt vulture heals 4;
//! - otherwise (no harness, or a baby): sit / follow. With a harness the injection has mounted it already.
use horizon_client::mod_api::{Host, Text};

use crate::state::{self, Change};
use crate::{screen, sounds};

const RAW_MEATS: [&str; 5] = [
    "minecraft:chicken",
    "minecraft:beef",
    "minecraft:porkchop",
    "minecraft:mutton",
    "minecraft:rabbit",
];

fn uuid_ints(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as i32,
        (uuid >> 64) as i32,
        (uuid >> 32) as i32,
        uuid as i32,
    ]
}

/// One `item` from the player's hands (none taken in creative, as vanilla). Whether they had one.
fn take(host: &mut dyn Host, creative: bool, item: &str) -> bool {
    creative || host.take_items(item, 1) == 1
}

pub fn clicked(host: &mut dyn Host, id: u32) {
    let Some(player) = host.acting_player() else {
        return;
    };
    let saved = state::saved(id);
    let held = host.held().map(|(item, _, _)| item).unwrap_or_default();
    if saved.owner != Some(uuid_ints(player.uuid)) {
        // Say why nothing happens, holding what only works on one's own vulture.
        if matches!(
            held.as_str(),
            "vulture:harness" | "minecraft:chest" | "minecraft:rotten_flesh"
        ) {
            let key = if saved.owner.is_some() {
                "message.vulture.not_yours"
            } else {
                "message.vulture.not_tamed"
            };
            host.action_bar(player.uuid, Text::translate(key, &[]));
        }
        return;
    }
    if host.sneaking() {
        screen::open(host, id);
        return;
    }
    let creative = player.game_mode == 1;
    let baby = saved.age < 0;
    let hurt = state::hurt(id);
    let change = match held.as_str() {
        "vulture:harness" if !saved.harness && !baby => Some(Change::PutHarness),
        "minecraft:chest" if saved.harness && saved.chests < 2 => Some(Change::AddChest),
        "minecraft:rotten_flesh" if saved.age == 0 && !state::in_love(id) => Some(Change::InLove),
        meat if RAW_MEATS.contains(&meat) && baby => Some(Change::Grow),
        meat if RAW_MEATS.contains(&meat) && hurt => Some(Change::Heal),
        _ => None,
    };
    match change {
        Some(change) => {
            if take(host, creative, &held) {
                state::request(id, change);
                if matches!(change, Change::Heal | Change::Grow | Change::InLove) {
                    sounds::queue(id, sounds::EAT, 1.0, 1.0);
                }
            }
        }
        // An empty hand, or anything else: sit / follow (a harnessed grown vulture was mounted by the injection).
        None => state::request(id, Change::ToggleSit),
    }
}
