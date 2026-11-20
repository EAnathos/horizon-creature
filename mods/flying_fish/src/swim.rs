//! How a flying fish swims and glides (SPECS §3), by hand (the library's `body`). In water it swims as a shoal in the
//! top blocks, turning with its neighbours. Frightened (a player near, a hit, a dolphin or a boat the mod sees, a
//! neighbour taking off), and now and then for nothing, it races up, bursts out of the water and glides a block or two
//! over the waves, beats its tail on a wave to glide again, and falls back in. Out of water and not gliding it is
//! beached: the library's `aquatic` makes it fall and flop.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World};

/// Its speed swimming and gliding (blocks a tick), and how fast it climbs out.
const SWIM_SPEED: f64 = 0.1;
const GLIDE_SPEED: f64 = 0.8;
const RISE_SPEED: (f64, f64) = (0.5, 0.45);
/// How deep under the surface it swims (blocks of water over it).
const DEPTH: (f64, f64) = (0.8, 2.5);
/// A glide's length (ticks, so 10 to 20 blocks), how high over the water, the tail-beats it may add, a dip's length.
const GLIDE_TICKS: (i32, i32) = (13, 25);
const GLIDE_HEIGHT: (f64, f64) = (1.0, 2.0);
const BEATS: i32 = 2;
const DIP_TICKS: i32 = 5;
/// The longest it tries to reach the surface (ticks), and its rest after a glide before it glides again.
const RISE_TICKS: i32 = 30;
const REST: i32 = 100;
/// How near a player frightens it, how far a fright spreads through the shoal (blocks), and one glide for nothing in
/// so many ticks.
const FRIGHT: f64 = 6.0;
const SPREAD: f64 = 8.0;
const FOR_NOTHING: i32 = 3000;
/// Its shoal: the neighbours it turns with, the gap it keeps from them (blocks).
const SHOAL: f64 = 5.0;
const GAP: f64 = 0.7;
/// A glide of more than this many blocks is a fact; a takeoff of this many is a shoal's.
pub(crate) const LONG_GLIDE: f64 = 30.0;
const SHOAL_TAKEOFF: usize = 4;

/// Its sounds: the splash out of and into the water (`crate`).
use crate::SPLASH;

/// A fish's place and heading.
type Place = ([f64; 3], [f64; 2]);
/// Each fish's place and heading, for its shoal (single player: the hooks share the ids).
static FISH: Mutex<BTreeMap<u32, Place>> = Mutex::new(BTreeMap::new());
/// Fish told to take off: in so many ticks, away along this heading.
static PANIC: Mutex<BTreeMap<u32, (i32, [f64; 2])>> = Mutex::new(BTreeMap::new());
/// The fish out of the water gliding, for their drawing (fins spread).
static GLIDING: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
/// What the mod's tick turns into facts: glides longer than `LONG_GLIDE` (by the fish) and shoals taking off (where).
static GLIDED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static TOOK_OFF: Mutex<Vec<[f64; 3]>> = Mutex::new(Vec::new());

/// Whether flying fish `id` is gliding (its fins spread).
pub(crate) fn gliding(id: u32) -> bool {
    GLIDING.lock().unwrap().contains(&id)
}

/// Whether flying fish `id` just ended a long glide.
pub(crate) fn glided(id: u32) -> bool {
    let mut all = GLIDED.lock().unwrap();
    let was = all.contains(&id);
    all.retain(|g| *g != id);
    was
}

/// Where shoals took off since the last call.
pub(crate) fn took_off() -> Vec<[f64; 3]> {
    std::mem::take(&mut *TOOK_OFF.lock().unwrap())
}

pub(crate) fn forget(id: u32) {
    FISH.lock().unwrap().remove(&id);
    PANIC.lock().unwrap().remove(&id);
    GLIDING.lock().unwrap().remove(&id);
    GLIDED.lock().unwrap().retain(|g| *g != id);
}

/// A threat at `from` frightens the fish at `at` and its shoal: each takes off within half a second, away from it.
pub(crate) fn scare(at: [f64; 3], from: [f64; 3]) {
    let fish = FISH.lock().unwrap();
    let mut panic = PANIC.lock().unwrap();
    let mut count = 0;
    for (id, (p, _)) in fish.iter() {
        if creature::body::distance(*p, at) > SPREAD || panic.contains_key(id) {
            continue;
        }
        let away = flat([p[0] - from[0], p[2] - from[2]]).unwrap_or([0.0, 1.0]);
        // Fanning out: the nearest first, each turned a little.
        let delay = (creature::body::distance(*p, at) * 1.2) as i32 + (*id % 4) as i32;
        let turn = (f64::from(*id % 7) - 3.0) * 0.12;
        panic.insert(*id, (delay, turned(away, turn)));
        count += 1;
    }
    if count >= SHOAL_TAKEOFF {
        TOOK_OFF.lock().unwrap().push(at);
    }
}

/// `v` made a unit vector, if it has a length.
fn flat(v: [f64; 2]) -> Option<[f64; 2]> {
    let n = v[0].hypot(v[1]);
    (n > 1e-6).then(|| [v[0] / n, v[1] / n])
}

/// `v` turned by `angle` radians.
fn turned(v: [f64; 2], angle: f64) -> [f64; 2] {
    let (s, c) = angle.sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c]
}

fn cell(p: Vec3) -> [i32; 3] {
    [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32]
}

fn water(cx: &World<'_>, p: [i32; 3]) -> bool {
    cx.block(p) == "minecraft:water"
}

/// The water over it (blocks), up to 8.
fn depth(m: &Body, cx: &World<'_>) -> f64 {
    let [x, y, z] = cell(Vec3 {
        y: m.position.y + 0.2,
        ..m.position
    });
    let water = (0..8).take_while(|&d| water(cx, [x, y + d, z])).count();
    water as f64 - (m.position.y + 0.2).fract()
}

/// The top of the water under `at` (its y), if there is water within a few blocks down before anything else.
fn surface_under(cx: &World<'_>, at: Vec3) -> Option<f64> {
    let [x, y, z] = cell(at);
    (0..6).map(|d| y - d).find_map(|y| match cx.block([x, y, z]).as_str() {
        "minecraft:air" | "minecraft:cave_air" => None,
        "minecraft:water" => Some(Some(f64::from(y) + 0.9)),
        _ => Some(None),
    })?
}

/// Its heading as a yaw (degrees), as the game turns a mob.
fn yaw(dir: [f64; 2]) -> f32 {
    (-dir[0]).atan2(dir[1]).to_degrees() as f32
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Swim,
    /// Racing up to the surface, for so many more ticks.
    Rise(i32),
    /// Over the water: ticks left of this stretch, tail-beats left.
    Glide(i32, i32),
    /// Its tail on a wave between two stretches.
    Dip(i32, i32),
}

#[derive(Debug)]
pub(crate) struct Fish {
    phase: Phase,
    dir: [f64; 2],
    /// Ticks to its next wander, and of rest left after a glide.
    wander: i32,
    rest: i32,
    /// Its health last tick (a hit frightens it), where its glide began, how high it glides, which way it turns at a
    /// shore.
    health: f32,
    start: [f64; 3],
    height: f64,
    turn: f64,
}

impl Default for Fish {
    fn default() -> Fish {
        Fish {
            phase: Phase::Swim,
            dir: [0.0, 1.0],
            wander: 0,
            rest: 0,
            health: -1.0,
            start: [0.0; 3],
            height: 1.5,
            turn: 0.35,
        }
    }
}

impl Fish {
    /// Its shoal's pull on its heading: their heading, toward them when it strays, away when too near.
    fn shoal(&self, m: &Body) -> [f64; 2] {
        let at = m.position.array();
        let fish = FISH.lock().unwrap();
        let (mut heading, mut toward, mut n) = ([0.0, 0.0], [0.0, 0.0], 0.0);
        for (id, (p, d)) in fish.iter() {
            let dist = creature::body::distance(*p, at);
            if *id == m.id || dist > SHOAL {
                continue;
            }
            heading = [heading[0] + d[0], heading[1] + d[1]];
            let push = if dist < GAP { -1.0 } else { 0.15 };
            toward = [toward[0] + (p[0] - at[0]) * push, toward[1] + (p[2] - at[2]) * push];
            n += 1.0;
        }
        if n == 0.0 {
            return self.dir;
        }
        let blend = [
            self.dir[0] + heading[0] / n * 0.08 + toward[0] / n * 0.05,
            self.dir[1] + heading[1] / n * 0.08 + toward[1] / n * 0.05,
        ];
        flat(blend).unwrap_or(self.dir)
    }

    /// What frightens it this tick, as the heading to flee along.
    fn fright(&mut self, m: &mut Body, cx: &World<'_>) -> Option<[f64; 2]> {
        let hit = self.health >= 0.0 && m.health < self.health;
        self.health = m.health;
        let told = {
            let mut panic = PANIC.lock().unwrap();
            match panic.get_mut(&m.id) {
                Some((0, away)) => {
                    let away = *away;
                    panic.remove(&m.id);
                    Some(away)
                }
                Some((wait, _)) => {
                    *wait -= 1;
                    None
                }
                None => None,
            }
        };
        if self.rest > 0 {
            self.rest -= 1;
            return None;
        }
        if told.is_some() {
            return told;
        }
        let at = m.position.array();
        let player = cx
            .player
            .as_ref()
            .filter(|p| p.alive && !p.spectator)
            .map(|p| p.position.array())
            .filter(|p| creature::body::distance(*p, at) < FRIGHT);
        let from = if hit { Some(player.unwrap_or(at)) } else { player };
        if let Some(from) = from {
            scare(at, from);
            let away = flat([at[0] - from[0], at[2] - from[2]]).unwrap_or(self.dir);
            PANIC.lock().unwrap().remove(&m.id);
            return Some(away);
        }
        let for_nothing = !cx.raining && m.random.next_int_bound(FOR_NOTHING) == Some(0);
        for_nothing.then_some(self.dir)
    }

    fn swim(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = true;
        if let Some(away) = self.fright(m, cx) {
            self.dir = away;
            self.phase = Phase::Rise(RISE_TICKS);
            self.rise(m, cx);
            return;
        }
        self.dir = self.shoal(m);
        self.wander -= 1;
        if self.wander <= 0 {
            self.wander = 60 + m.random.next_int_bound(80).unwrap_or(0);
            let angle = (f64::from(m.random.next_float()) - 0.5) * 1.6;
            self.dir = turned(self.dir, angle);
        }
        // A wall ahead: it turns away.
        let ahead = Vec3 {
            x: m.position.x + self.dir[0] * 1.2,
            y: m.position.y + 0.2,
            z: m.position.z + self.dir[1] * 1.2,
        };
        if !water(cx, cell(ahead)) {
            let angle = TAU / 4.0 + f64::from(m.random.next_float()) * TAU / 4.0;
            self.dir = turned(self.dir, angle);
        }
        let depth = depth(m, cx);
        let up = if depth > DEPTH.1 {
            0.04
        } else if depth < DEPTH.0 {
            -0.03
        } else {
            0.0
        };
        m.velocity = Vec3 {
            x: self.dir[0] * SWIM_SPEED,
            y: up,
            z: self.dir[1] * SWIM_SPEED,
        };
        self.face(m, 0.0);
    }

    fn rise(&mut self, m: &mut Body, cx: &World<'_>) {
        let Phase::Rise(left) = self.phase else {
            return;
        };
        if !m.in_water {
            // Out: it spreads its fins and glides.
            self.start = m.position.array();
            self.height = GLIDE_HEIGHT.0 + f64::from(m.random.next_float()) * (GLIDE_HEIGHT.1 - GLIDE_HEIGHT.0);
            self.turn = if m.random.next_int_bound(2) == Some(0) {
                0.35
            } else {
                -0.35
            };
            let beats = if cx.raining {
                0
            } else {
                m.random.next_int_bound(BEATS + 1).unwrap_or(0)
            };
            self.phase = Phase::Glide(self.stretch(m, cx), beats);
            GLIDING.lock().unwrap().insert(m.id);
            creature::sounds::queue(m.id, SPLASH, 0.6, 1.2 + m.random.next_float() * 0.2);
            self.glide(m, cx);
            return;
        }
        if left <= 0 {
            self.phase = Phase::Swim;
            return;
        }
        self.phase = Phase::Rise(left - 1);
        m.no_gravity = true;
        m.velocity = Vec3 {
            x: self.dir[0] * RISE_SPEED.0,
            y: RISE_SPEED.1,
            z: self.dir[1] * RISE_SPEED.0,
        };
        self.face(m, -30.0);
    }

    /// A stretch of glide (ticks), shorter in the rain.
    fn stretch(&self, m: &mut Body, cx: &World<'_>) -> i32 {
        let t = GLIDE_TICKS.0 + m.random.next_int_bound(GLIDE_TICKS.1 - GLIDE_TICKS.0).unwrap_or(0);
        if cx.raining { t / 2 } else { t }
    }

    fn glide(&mut self, m: &mut Body, cx: &World<'_>) {
        let (left, beats, dipping) = match self.phase {
            Phase::Glide(l, b) => (l, b, false),
            Phase::Dip(l, b) => (l, b, true),
            _ => return,
        };
        m.no_gravity = true;
        m.fall_distance = 0.0;
        let Some(surface) = surface_under(cx, m.position) else {
            // Over land or too high: it drops.
            return self.land(m);
        };
        // Bumped into something: it drops.
        let moved = m.velocity.x.hypot(m.velocity.z);
        if !dipping && left < GLIDE_TICKS.0 - 1 && moved < GLIDE_SPEED * 0.3 {
            return self.land(m);
        }
        // No water ahead: it turns back toward open water.
        let ahead = Vec3 {
            x: m.position.x + self.dir[0] * 4.0,
            y: m.position.y,
            z: m.position.z + self.dir[1] * 4.0,
        };
        if surface_under(cx, ahead).is_none() {
            self.dir = turned(self.dir, self.turn);
        }
        let target = surface + if dipping { 0.1 } else { self.height };
        let up = ((target - m.position.y) * 0.3).clamp(-0.25, 0.25);
        let speed = if dipping { GLIDE_SPEED * 0.75 } else { GLIDE_SPEED };
        m.velocity = Vec3 {
            x: self.dir[0] * speed,
            y: up,
            z: self.dir[1] * speed,
        };
        self.face(m, 0.0);
        self.phase = match (dipping, left, beats) {
            (_, l, b) if l > 1 => {
                if dipping {
                    Phase::Dip(l - 1, b)
                } else {
                    Phase::Glide(l - 1, b)
                }
            }
            (false, _, 0) => return self.land(m),
            (false, _, b) => {
                creature::sounds::queue(m.id, SPLASH, 0.4, 1.4);
                Phase::Dip(DIP_TICKS, b - 1)
            }
            (true, _, b) => Phase::Glide(self.stretch(m, cx), b),
        };
    }

    /// The glide ends: it falls back into the water (or onto the ground, beached).
    fn land(&mut self, m: &mut Body) {
        let at = m.position.array();
        let far = (at[0] - self.start[0]).hypot(at[2] - self.start[2]);
        if far > LONG_GLIDE {
            GLIDED.lock().unwrap().push(m.id);
        }
        GLIDING.lock().unwrap().remove(&m.id);
        self.phase = Phase::Swim;
        self.rest = REST;
        m.no_gravity = false;
        m.velocity = Vec3 {
            x: self.dir[0] * 0.3,
            y: -0.3,
            z: self.dir[1] * 0.3,
        };
        self.face(m, 20.0);
    }

    fn face(&self, m: &mut Body, pitch: f32) {
        let yaw = yaw(self.dir);
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        m.pitch = pitch;
    }
}

impl Behaviour for Fish {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        FISH.lock().unwrap().insert(m.id, (m.position.array(), self.dir));
        m.in_water || self.phase != Phase::Swim
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        match self.phase {
            Phase::Swim => self.swim(m, cx),
            Phase::Rise(_) => self.rise(m, cx),
            Phase::Glide(..) | Phase::Dip(..) => self.glide(m, cx),
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        if self.phase != Phase::Swim {
            GLIDING.lock().unwrap().remove(&m.id);
            self.phase = Phase::Swim;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heading_turns_and_keeps_its_length() {
        let t = turned([0.0, 1.0], TAU / 4.0);
        assert!((t[0] + 1.0).abs() < 1e-9 && t[1].abs() < 1e-9);
        assert!(flat([0.0, 0.0]).is_none());
        assert!((yaw([0.0, 1.0])).abs() < 1e-6);
    }

    #[test]
    fn a_fright_spreads_through_the_shoal() {
        for id in 900..906 {
            FISH.lock()
                .unwrap()
                .insert(id, ([f64::from(id - 900), 60.0, 0.0], [0.0, 1.0]));
        }
        FISH.lock().unwrap().insert(999, ([100.0, 60.0, 0.0], [0.0, 1.0]));
        scare([0.0, 60.0, 0.0], [-3.0, 60.0, 0.0]);
        let panic = PANIC.lock().unwrap();
        assert!((900..906).all(|id| panic.get(&id).is_some_and(|(_, away)| away[0] > 0.5)));
        assert!(!panic.contains_key(&999));
        assert!(!took_off().is_empty());
    }
}
