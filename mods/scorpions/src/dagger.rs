//! The scorpion dagger (SPECS §5): Poisonous, and thrown.
//!
//! - **Poisonous**: its hit poisons a mob or a player 3 s (`poison`, from a melee hit or a thrown one). It is the
//!   dagger's own, shown under its name as an enchantment would be (`tooltip`) and shimmering (its item's
//!   `enchantment_glint_override`).
//! - **Thrown** (`throw`, right-click): it leaves the hand and flies on an arc where the player looks, point first and
//!   turning over forward as a thrown knife, a projectile of the library's (`creature::projectile`,
//!   `scorpions:thrown_dagger`) drawn as the dagger. The first mob or player its box meets is hurt (`DAMAGE`) and
//!   poisoned, and the dagger bounces off and falls beside it (`bounce`). Landed (the ground, a wall), it stays stuck in
//!   it, point in, until a player walks onto it and takes it back (`stuck`), as a trident. Each throw wears it by one use
//!   (broken at the last). The flying dagger holds the item in its own main hand, so a world saved with it stuck in the
//!   ground keeps it. In creative the dagger stays in hand and the thrown one is only for show.
use std::collections::{BTreeMap, BTreeSet};
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
const SPEED: f64 = 1.1;
const GRAVITY: f64 = 0.06;
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
/// How close a player walks to a stuck dagger to take it (blocks), and how long it stays stuck before it can be taken
/// (ticks: not by its thrower at once, thrown at their feet).
const PICK_UP: f64 = 1.2;
const PICK_UP_AFTER: i32 = 10;
/// How deep its point goes into what it landed in (blocks), and the least it points down stuck in the ground
/// (degrees).
const SUNK: f32 = 0.15;
const STUCK_PITCH: f32 = 55.0;
/// How fast it turns over forward in flight (degrees a tick: about a turn a second).
const SPIN: f32 = 18.0;

/// How much of its speed a dagger keeps, backwards, bouncing off a body it hit.
const BOUNCE: f64 = 0.1;

/// The daggers falling after a hit (`bounce`): they hurt nothing more.
static SPENT: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Who threw each flying dagger (its entity id).
static FLYING: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// A dagger stuck where it landed: where it is held, and how long it has been there (ticks).
#[derive(Clone, Copy)]
struct Stuck {
    at: [f64; 3],
    ticks: i32,
    /// Stuck in the ground (else in a wall): drawn blade down.
    ground: bool,
}

/// The stuck daggers, read by the drawing too (single player: the same id).
static STUCK: Mutex<BTreeMap<u32, Stuck>> = Mutex::new(BTreeMap::new());

/// Poisons `target` (a mob's or a player's id) with the dagger's venom.
pub fn poison(host: &mut dyn ServerHost, target: u32) {
    creature::mob::give_effect(host, target, "minecraft:poison", POISON);
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
    // The thrown dagger holds the item (none in creative): saved with it, given back when it is taken.
    if !creative && let Some(thrown) = host.entity(id).and_then(|e| host.living(e)) {
        let _ = thrown.set_hand_item(host, Hand::Main, Some(stack.clone()));
    }
    FLYING.lock().unwrap().insert(id, player.entity().id());
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

/// From the thrown dagger's tick: in flight, the first body it meets is hurt and poisoned and it falls there as an item;
/// landed, it stays stuck (`stuck`).
pub fn tick(host: &mut dyn ServerHost, id: u32) {
    if STUCK.lock().unwrap().contains_key(&id) {
        stuck(host, id);
        return;
    }
    projectile::tick_post(host, id);
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let mut end = projectile::ended(id);
    if end.is_none() && !SPENT.lock().unwrap().contains(&id) {
        let thrower = FLYING.lock().unwrap().get(&id).copied().unwrap_or(u32::MAX);
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
    FLYING.lock().unwrap().remove(&id);
    match end {
        // It hit a body: it bounces off and falls beside it, where it sticks as landed.
        End::Hit => {
            bounce(host, id, at, level);
            e.discard(host);
            forget(id);
        }
        // It landed: it stays stuck there, held still.
        End::Landed => {
            let sound = "minecraft:item.trident.hit_ground";
            level.play_sound(host, sound, at, SoundSource::Neutral, 0.8, 1.3, None);
            let ground = e.on_ground(host).unwrap_or(true);
            STUCK.lock().unwrap().insert(id, Stuck { at, ticks: 0, ground });
            hold_still(host, id, at);
        }
    }
}

/// A dagger that hit a body (`id`, at `at`): a new one, holding its item, bounces back off it and falls, harmless.
fn bounce(host: &mut dyn ServerHost, id: u32, at: [f64; 3], level: mod_api::LevelRef) {
    let v = projectile::velocity(id).unwrap_or([0.0; 3]);
    let launch = Launch {
        velocity: [-v[0] * BOUNCE, 0.2, -v[2] * BOUNCE],
        gravity: GRAVITY,
        life: LIFE,
        target: None,
    };
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    let Some(new) = projectile::launch(host, &dimension, THROWN, at, launch) else {
        return;
    };
    let held = host
        .entity(id)
        .and_then(|e| host.living(e))
        .and_then(|l| l.hand_item(&*host, Hand::Main));
    if let (Some(stack), Some(living)) = (held, host.entity(new).and_then(|e| host.living(e))) {
        let _ = living.set_hand_item(host, Hand::Main, Some(stack));
    }
    SPENT.lock().unwrap().insert(new);
}

/// The item the thrown dagger holds, one use worn, taken from it; none in creative, or when that use broke it.
fn take(host: &mut dyn ServerHost, id: u32) -> Option<ItemStack> {
    let living = host.entity(id).and_then(|e| host.living(e))?;
    let mut stack = living.hand_item(&*host, Hand::Main)?;
    let _ = living.set_hand_item(host, Hand::Main, None);
    stack.set_damage(stack.damage() + 1);
    if stack.damage() >= USES {
        if let Some(e) = host.entity(id)
            && let (Some(at), Some(level)) = (e.position(host), e.level(host))
        {
            let sound = "minecraft:entity.item.break";
            level.play_sound(host, sound, at, SoundSource::Neutral, 0.8, 1.0, None);
        }
        return None;
    }
    Some(stack)
}

/// Keeps a stuck dagger where it landed: no gravity, no push.
fn hold_still(host: &mut dyn ServerHost, id: u32, at: [f64; 3]) {
    let Some(e) = host.entity(id) else {
        return;
    };
    e.set_no_gravity(host, true);
    e.set_velocity(host, [0.0; 3]);
    let moved = e
        .position(host)
        .is_some_and(|p| (0..3).any(|k| (p[k] - at[k]).abs() > 0.01));
    if moved {
        e.teleport_to(host, at, None);
    }
}

/// A stuck dagger's tick: held still; a player walking onto it takes it back (the item to their inventory).
fn stuck(host: &mut dyn ServerHost, id: u32) {
    let Some(state) = STUCK.lock().unwrap().get_mut(&id).map(|s| {
        s.ticks += 1;
        *s
    }) else {
        return;
    };
    hold_still(host, id, state.at);
    if state.ticks < PICK_UP_AFTER {
        return;
    }
    let level = host.entity(id).and_then(|e| e.level(host));
    let taker = host.players().server_players().into_iter().find(|p| {
        let p_level = p.level(&*host);
        let near = p.entity().position(&*host).is_some_and(|p| {
            let d = [p[0] - state.at[0], p[1] + 0.5 - state.at[1], p[2] - state.at[2]];
            d.iter().map(|v| v * v).sum::<f64>() < PICK_UP * PICK_UP
        });
        near && p_level == level && p.is_spectator(&*host) != Some(true)
    });
    let Some(player) = taker else {
        return;
    };
    if let Some(stack) = take(host, id) {
        creature::hand::give(host, player, stack);
    }
    if let Some(e) = host.entity(id) {
        if let Some(level) = level {
            let sound = "minecraft:entity.item.pickup";
            level.play_sound(host, sound, state.at, SoundSource::Players, 0.2, 1.4, None);
        }
        e.discard(host);
    }
    forget(id);
}

/// From the mod's `living_death` (or a removal).
pub fn forget(id: u32) {
    FLYING.lock().unwrap().remove(&id);
    SPENT.lock().unwrap().remove(&id);
    STUCK.lock().unwrap().remove(&id);
    projectile::forget(id);
}

/// Draws a thrown dagger: the dagger itself, point first along where it flies; stuck, as it landed, its point sunk in.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let Some(&key) = KEY.get() else {
            return Ok(());
        };
        let v = projectile::velocity(entity.id).unwrap_or([0.0, -1.0, 1.0]);
        let flat = v[0].hypot(v[2]);
        let yaw = v[0].atan2(v[2]).to_degrees() as f32;
        let mut pitch = (-v[1]).atan2(flat).to_degrees() as f32;
        let stuck = STUCK.lock().unwrap().get(&entity.id).copied();
        let sunk = if stuck.is_some() { SUNK } else { 0.0 };
        // In the ground, its blade goes in, steeper than it came down; in flight it turns over forward, as a thrown
        // knife.
        match stuck {
            Some(s) if s.ground => pitch = pitch.max(STUCK_PITCH),
            Some(_) => {}
            None => pitch += entity.age * SPIN,
        }
        draw.push();
        draw.translate([0.0, SIZE.height / 2.0 - sunk, 0.0]);
        // Its heading, then its pitch: the item's blade, drawn diagonally (its point up and to the right), first laid
        // point forward (+z), its flat side upright.
        draw.rotate_about([0.0, 1.0, 0.0], yaw, [0.0; 3]);
        draw.rotate_about([1.0, 0.0, 0.0], pitch, [0.0; 3]);
        draw.rotate_about([0.0, 1.0, 0.0], -90.0, [0.0; 3]);
        draw.rotate_about([0.0, 0.0, 1.0], -45.0, [0.0; 3]);
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
