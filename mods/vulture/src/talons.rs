//! The talons (SPECS §6.4): ridden and in flight, the "talons" key (R, rebindable in Controls: a registered key
//! mapping) catches the creature just below, and lets it go on the next press.
//!
//! - The key is heard on the client (`key_mapping_pre`), sent to the server (the `vulture:talons` payload) and taken by
//!   the ridden vulture's `Fly` within half a second (`take_press`), which picks the creature: a small mob (at most 1 × 1 block: chicken, rabbit, pig, sheep, fox, cat,
//!   wolf, babies...) or a tamed animal, at most 4 blocks below and 2.5 around, the nearest, never itself. The legs
//!   thrust down on every try, caught or missed (`state::grabbed`, drawn by `pose`).
//! - While it carries one, the server tick hook (`carry`) keeps the creature under its talons.
//! - It lets go on the key, when its rider gets off (`Fly::stop`) or when it dies.
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use creature::body::{Body, World};
use mod_api::ServerHost;

/// The key mapping's id and its default key.
pub const KEY: &str = "key.vulture.talons";
pub const DEFAULT_KEY: &str = "key.keyboard.r";
/// How far below its feet it reaches, and how far around, in blocks (a circle; 1.5 in a square was too tight to aim).
const REACH_DOWN: f64 = 4.0;
const REACH_AROUND: f64 = 2.5;
/// The biggest creature it lifts, in blocks (width and height).
const MAX_SIZE: f64 = 1.0;
/// How far below the vulture's position (its feet) the carried creature's feet are held: its back against the
/// talons.
const HANG: f64 = 0.7;

/// When each rider last pressed the key, in milliseconds since the epoch: a press moves only the sender's own vulture.
static PRESSED: Mutex<BTreeMap<u128, u64>> = Mutex::new(BTreeMap::new());
/// Each vulture's catch, and where to hold it.
static CARRIED: Mutex<BTreeMap<u32, (u32, [f64; 3])>> = Mutex::new(BTreeMap::new());

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// From the server's `vulture:talons` payload: player `player`, riding a vulture, pressed the talons key.
pub fn press(player: u128) {
    PRESSED.lock().unwrap().insert(player, now());
}

/// A press by `player` in the last half second, taken once.
pub fn take_press(player: u128) -> bool {
    PRESSED
        .lock()
        .unwrap()
        .remove(&player)
        .is_some_and(|at| now().saturating_sub(at) < 500)
}

/// From `player_logged_out`.
pub fn left(player: u128) {
    PRESSED.lock().unwrap().remove(&player);
}

pub fn carrying(id: u32) -> bool {
    CARRIED.lock().unwrap().contains_key(&id)
}

pub fn release(id: u32) {
    CARRIED.lock().unwrap().remove(&id);
}

/// From the ridden flight: catches the nearest creature below it that it can lift, or lets go of the one it holds.
pub fn toggle(m: &Body, cx: &World<'_>) {
    if carrying(m.id) {
        release(m.id);
        return;
    }
    if m.on_ground {
        return;
    }
    // The talons snap down whether they catch anything or not.
    crate::state::grabbed(m);
    let (x, y, z) = (m.position.x, m.position.y, m.position.z);
    let host = cx.host();
    let catch = cx
        .ground
        .level
        .entities_in(
            host,
            [x - REACH_AROUND, y - REACH_DOWN, z - REACH_AROUND],
            [x + REACH_AROUND, y, z + REACH_AROUND],
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.id() != m.id)
        .filter_map(|e| {
            let living = host.living(e)?;
            if !living.alive(host).unwrap_or(false) {
                return None;
            }
            let at = e.position(host)?;
            let [w, h] = e.dimensions(host)?;
            let small = w <= MAX_SIZE + 0.01 && h <= MAX_SIZE + 0.01;
            let tame = living.owner(host).is_some();
            let close = (at[0] - x).hypot(at[2] - z) <= REACH_AROUND;
            ((small || tame) && close && at[1] <= y).then_some((e.id(), at))
        })
        .min_by(|a, b| {
            let d = |p: [f64; 3]| (p[0] - x).powi(2) + (p[1] - y).powi(2) + (p[2] - z).powi(2);
            d(a.1).total_cmp(&d(b.1))
        });
    if let Some((id, _)) = catch {
        CARRIED.lock().unwrap().insert(m.id, (id, [x, y - HANG, z]));
    }
}

/// From the ridden flight, every tick it carries: where to hold the catch, one tick ahead along the vulture's velocity
/// (the catch moves after the vulture has moved), so it stays in the talons instead of trailing behind.
pub fn follow(m: &Body) {
    if let Some(entry) = CARRIED.lock().unwrap().get_mut(&m.id) {
        let v = m.velocity;
        entry.1 = [m.position.x + v.x, m.position.y + v.y - HANG, m.position.z + v.z];
    }
}

/// From `entity_tick_post`: keeps the catch under the vulture's talons; lets go of one that is gone.
pub fn carry(host: &mut dyn ServerHost, id: u32) {
    let Some((caught, at)) = CARRIED.lock().unwrap().get(&id).copied() else {
        return;
    };
    let held = host.entity(caught).is_some_and(|e| {
        e.reset_fall_distance(host);
        e.set_velocity(host, [0.0; 3]);
        e.teleport_to(host, at, None)
    });
    if !held {
        release(id);
    }
}
