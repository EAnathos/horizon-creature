//! The fairy's own behaviour (SPECS §2, §3), above the library's wild flight (`flutter`), which it takes over while it
//! has something to do (both run from the server tick: the library's `body`):
//!
//! - **Following**: befriended and not told to stay, it circles its friend's head.
//! - **Staying**: told to stay, it hovers where it was.
//! - **Drawn**: wild, a player holding glow berries (or sweet berries) within `DRAWN` blocks draws it: it hovers before
//!   their face, to be fed (a right-click, the mod's `entity_interact`).
//! - **Fleeing**: wild, a player who comes within `SHY` blocks without sneaking (and without berries), or runs within
//!   `RUNNER`, scares it off.
//!
//! The player is the nearest one (`World::player`).
use std::collections::BTreeSet;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, World};

/// Its friend further than this (blocks): it comes to them at once (the mod's tick teleports it).
pub const TELEPORT: f64 = 24.0;
/// How it circles its friend's head: how far out, how far above their eyes, how fast around (degrees a tick).
const ORBIT: f64 = 0.9;
const ORBIT_ABOVE: f64 = 0.35;
const ORBIT_SPEED: f64 = 3.0;
/// The fastest it flies after its friend, and how hard it steers (a share of the way left, each tick).
const FOLLOW_SPEED: f64 = 0.7;
const FOLLOW_PULL: f64 = 0.18;
/// Drawn by berries within this many blocks (SPECS §3); it hovers this far before the player's eyes.
const DRAWN: f64 = 6.0;
const BEFORE: f64 = 0.9;
/// Shy (SPECS §2): a player this close without sneaking, or running this close, scares it off for `FLEE_TICKS`.
const SHY: f64 = 4.0;
const RUNNER: f64 = 8.0;
const FLEE_TICKS: i32 = 50;
const FLEE_SPEED: f64 = 0.2;
const FLEE_FAR: f64 = 7.0;

/// What draws it, and what it eats.
pub const GLOW_BERRIES: &str = "minecraft:glow_berries";
pub const SWEET_BERRIES: &str = "minecraft:sweet_berries";

/// The fairies this behaviour moves now, for the drawing (their wings beat).
static ACTIVE: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// Whether this behaviour moves it now (it is flying).
pub fn active(fairy: u32) -> bool {
    ACTIVE.lock().unwrap().contains(&fairy)
}

pub fn forget(fairy: u32) {
    ACTIVE.lock().unwrap().remove(&fairy);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Follow,
    Stay([f64; 3]),
    Drawn,
    Flee([f64; 3]),
}

#[derive(Debug, Default)]
pub struct Fairy {
    mode: Option<Mode>,
    /// Ticks in the current mode.
    ticks: i32,
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

fn here(m: &Body) -> [f64; 3] {
    [m.position.x, m.position.y, m.position.z]
}

impl Fairy {
    /// What it should do now, if anything (else the wild flight goes on).
    fn choose(&self, m: &mut Body, cx: &World<'_>) -> Option<Mode> {
        let player = cx.player.as_ref().filter(|p| p.alive && !p.spectator);
        if let Some(friend) = creature::friend::of(m.id) {
            if friend.staying {
                return Some(match self.mode {
                    Some(Mode::Stay(at)) => Mode::Stay(at),
                    _ => Mode::Stay(here(m)),
                });
            }
            return player.filter(|p| friend.is(p.uuid)).map(|_| Mode::Follow);
        }
        let p = player?;
        let d = distance(p.eye, here(m));
        let berries = |hand: &Option<String>| hand.as_deref().is_some_and(|i| i == GLOW_BERRIES || i == SWEET_BERRIES);
        if (berries(&p.main_hand) || berries(&p.off_hand)) && d < DRAWN && !p.sprinting {
            return Some(Mode::Drawn);
        }
        if let Some(Mode::Flee(to)) = self.mode
            && self.ticks < FLEE_TICKS
        {
            return Some(Mode::Flee(to));
        }
        if (!p.sneaking && d < SHY) || (p.sprinting && d < RUNNER) {
            // Away from them, a few blocks off and up.
            let away = [m.position.x - p.position.x, m.position.z - p.position.z];
            let len = away[0].hypot(away[1]).max(0.1);
            return Some(Mode::Flee([
                m.position.x + away[0] / len * FLEE_FAR,
                p.position.y + 2.5,
                m.position.z + away[1] / len * FLEE_FAR,
            ]));
        }
        None
    }
}

/// Flies straight at `target`, quickly (following its friend): a share of the way each tick, at most `FOLLOW_SPEED`.
fn pull(m: &mut Body, target: [f64; 3]) {
    let d = [
        target[0] - m.position.x,
        target[1] - m.position.y,
        target[2] - m.position.z,
    ];
    let mut v = d.map(|x| x * FOLLOW_PULL);
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > FOLLOW_SPEED {
        v = v.map(|x| x * FOLLOW_SPEED / len);
    }
    m.velocity.x = v[0];
    m.velocity.y = v[1];
    m.velocity.z = v[2];
}

/// Turns to face `at`.
fn face(m: &mut Body, at: [f64; 3]) {
    let yaw = (-(at[0] - m.position.x)).atan2(at[2] - m.position.z).to_degrees() as f32;
    m.yaw = yaw;
    m.body_yaw = yaw;
    m.head_yaw = yaw;
}

impl Behaviour for Fairy {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        match self.choose(m, cx) {
            Some(mode) => {
                if self.mode != Some(mode) {
                    self.ticks = 0;
                }
                self.mode = Some(mode);
                ACTIVE.lock().unwrap().insert(m.id);
                true
            }
            None => false,
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.mode = None;
        ACTIVE.lock().unwrap().remove(&m.id);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = true;
        m.fall_distance = 0.0;
        self.ticks += 1;
        let Some(mode) = self.mode else {
            return;
        };
        match mode {
            Mode::Follow => {
                let Some(p) = cx.player.as_ref() else {
                    return;
                };
                let angle = (f64::from(m.age) * ORBIT_SPEED).to_radians();
                let target = [
                    p.eye[0] + angle.cos() * ORBIT,
                    p.eye[1] + ORBIT_ABOVE + 0.1 * (angle * 2.0).sin(),
                    p.eye[2] + angle.sin() * ORBIT,
                ];
                pull(m, target);
                face(m, p.eye);
            }
            Mode::Stay(at) => {
                let bob = 0.15 * (f64::from(m.age) * 0.08).sin();
                creature::flutter::steer(m, [at[0], at[1] + bob, at[2]], 0.05);
            }
            Mode::Drawn => {
                let Some(p) = cx.player.as_ref() else {
                    return;
                };
                let before = [
                    p.eye[0] + p.look[0] * BEFORE,
                    p.eye[1] + p.look[1] * BEFORE - 0.4,
                    p.eye[2] + p.look[2] * BEFORE,
                ];
                creature::flutter::steer(m, before, 0.15);
                face(m, p.eye);
            }
            Mode::Flee(to) => {
                creature::flutter::steer(m, to, FLEE_SPEED);
            }
        }
    }
}
