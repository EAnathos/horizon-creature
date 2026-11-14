//! The cyclops's eye, its weak spot (SPECS §2): an arrow or a trident that strikes it deals double damage and stuns
//! the cyclops for `STUN` ticks: it stands dazed, stars about its head, its head lolling (`Stunned`, `tick`).
//!
//! The damage hook has no host to ask where the projectile was, so each tick the arrows and tridents near a cyclops
//! are noted with their velocity (`watch`), and on the hit their path is followed to the cyclops's face: the eye is
//! struck if it crosses the face within the eye's height and width.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::time::Instant;

use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Host, LivingIncomingDamageEvent, Mob, ParticleOptions, ParticleType};

const PROJECTILES: [&str; 3] = ["minecraft:arrow", "minecraft:spectral_arrow", "minecraft:trident"];
const DAMAGE_TYPES: [&str; 2] = ["minecraft:arrow", "minecraft:trident"];
/// The eye, from the cyclops's feet (blocks): its height band, half its width, how far its face is in front.
const EYE_LOW: f64 = 3.15;
const EYE_HIGH: f64 = 3.7;
const EYE_SIDE: f64 = 0.4;
const FACE: f64 = 0.4;
/// How long a struck cyclops stays dazed (ticks), and how near a projectile must be to be watched (blocks).
const STUN: i32 = 40;
const WATCH: f64 = 10.0;

/// A projectile's position and velocity.
type Motion = ([f64; 3], [f64; 3]);
/// The arrows and tridents near a cyclops, last tick: where and how fast.
static FLYING: Mutex<BTreeMap<u32, Motion>> = Mutex::new(BTreeMap::new());
/// Each cyclops's feet and facing (degrees), from its tick.
static FACING: Mutex<BTreeMap<u32, ([f64; 3], f32)>> = Mutex::new(BTreeMap::new());
/// Struck in the eye: the stun to start (its goal), the cry to play (its tick), when (the drawing).
static STRUCK: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
static CRY: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
static DAZED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());

/// From `level_tick_post`: notes the arrows and tridents near a cyclops.
pub fn watch(host: &mut dyn Host, dimension: &str) {
    let cyclopes: Vec<[f64; 3]> = FACING.lock().unwrap().values().map(|(at, _)| *at).collect();
    if cyclopes.is_empty() {
        FLYING.lock().unwrap().clear();
        return;
    }
    let mut flying = BTreeMap::new();
    for (id, kind, at) in host.level_entities(dimension) {
        if !PROJECTILES.contains(&kind.as_str()) {
            continue;
        }
        let near = cyclopes
            .iter()
            .any(|c| (0..3).map(|k| (c[k] - at[k]).powi(2)).sum::<f64>() < WATCH * WATCH);
        if let (true, Some(v)) = (near, host.entity(id).and_then(|e| host.entity_velocity(e))) {
            flying.insert(id, (at, v));
        }
    }
    *FLYING.lock().unwrap() = flying;
}

/// From the cyclops's tick: where it stands and faces, and its stars and cry while dazed.
pub fn tick(host: &mut dyn Host, entity: u32, at: [f64; 3]) {
    if let Some(r) = host.entity(entity).and_then(|e| host.entity_rotation(e)) {
        FACING.lock().unwrap().insert(entity, (at, r[0]));
    }
    if CRY.lock().unwrap().remove(&entity) {
        host.play_sound(
            "cyclops:entity.cyclops.stunned",
            at,
            creature::sounds::HOSTILE,
            1.5,
            1.0,
        );
    }
    if dazed(entity).is_some() {
        let stars = ParticleOptions::simple(ParticleType::Crit);
        host.send_particles(&stars, None, [at[0], at[1] + 4.2, at[2]], [0.5, 0.15, 0.5], 0.05, 2);
    }
}

/// From the mod's `living_incoming_damage`: an arrow or trident that struck the eye deals double and stuns.
pub fn struck(event: &mut LivingIncomingDamageEvent<'_>) {
    if event.kind != crate::CYCLOPS || !DAMAGE_TYPES.contains(&event.damage_type) {
        return;
    }
    let Ok(cyclops) = u32::try_from(event.entity) else {
        return;
    };
    let Some((c, yaw)) = FACING.lock().unwrap().get(&cyclops).copied() else {
        return;
    };
    // The projectile: by its id, or, as Horizon doesn't name an arrow's yet (0), the nearest one noted.
    let flying = FLYING.lock().unwrap();
    let projectile = match u32::try_from(event.direct) {
        Ok(id) if id > 0 => flying.get(&id).copied(),
        _ => flying
            .values()
            .min_by(|a, b| distance(a.0, c).total_cmp(&distance(b.0, c)))
            .copied(),
    };
    drop(flying);
    let Some((p, v)) = projectile else {
        return;
    };
    let yaw = f64::from(yaw).to_radians();
    let (forward, side) = ([-yaw.sin(), yaw.cos()], [yaw.cos(), yaw.sin()]);
    let along = |d: [f64; 3], axis: [f64; 2]| d[0] * axis[0] + d[2] * axis[1];
    // Where its path crosses the face: it must be coming at the cyclops from the front.
    let rel = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let closing = along(v, forward);
    if closing >= 0.0 || along(rel, forward) < FACE {
        return;
    }
    let t = (FACE - along(rel, forward)) / closing;
    let hit = [rel[0] + v[0] * t, rel[1] + v[1] * t, rel[2] + v[2] * t];
    if (EYE_LOW..=EYE_HIGH).contains(&hit[1]) && along(hit, side).abs() <= EYE_SIDE {
        event.amount *= 2.0;
        STRUCK.lock().unwrap().insert(cyclops);
        CRY.lock().unwrap().insert(cyclops);
        DAZED.lock().unwrap().insert(cyclops, Instant::now());
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// Seconds since cyclops `entity` was struck in the eye, while it is dazed.
pub fn dazed(entity: u32) -> Option<f32> {
    DAZED
        .lock()
        .unwrap()
        .get(&entity)
        .map(|at| at.elapsed().as_secs_f32())
        .filter(|s| *s < STUN as f32 / 20.0)
}

pub fn forget(entity: u32) {
    FACING.lock().unwrap().remove(&entity);
    STRUCK.lock().unwrap().remove(&entity);
    CRY.lock().unwrap().remove(&entity);
    DAZED.lock().unwrap().remove(&entity);
}

/// Dazed: it stands still and does nothing, whatever it was doing.
#[derive(Clone, Debug, Default)]
pub struct Stunned {
    ticks: i32,
}

impl Goal for Stunned {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        if STRUCK.lock().unwrap().remove(&m.id) {
            self.ticks = STUN;
            return true;
        }
        false
    }
    fn can_continue(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        self.ticks > 0
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        self.ticks -= 1;
        m.ai.nav.stop();
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
