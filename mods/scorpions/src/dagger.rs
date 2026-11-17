//! The scorpion dagger (SPECS §5): Poisonous, and thrown.
//!
//! - **Poisonous**: its hit poisons a mob or a player 3 s (`poison`, from a melee hit or a thrown one). It is the
//!   dagger's own, shown under its name as an enchantment would be (`tooltip`) and shimmering (its item's
//!   `enchantment_glint_override`).
//! - **Thrown** (`throw`, right-click): it leaves the hand and flies where the player looks, a projectile of the library's
//!   (`creature::projectile`, `scorpions:thrown_dagger`) drawn as the dagger turning end over end. The first mob or
//!   player its box meets is hurt (`DAMAGE`) and poisoned; hit or landed, it falls as an item, worn by one use (broken at
//!   the last), to be picked up again. In creative the dagger stays in hand and the thrown one is only for show.
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use creature::projectile::{self, End, Launch};
use mod_api::{
    BlockEntityDrawContext, ClientHost, EntityRenderState, EntityRenderer, Hand, HostSide, InteractionResult,
    ItemDisplayContext, ItemKey, ItemPart, ItemStack, ServerHost, SoundSource, Text, TooltipEvent, UseItemEvent,
};

pub const DAGGER: &str = "scorpions:scorpion_dagger";
pub const THROWN: &str = "scorpions:thrown_dagger";
/// The thrown dagger's box (blocks: width, height, eye height).
pub const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.3,
    height: 0.3,
    eye_height: 0.15,
};
/// How fast it leaves the hand, its gravity (blocks a tick), how long it may fly (ticks).
const SPEED: f64 = 1.6;
const GRAVITY: f64 = 0.04;
const LIFE: i32 = 200;
/// What a thrown dagger deals, and how close its box must come to a body to hit it (blocks).
const DAMAGE: f32 = 4.0;
const REACH: f64 = 0.5;
/// Its uses (its item's `max_damage`).
const USES: u32 = 64;
/// How long the venom poisons (ticks).
const POISON: i32 = 60;
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
pub fn poison(host: &mut dyn ServerHost, target: u32) {
    crate::poison(host, target, "minecraft:poison", POISON);
}

/// The dagger's item key, to draw it (a renderer has no host to look it up).
pub static KEY: OnceLock<ItemKey> = OnceLock::new();

/// The dagger: right-click throws it.
pub struct Dagger;

#[mod_api::sunrise_part]
impl ItemPart for Dagger {
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        if let HostSide::Server(host) = host {
            throw(host, event.player, event.hand, event.stack);
        }
        Some(InteractionResult::Consume)
    }

    /// The dagger's Poisonous line.
    fn tooltip(&self, host: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        tooltip(host, event);
    }
}

/// `player` throws the dagger `stack` held in `hand`.
fn throw(host: &mut dyn ServerHost, player: u128, hand: Hand, stack: &ItemStack) {
    let Some(player) = host.player(player) else {
        return;
    };
    let (Some((eye, look)), Some(level)) = (creature::hand::eye_and_look(&*host, player), player.level(&*host)) else {
        return;
    };
    let from = [
        eye[0] + look[0] * 0.6,
        eye[1] + look[1] * 0.6 - 0.25,
        eye[2] + look[2] * 0.6,
    ];
    let launch = Launch {
        velocity: look.map(|k| k * SPEED),
        gravity: GRAVITY,
        life: LIFE,
        target: None,
    };
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    let Some(id) = projectile::launch(host, &dimension, THROWN, from, launch) else {
        return;
    };
    let creative = player.has_infinite_materials(&*host).unwrap_or(false);
    let kept = (!creative).then(|| stack.clone());
    FLYING.lock().unwrap().insert(
        id,
        Flying {
            thrower: player.entity().id(),
            stack: kept,
        },
    );
    creature::hand::use_one(host, player, hand);
    level.play_sound(
        host,
        "minecraft:item.trident.throw",
        eye,
        SoundSource::Neutral,
        0.8,
        1.4,
        None,
    );
}

/// From the thrown dagger's tick: the first body it meets is hurt and poisoned; hit or landed, it falls as an item.
pub fn tick(host: &mut dyn ServerHost, id: u32) {
    projectile::tick_post(host, id);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let mut end = projectile::ended(id);
    if end.is_none() {
        let thrower = FLYING.lock().unwrap().get(&id).map_or(u32::MAX, |f| f.thrower);
        if let Some(&target) = projectile::hits(&*host, level, at, THROWN, thrower, REACH).first()
            && let Some(t) = host.entity(target)
        {
            let attacker = host.entity(thrower);
            t.hurt(host, "minecraft:thrown", DAMAGE, Some(e), attacker);
            poison(host, target);
            level.play_sound(
                host,
                "minecraft:item.trident.hit",
                at,
                SoundSource::Neutral,
                1.0,
                1.3,
                None,
            );
            projectile::stop(id, End::Hit);
            end = Some(End::Hit);
        }
    }
    let Some(end) = end else {
        return;
    };
    if end == End::Landed {
        let sound = "minecraft:item.trident.hit_ground";
        level.play_sound(host, sound, at, SoundSource::Neutral, 0.8, 1.3, None);
    }
    // It falls as an item, one use worn; at its last use it breaks.
    if let Some(mut stack) = FLYING.lock().unwrap().remove(&id).and_then(|f| f.stack) {
        stack.set_damage(stack.damage() + 1);
        if stack.damage() >= USES {
            level.play_sound(
                host,
                "minecraft:entity.item.break",
                at,
                SoundSource::Neutral,
                0.8,
                1.0,
                None,
            );
        } else {
            creature::hand::drop_stack(host, level, [at[0], at[1] + 0.1, at[2]], stack);
        }
    }
    e.discard(host);
    projectile::forget(id);
}

/// From the mod's `living_death` (or a removal).
pub fn forget(id: u32) {
    FLYING.lock().unwrap().remove(&id);
    projectile::forget(id);
}

/// Draws a thrown dagger: the dagger itself, pointing where it flies and turning end over end.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let Some(&key) = KEY.get() else {
            return Ok(());
        };
        let v = projectile::velocity(entity.id).unwrap_or([0.0, 0.0, 1.0]);
        let yaw = (-v[0]).atan2(v[2]).to_degrees() as f32;
        draw.push();
        draw.translate([0.0, SIZE.height / 2.0, 0.0]);
        draw.rotate_about([0.0, 1.0, 0.0], yaw, [0.0; 3]);
        draw.rotate_about([1.0, 0.0, 0.0], entity.age * SPIN, [0.0; 3]);
        draw.scale([0.8; 3]);
        draw.item(&ItemStack::of(key), ItemDisplayContext::Fixed, None, 0);
        draw.pop();
        Ok(())
    }
}

/// The dagger says it is Poisonous, under its name, as an enchantment would.
fn tooltip(host: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
    if creature::hand::is(host, event.stack, DAGGER) {
        let at = event.lines.len().min(1);
        let line = Text::translate("enchantment.scorpions.poisonous", vec![]);
        event.lines.insert(at, (line, GREY));
    }
}
