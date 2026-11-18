//! Right-click on a vulture (SPECS §6), on the server, with the clicking hand (`entity_interact` hook, answered on both
//! sides so the client opens no trading screen).
//!
//! It changes the vulture through `state::request` (applied on its next tick) and mounts with
//! `EntityRef::start_riding`, as `/ride` does.
//!
//! Only its owner gets anything from it:
//! - sneaking: its screen (`screen.rs`);
//! - holding a lead: tied on (`EntityRef::leash_to`), by anyone; on a lead, any click takes it off;
//! - holding a saddle (vanilla's): puts it on, drawn as a seat and a breast strap (not on a baby); a chest: adds one
//!   (two at most, saddle needed);
//! - holding rotten flesh, grown and not resting from breeding: in love for 30 s (SPECS §8);
//! - holding raw meat: a baby grows a minute older; a hurt vulture heals 4;
//! - otherwise: mounts it, saddled and grown up; else sit / follow.
use mod_api::{EntityInteractEvent, Hand, PlayerRef, ServerHost, Text};

use crate::state::{self, Change};
use crate::{screen, sounds};

/// What makes it rideable: a plain saddle (its state still calls the gear its harness).
pub const SADDLE: &str = "minecraft:saddle";

const RAW_MEATS: [&str; 5] = [
    "minecraft:chicken",
    "minecraft:beef",
    "minecraft:porkchop",
    "minecraft:mutton",
    "minecraft:rabbit",
];

/// One of what `player` holds in `hand` used up (none in creative, as vanilla).
fn take(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand) {
    creature::hand::use_one(host, player, hand);
}

pub fn clicked(host: &mut dyn ServerHost, id: u32, event: &EntityInteractEvent<'_>) {
    let (Some(player), Some(vulture)) = (host.players().acting_player(), host.entity(id)) else {
        return;
    };
    let saved = state::saved(id);
    let held = event.stack.map(|s| creature::hand::id(&*host, s)).unwrap_or_default();
    let hand = event.hand;
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    // A lead: tied on, as on any animal, wild or tamed, by anyone; on a lead, a click takes it off and gives it back.
    if vulture.leash_holder(&*host).is_some() {
        vulture.detach_leash(host, !creative);
        state::request(id, Change::Unleash);
        return;
    }
    if held == "minecraft:lead" {
        if vulture.leash_to(host, player.entity()) {
            take(host, player, hand);
            state::request(id, Change::Leash);
        }
        return;
    }
    if saved.owner != Some(creature::friend::ints(player.uuid())) {
        // Say why nothing happens, holding what only works on one's own vulture.
        if matches!(held.as_str(), SADDLE | "minecraft:chest" | "minecraft:rotten_flesh") {
            let key = if saved.owner.is_some() {
                "message.vulture.not_yours"
            } else {
                "message.vulture.not_tamed"
            };
            let _ = player.action_bar(host, Text::translate(key, vec![]));
        }
        return;
    }
    if player.entity().is_shift_key_down(&*host).unwrap_or(false) {
        screen::ask(host, player, id);
        return;
    }
    let baby = saved.age < 0;
    let hurt = state::hurt(id);
    let change = match held.as_str() {
        SADDLE if !saved.harness && !baby => Some(Change::PutHarness),
        "minecraft:chest" if saved.harness && saved.chests < 2 => Some(Change::AddChest),
        "minecraft:rotten_flesh" if saved.age == 0 && !state::in_love(id) => Some(Change::InLove),
        meat if RAW_MEATS.contains(&meat) && baby => Some(Change::Grow),
        meat if RAW_MEATS.contains(&meat) && hurt => Some(Change::Heal),
        _ => None,
    };
    match change {
        Some(change) => {
            take(host, player, hand);
            state::request(id, change);
            if matches!(change, Change::InLove) {
                creature::bestiary::learn(host, player.uuid(), crate::KIND, crate::BREED_FACT);
            }
            if matches!(change, Change::Heal | Change::Grow | Change::InLove) {
                sounds::queue(id, sounds::EAT, 1.0, 1.0);
            }
        }
        // An empty hand, or anything else: up on its back, else sit / follow.
        None if saved.harness && !baby => {
            if saved.sitting {
                state::request(id, Change::ToggleSit);
            }
            let _ = player.entity().start_riding(host, vulture, true);
        }
        None => state::request(id, Change::ToggleSit),
    }
}
