//! The talons (SPECS §6.4): ridden and in flight, the "talons" key (R, rebindable: `key.vulture.talons` in
//! `horizon-mods.txt`) catches the creature just below, and lets it go on the next press.
//!
//! - The key is heard on the client (`key` hook) and taken by the ridden vulture's `Fly` goal within half a second
//!   (`take_press`), which picks the creature: a small mob (at most 1 × 1 block: chicken, rabbit, pig, sheep, fox, cat,
//!   wolf, babies...) or a tamed animal, at most 3 blocks below, never itself.
//! - While it carries one, the server tick hook (`carry`) keeps the creature under its talons; its flight is heavier
//!   (`flight::Controls::heavy`).
//! - It lets go on the key, when its rider gets off (`Fly::stop`) or when it dies.
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use horizon_client::mobs::MobWorld;
use horizon_client::mod_api::{Host, Mob};

pub const KEY: &str = "talons";
pub const DEFAULT_KEY: &str = "key.keyboard.r";
/// How far below its feet it reaches, and how far around, in blocks.
const REACH_DOWN: f64 = 3.0;
const REACH_AROUND: f64 = 1.5;
/// The biggest creature it lifts, in blocks (width and height).
const MAX_SIZE: f64 = 1.0;
/// How far below the vulture's feet the carried creature hangs.
const HANG: f64 = 1.0;

/// When the key was last pressed, in milliseconds since the epoch.
static PRESSED: AtomicU64 = AtomicU64::new(0);
/// Each vulture's catch, and where to hold it.
static CARRIED: Mutex<BTreeMap<u32, (u32, [f64; 3])>> = Mutex::new(BTreeMap::new());

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// From the `key` hook: the talons key went down.
pub fn press() {
    PRESSED.store(now(), Ordering::Relaxed);
}

/// A press in the last half second, taken once.
pub fn take_press() -> bool {
    let at = PRESSED.swap(0, Ordering::Relaxed);
    at != 0 && now().saturating_sub(at) < 500
}

pub fn carrying(id: u32) -> bool {
    CARRIED.lock().unwrap().contains_key(&id)
}

pub fn release(id: u32) {
    CARRIED.lock().unwrap().remove(&id);
}

/// From the ridden goal: catches the nearest creature below it that it can lift, or lets go of the one it holds.
pub fn toggle(m: &Mob, w: &MobWorld) {
    if carrying(m.id) {
        release(m.id);
        return;
    }
    if m.on_ground {
        return;
    }
    let (x, y, z) = (m.position.x, m.position.y, m.position.z);
    let catch = w
        .others
        .iter()
        .filter(|s| s.alive && s.id != m.id)
        .filter(|s| {
            let (lo, hi) = (s.bounds.min_position(), s.bounds.max_position());
            let small =
                hi.x - lo.x <= MAX_SIZE + 0.01 && hi.y - lo.y <= MAX_SIZE + 0.01 && hi.z - lo.z <= MAX_SIZE + 0.01;
            let below = s.position.y <= y && s.position.y >= y - REACH_DOWN;
            let close = (s.position.x - x).abs() <= REACH_AROUND && (s.position.z - z).abs() <= REACH_AROUND;
            (small || s.tame) && below && close
        })
        .min_by(|a, b| (y - a.position.y).total_cmp(&(y - b.position.y)));
    if let Some(s) = catch {
        CARRIED.lock().unwrap().insert(m.id, (s.id, [x, y - HANG, z]));
    }
}

/// From the goal, every tick it carries: where to hold the catch.
pub fn follow(m: &Mob) {
    if let Some(entry) = CARRIED.lock().unwrap().get_mut(&m.id) {
        entry.1 = [m.position.x, m.position.y - HANG, m.position.z];
    }
}

/// From `entity_tick_post`: keeps the catch under the vulture's talons; lets go of one that is gone.
pub fn carry(host: &mut dyn Host, dimension: &str, id: u32) {
    let Some((caught, at)) = CARRIED.lock().unwrap().get(&id).copied() else {
        return;
    };
    if !host.teleport_mob(dimension, caught, at) {
        release(id);
    }
}
