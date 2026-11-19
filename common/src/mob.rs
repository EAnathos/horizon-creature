//! What every creature of the series needs from its mob, since each is a copy of a vanilla mob (the wandering trader):
//! its own size (SPECS §3.2), its template's goals taken off for its own (§6.8), monsters leaving it alone (§3.4), and
//! for small wild ones, despawning far from players. Its silence is its entity type's (`"sounds"` in its JSON).
//!
//! A mod registers each of its creatures once, from its `register` hook (`ours`); the library's hooks do the rest:
//! `entity_size` gives the creature its box, `entity_join_level` takes its template's goals off, `living_change_target`
//! keeps monsters off it.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{ActiveEffect, EntitySizeEvent, EntityTickPostEvent, Host, HostSide, MobRef, ServerHost, Tag};

use crate::colours::Random;

/// A creature's box: width, height and eye height, in blocks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
}

/// One entity `entity_tick_post` tells: the level it ticked in, its ID and its type.
pub struct Ticked<'a> {
    pub dimension: &'a str,
    pub entity: u32,
    pub kind: &'a str,
}

/// From a mod's `entity_tick_post`: `tick` for each entity the server tells, one at a time (the game tells them in
/// batches, on both sides; the series' creatures live on the server, so a client's are left alone).
pub fn each_ticked(
    host: HostSide,
    event: &EntityTickPostEvent<'_>,
    mut tick: impl FnMut(&mut dyn ServerHost, &Ticked<'_>),
) {
    let HostSide::Server(host) = host else {
        return;
    };
    for (&entity, &kind) in event.entities.iter().zip(event.kinds) {
        tick(
            host,
            &Ticked {
                dimension: event.dimension,
                entity,
                kind,
            },
        );
    }
}

/// The series' creatures by entity type, with their size.
static OURS: Mutex<BTreeMap<&'static str, Size>> = Mutex::new(BTreeMap::new());

/// The prefix of our goals' names (`Goal::name`): every other goal of one of our creatures is its template's, taken off
/// when it joins the level.
pub const GOAL_PREFIX: &str = "creature:";

/// From the mod's `register` hook, for each of its creatures (`worms:worm`): its own box, its template's goals taken
/// off for the mod's own (the mod adds them from its `EntityPart`), and spared by monsters (`spared`).
pub fn ours(kind: &'static str, size: Size) {
    OURS.lock().unwrap().insert(kind, size);
}

/// The box of one of the series' creatures.
pub fn size_of(kind: &str) -> Option<Size> {
    OURS.lock().unwrap().get(kind).copied()
}

/// Our creatures' template's height (`minecraft:wandering_trader`), which a client keeps for them: `entity_size` isn't
/// asked for a client's copies (sunrise-org/sunrise#1134).
pub const TEMPLATE_HEIGHT: f32 = 1.95;

/// The base `minecraft:scale` of a creature much taller than its template, set so a client's box (the template's,
/// scaled) is as tall as its own: the player's melee and clicks pick it there (sunrise-org/sunrise#1134). Its renderer
/// divides it back out (`client_box_scale`).
static CLIENT_SCALE: Mutex<BTreeMap<&'static str, f32>> = Mutex::new(BTreeMap::new());

/// From the mod's `register`, for a creature much taller than its template: the scale each one gets as it joins (the
/// mod sets `minecraft:scale` on it), its height over the template's. A creature made only for drawing (the book's)
/// never joins: it keeps the template's size.
pub fn scale_client_box(kind: &'static str) -> f32 {
    let scale = size_of(kind).map_or(1.0, |s| s.height / TEMPLATE_HEIGHT);
    CLIENT_SCALE.lock().unwrap().insert(kind, scale);
    scale
}

/// The base scale `scale_client_box` gave `kind`, 1 for the others.
pub fn client_box_scale(kind: &str) -> f32 {
    CLIENT_SCALE.lock().unwrap().get(kind).copied().unwrap_or(1.0)
}

/// Whether `kind` is one of the series' creatures.
pub fn is_ours(kind: &str) -> bool {
    OURS.lock().unwrap().contains_key(kind)
}

/// From the library's `entity_size`: our creature's own box in place of its template's (a trader's).
pub fn size(host: &dyn Host, event: &mut EntitySizeEvent<'_>) {
    let Some(kind) = event.entity.kind(host) else {
        return;
    };
    if let Some(s) = OURS.lock().unwrap().get(kind.id(host).as_str()) {
        *event.size = [s.width, s.height, s.eye_height];
    }
}

/// From the library's `entity_join_level`: our creature `mob` loses its template's goals, keeping ours (named with
/// `GOAL_PREFIX`) and `float` (it swims up in water). Goals are not saved, so this runs at every join, a load from disk
/// included.
pub fn clear_template_goals(host: &mut dyn ServerHost, mob: MobRef) {
    for list in [mob.goals(host), mob.target_goals(host)].into_iter().flatten() {
        for goal in list {
            if !goal.id.starts_with(GOAL_PREFIX) && goal.id != "float" {
                mob.remove_goal(host, goal.id);
                mob.remove_target_goal(host, goal.id);
            }
        }
    }
}

/// Whether a mob of kind `entity` may not take a mob of kind `target` as its target (SPECS §3.4): our creatures are
/// copies of the wandering trader, which zombies and illagers hunt as they hunt villagers. From the library's
/// `living_change_target`, which cancels the change. Our own creatures may still target each other.
pub fn spared(entity: &str, target: &str) -> bool {
    is_ours(target) && !is_ours(entity)
}

/// Gives `target` (a mob or a player) effect `id` (`minecraft:poison`) for `ticks`.
pub fn give_effect(host: &mut dyn ServerHost, target: u32, id: &str, ticks: i32) {
    let (Some(effect), Some(living)) = (host.effect_key(id), host.entity(target).and_then(|e| host.living(e))) else {
        return;
    };
    living.add_effect(host, &ActiveEffect::new(effect, ticks, 0, false, true, true));
}

/// How far the nearest player in level `dimension` is from `at`, in blocks (`f64::MAX` with none there).
pub fn nearest_player(host: &dyn ServerHost, dimension: &str, at: [f64; 3]) -> f64 {
    host.players()
        .server_players()
        .into_iter()
        .filter(|p| p.dimension(host).is_some_and(|d| d.to_string() == dimension))
        .filter_map(|p| p.entity().position(host))
        .map(|p| crate::body::distance(p, at))
        .fold(f64::MAX, f64::min)
}

/// Despawning, as vanilla's `Mob.checkDespawn` for ambient creatures: gone at once with no player within `GONE`
/// blocks, and one chance in `CHANCE` a tick with none within `NEAR`.
const GONE: f64 = 128.0;
const NEAR: f64 = 32.0;
const CHANCE: u64 = 800;

/// From the mod's `entity_tick_post`, for a small wild creature (a butterfly, a moth, a firefly, a worm): takes it out
/// of the world far from every player, unless `keep` (settled, say) or it has a name. Whether it was taken out.
pub fn despawn_far(host: &mut dyn ServerHost, dimension: &str, entity: u32, keep: bool) -> bool {
    if keep {
        return false;
    }
    let Some(e) = host.entity(entity) else {
        return false;
    };
    let Some(at) = e.position(host) else {
        return false;
    };
    let nearest = nearest_player(&*host, dimension, at);
    let gone = nearest > GONE || (nearest > NEAR && Random::new(entity).next().is_multiple_of(CHANCE));
    if !gone {
        return false;
    }
    // A name tag keeps it, as in vanilla.
    let named = e
        .nbt(host)
        .is_some_and(|nbt| matches!(nbt.get(&"CustomName".into()), Some(t) if !matches!(t, Tag::End)));
    !named && e.discard(host)
}
