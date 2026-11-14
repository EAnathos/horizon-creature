//! The fairy's own goal (SPECS §2, §3), above the library's wild flight (`flutter`), which it takes over while it has
//! something to do:
//!
//! - **Following**: befriended and not told to stay, it circles its friend's head.
//! - **Staying**: told to stay, it hovers where it was.
//! - **Drawn**: wild, a sneaking player holding glow berries (or sweet berries) within `DRAWN` blocks draws it: it
//!   hovers before their face, then nibbles one (`NIBBLES`, taken by the mod's tick, which has the host); one chance in
//!   3 (6 for a sweet berry) it befriends them.
//! - **Fleeing**: wild, a player who comes within `SHY` blocks without sneaking, or runs within `RUNNER`, scares it off.
//!
//! The player is the one the goal sees (`cx.w.player`): single player, as the rest of the series.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

/// Its friend further than this (blocks): it comes to them at once (the mod's tick teleports it).
pub const TELEPORT: f64 = 24.0;
/// How it circles its friend's head: how far out, how far above their eyes, how fast around (degrees a tick).
const ORBIT: f64 = 0.9;
const ORBIT_ABOVE: f64 = 0.35;
const ORBIT_SPEED: f64 = 3.0;
/// The fastest it flies after its friend, and how hard it steers (a share of the way left, each tick).
const FOLLOW_SPEED: f64 = 0.7;
const FOLLOW_PULL: f64 = 0.18;
/// Drawn by berries within this many blocks (SPECS §3); it hovers this far before the player's eyes, `NIBBLE_AFTER`
/// ticks before it nibbles, then stays there and waits `NIBBLE_PAUSE` ticks before the next one.
const DRAWN: f64 = 4.0;
const BEFORE: f64 = 0.7;
const NIBBLE_AFTER: i32 = 40;
const NIBBLE_PAUSE: i32 = 100;
/// One chance in this many that a glow berry befriends; a sweet berry is half as good.
const GLOW_BERRY_CHANCE: i32 = 3;
const SWEET_BERRY_CHANCE: i32 = 6;
/// Shy (SPECS §2): a player this close without sneaking, or running this close, scares it off for `FLEE_TICKS`.
const SHY: f64 = 4.0;
const RUNNER: f64 = 8.0;
const FLEE_TICKS: i32 = 50;
const FLEE_SPEED: f64 = 0.2;
const FLEE_FAR: f64 = 7.0;

const GLOW_BERRIES: &str = "minecraft:glow_berries";
const SWEET_BERRIES: &str = "minecraft:sweet_berries";

/// A berry nibbled from a player's hand: whose, which, and whether it befriended them.
#[derive(Clone, Copy, Debug)]
pub struct Nibble {
    pub player: [i32; 4],
    pub glow: bool,
    pub befriends: bool,
}

/// Berries nibbled, and the players a fairy fled from (for the bestiary), for the mod's tick.
static NIBBLES: Mutex<BTreeMap<u32, Vec<Nibble>>> = Mutex::new(BTreeMap::new());
static FLED: Mutex<BTreeMap<u32, Vec<[i32; 4]>>> = Mutex::new(BTreeMap::new());
/// The fairies this goal moves now, for the drawing (their wings beat).
static ACTIVE: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

pub fn take_nibbles(fairy: u32) -> Vec<Nibble> {
    NIBBLES.lock().unwrap().remove(&fairy).unwrap_or_default()
}

pub fn take_fled(fairy: u32) -> Vec<[i32; 4]> {
    FLED.lock().unwrap().remove(&fairy).unwrap_or_default()
}

/// Whether this goal moves it now (it is flying).
pub fn active(fairy: u32) -> bool {
    ACTIVE.lock().unwrap().contains(&fairy)
}

pub fn forget(fairy: u32) {
    NIBBLES.lock().unwrap().remove(&fairy);
    FLED.lock().unwrap().remove(&fairy);
    ACTIVE.lock().unwrap().remove(&fairy);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Follow,
    Stay([f64; 3]),
    Drawn,
    Flee([f64; 3]),
}

#[derive(Clone, Debug, Default)]
pub struct Fairy {
    mode: Option<Mode>,
    /// Ticks in the current mode.
    ticks: i32,
    /// Ticks before it may nibble again.
    pause: i32,
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

fn here(m: &Mob) -> [f64; 3] {
    [m.position.x, m.position.y, m.position.z]
}

impl Fairy {
    /// What it should do now, if anything (else the wild flight goes on).
    fn choose(&self, m: &mut Mob, cx: &Cx) -> Option<Mode> {
        let player = cx.w.player.as_ref().filter(|p| p.alive && !p.spectator);
        if let Some(friend) = creature::friend::of(m.id) {
            if friend.staying {
                return Some(match self.mode {
                    Some(Mode::Stay(at)) => Mode::Stay(at),
                    _ => Mode::Stay(here(m)),
                });
            }
            return player.filter(|p| p.uuid == friend.uuid).map(|_| Mode::Follow);
        }
        if let Some(Mode::Flee(to)) = self.mode
            && self.ticks < FLEE_TICKS
        {
            return Some(Mode::Flee(to));
        }
        let p = player?;
        let d = distance(p.eye, here(m));
        let berries = |hand: &Option<std::sync::Arc<str>>| {
            hand.as_deref().is_some_and(|i| i == GLOW_BERRIES || i == SWEET_BERRIES)
        };
        let holds = berries(&p.main_hand) || berries(&p.off_hand);
        if p.sneaking && holds && d < DRAWN {
            return Some(Mode::Drawn);
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

    fn start_mode(&mut self, m: &Mob, cx: &Cx, mode: Mode) {
        if self.mode != Some(mode) {
            self.ticks = 0;
            if let (Mode::Flee(_), Some(p)) = (mode, cx.w.player.as_ref()) {
                FLED.lock().unwrap().entry(m.id).or_default().push(p.uuid);
            }
        }
        self.mode = Some(mode);
    }
}

/// Flies straight at `target`, quickly (following its friend): a share of the way each tick, at most `FOLLOW_SPEED`.
fn pull(m: &mut Mob, target: [f64; 3]) {
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
fn face(m: &mut Mob, at: [f64; 3]) {
    let yaw = (-(at[0] - m.position.x)).atan2(at[2] - m.position.z).to_degrees() as f32;
    m.yaw = yaw;
    m.body_yaw = yaw;
    m.head_yaw = yaw;
}

impl Goal for Fairy {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        self.pause -= 1;
        match self.choose(m, cx) {
            Some(mode) => {
                self.start_mode(m, cx, mode);
                true
            }
            None => false,
        }
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        self.can_use(m, cx)
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        ACTIVE.lock().unwrap().insert(m.id);
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.mode = None;
        ACTIVE.lock().unwrap().remove(&m.id);
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        m.no_gravity = true;
        m.fall_distance = 0.0;
        self.ticks += 1;
        let Some(mode) = self.mode else {
            return;
        };
        match mode {
            Mode::Follow => {
                let Some(p) = cx.w.player.as_ref() else {
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
                let Some(p) = cx.w.player.as_ref() else {
                    return;
                };
                let before = [
                    p.eye[0] + p.look[0] * BEFORE,
                    p.eye[1] + p.look[1] * BEFORE - 0.3,
                    p.eye[2] + p.look[2] * BEFORE,
                ];
                creature::flutter::steer(m, before, 0.12);
                face(m, p.eye);
                if self.ticks >= NIBBLE_AFTER && self.pause <= 0 && distance(here(m), before) < 0.8 {
                    let glow = p.main_hand.as_deref() == Some(GLOW_BERRIES)
                        || p.main_hand.as_deref() != Some(SWEET_BERRIES) && p.off_hand.as_deref() == Some(GLOW_BERRIES);
                    let chance = if glow { GLOW_BERRY_CHANCE } else { SWEET_BERRY_CHANCE };
                    let befriends = m.random.next_int_bound(chance).unwrap_or(1) == 0;
                    NIBBLES.lock().unwrap().entry(m.id).or_default().push(Nibble {
                        player: p.uuid,
                        glow,
                        befriends,
                    });
                    self.pause = NIBBLE_PAUSE;
                    self.ticks = 0;
                }
            }
            Mode::Flee(to) => {
                creature::flutter::steer(m, to, FLEE_SPEED);
            }
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
