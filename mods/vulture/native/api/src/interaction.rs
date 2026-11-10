//! Right-click on a vulture (SPECS §6), on the server, with the acting player's hand (`interact_entity` hook).
//!
//! It changes the mob through `state::request` (applied by its goal on its next tick), mounts with `Host::mount` (as
//! `/ride` does, on the server, so its goals see the rider), and always takes the click (no trading screen). Not an
//! injection into `native_interact_entity`: that one sees a copy of the mob (workaround for horizon-rust/horizon#13972).
//!
//! Only its owner gets anything from it:
//! - sneaking: its screen (`screen.rs`);
//! - holding a lead: tied on (Horizon refuses leads on the trader it copies); on a lead, any click takes it off;
//! - holding the vulture harness: puts it on (not on a baby); a chest: adds one (two at most, harness needed);
//! - holding rotten flesh, grown and not resting from breeding: in love for 30 s (SPECS §8);
//! - holding raw meat: a baby grows a minute older; a hurt vulture heals 4;
//! - otherwise: mounts it, with a harness on and grown up; else sit / follow.
use horizon_client::items::ItemStack;
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
    let creative = player.game_mode == 1;
    // A lead: tied on, as on any animal, wild or tamed, by anyone; on a lead, a click takes it off and gives it back.
    if state::leashed(id) {
        state::request(id, Change::Unleash);
        if !creative {
            let _ = host.give_stack(player.uuid, ItemStack::new("minecraft:lead", 1));
        }
        return;
    }
    if held == "minecraft:lead" {
        if take(host, creative, &held) {
            state::request(id, Change::Leash(uuid_ints(player.uuid)));
        }
        return;
    }
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
    if player.sneaking {
        screen::ask(host, id);
        return;
    }
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
                if matches!(change, Change::InLove) {
                    creature::bestiary::learn(host, player.uuid, crate::KIND, crate::BREED_FACT);
                }
                if matches!(change, Change::Heal | Change::Grow | Change::InLove) {
                    sounds::queue(id, sounds::EAT, 1.0, 1.0);
                }
            }
        }
        // An empty hand, or anything else: up on its back, else sit / follow.
        None if saved.harness && !baby => {
            if saved.sitting {
                state::request(id, Change::ToggleSit);
            }
            host.mount(player.entity_id as u32, id);
        }
        None => state::request(id, Change::ToggleSit),
    }
}
