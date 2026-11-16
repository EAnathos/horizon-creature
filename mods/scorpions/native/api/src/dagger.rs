//! The scorpion dagger (SPECS §5): Poisonous, and thrown.
//!
//! - **Poisonous**: its hit poisons a mob or a player 3 s (`poison`, from a melee hit or a thrown one). It is the
//!   dagger's own, shown under its name as an enchantment would be (`tooltip`) and shimmering (its item's
//!   `enchantment_glint_override`): Horizon at our pin loads no mod enchantment (docs/HORIZON-ISSUES.md).
//! - **Thrown** (`throw`, right-click): it leaves the hand and flies where the player looks, a projectile of the library's
//!   (`creature::projectile`, `scorpions:thrown_dagger`) drawn as the dagger turning end over end. The first mob or
//!   player its box meets is hurt (`DAMAGE`) and poisoned; hit or landed, it falls as an item, worn by one use (broken at
//!   the last), to be picked up again. In creative the dagger stays in hand and the thrown one is only for show.
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::projectile::{self, End, Launch};
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{DisplayContext, Host, MobSubmitEvent, Text, TooltipEvent};

pub const DAGGER: &str = "scorpions:scorpion_dagger";
pub const THROWN: &str = "scorpions:thrown_dagger";
/// The thrown dagger's box (blocks: width, height, eye height).
pub const SIZE: (f64, f64, f64) = (0.3, 0.3, 0.15);
/// How fast it leaves the hand, its gravity (blocks a tick), how long it may fly (ticks).
const SPEED: f64 = 1.6;
const GRAVITY: f64 = 0.04;
const LIFE: i32 = 200;
/// What a thrown dagger deals, and how close its box must come to a body to hit it (blocks).
const DAMAGE: f32 = 4.0;
const REACH: f64 = 0.5;
/// Its uses (its item's `max_damage`).
const USES: u32 = 64;
/// How long the venom poisons a player (ticks); a mob's is the mod's `VENOM_TICKS`.
const PLAYER_POISON: i32 = 60;
/// The Poisonous line's colour, as tooltip lines take it (`rgba(170, 170, 170, 255)`: an enchantment's grey).
const GREY: u32 = 0xFFAA_AAAA;
/// How fast it turns end over end in flight (degrees a tick).
const SPIN: f32 = 40.0;

/// A thrown dagger in flight: who threw it, and the dagger to give back when it falls (none in creative).
struct Flying {
    thrower: u32,
    stack: Option<ItemStack>,
}

static FLYING: Mutex<BTreeMap<u32, Flying>> = Mutex::new(BTreeMap::new());

/// Poisons `target` (a mob's or a player's id) with the dagger's venom.
pub fn poison(host: &mut dyn Host, target: u32) {
    match host.server_players().into_iter().find(|p| p.entity_id as u32 == target) {
        Some(player) => {
            host.add_effect(player.uuid, &crate::poison_effect("minecraft:poison", PLAYER_POISON));
        }
        None => crate::envenom(target),
    }
}

/// From the mod's `use_item`: a right-click with the dagger throws it. Whether it was the dagger.
pub fn throw(host: &mut dyn Host, stack: &ItemStack) -> bool {
    if &*stack.item != DAGGER {
        return false;
    }
    let Some(player) = host.acting_player() else {
        return true;
    };
    let (yaw, pitch) = (f64::from(player.yaw).to_radians(), f64::from(player.pitch).to_radians());
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let eye = [player.position[0], player.position[1] + 1.5, player.position[2]];
    let from = [
        eye[0] + look[0] * 0.6,
        eye[1] + look[1] * 0.6 - 0.15,
        eye[2] + look[2] * 0.6,
    ];
    let launch = Launch {
        velocity: look.map(|k| k * SPEED),
        gravity: GRAVITY,
        life: LIFE,
        target: None,
    };
    let Some(id) = projectile::launch(host, &player.dimension, THROWN, from, launch) else {
        return true;
    };
    let creative = player.game_mode == 1;
    let kept = (!creative).then(|| stack.clone());
    FLYING.lock().unwrap().insert(
        id,
        Flying {
            thrower: player.entity_id as u32,
            stack: kept,
        },
    );
    creature::hand::use_one(host, stack);
    host.play_sound("minecraft:item.trident.throw", eye, creature::sounds::NEUTRAL, 0.8, 1.4);
    true
}

/// From the thrown dagger's tick: the first body it meets is hurt and poisoned; hit or landed, it falls as an item.
pub fn tick(host: &mut dyn Host, dimension: &str, id: u32) {
    let Some(at) = host.entity(id).and_then(|e| host.entity_position(e)) else {
        return;
    };
    let mut end = projectile::ended(id);
    if end.is_none() {
        let thrower = FLYING.lock().unwrap().get(&id).map_or(u32::MAX, |f| f.thrower);
        if let Some(&target) = projectile::hits(host, dimension, at, THROWN, thrower, REACH).first() {
            let attacker = (thrower != u32::MAX).then_some(thrower);
            host.hurt_entity(dimension, target, DAMAGE, "minecraft:thrown", attacker);
            poison(host, target);
            host.play_sound("minecraft:item.trident.hit", at, creature::sounds::NEUTRAL, 1.0, 1.3);
            projectile::stop(id, End::Hit);
            end = Some(End::Hit);
        }
    }
    let Some(end) = end else {
        return;
    };
    if end == End::Landed {
        host.play_sound(
            "minecraft:item.trident.hit_ground",
            at,
            creature::sounds::NEUTRAL,
            0.8,
            1.3,
        );
    }
    // It falls as an item, one use worn; at its last use it breaks.
    if let Some(mut stack) = FLYING.lock().unwrap().remove(&id).and_then(|f| f.stack) {
        stack.damage += 1;
        if stack.damage >= USES {
            host.play_sound("minecraft:entity.item.break", at, creature::sounds::NEUTRAL, 0.8, 1.0);
        } else {
            creature::hand::drop_stack(host, dimension, [at[0], at[1] + 0.1, at[2]], &stack);
        }
    }
    host.remove_mob(dimension, id);
    projectile::forget(id);
}

/// From the mod's `living_death` (or a removal).
pub fn forget(id: u32) {
    FLYING.lock().unwrap().remove(&id);
    projectile::forget(id);
}

/// Draws a thrown dagger: the dagger itself, pointing where it flies and turning end over end.
pub fn submit(host: &dyn Host, event: &mut MobSubmitEvent<'_>) {
    let mob = event.mob;
    let v = projectile::velocity(mob.id).unwrap_or([0.0, 0.0, 1.0]);
    let yaw = (-v[0]).atan2(v[2]).to_degrees() as f32;
    let draw = &mut *event.draw;
    draw.push();
    draw.translate([0.0, SIZE.1 as f32 / 2.0, 0.0]);
    draw.rotate_about([0.0, 1.0, 0.0], yaw, [0.0; 3]);
    draw.rotate_about([1.0, 0.0, 0.0], mob.age * SPIN, [0.0; 3]);
    draw.scale([0.8; 3]);
    host.draw_item(draw, &ItemStack::new(DAGGER, 1), DisplayContext::Fixed, None, 0);
    draw.pop();
}

/// The dagger says it is Poisonous, under its name, as an enchantment would.
pub fn tooltip(event: &mut TooltipEvent<'_>) {
    if &*event.stack.item == DAGGER {
        let at = event.lines.len().min(1);
        event
            .lines
            .insert(at, (Text::translate("enchantment.scorpions.poisonous", &[]), GREY));
    }
}
