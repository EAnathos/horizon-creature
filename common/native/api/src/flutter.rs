//! Small fliers (SPECS §3.5): short erratic flights, landing (on flowers), resting; butterflies, moths and fireflies.
//!
//! A mod gives each of its creatures a `Kind` (out by day or at night, speed, height, how long it flies and rests,
//! drawn to light, landing on flowers) and adds `Flutter::new(&KIND)` to its goals.
//!
//! - It is about by day in fair weather (or at night, for a night kind); otherwise it rests where it is.
//! - Resting, it sits a while (on a flower if it found one and its kind lands on flowers), then flies off again.
//! - Flying, it flutters towards a point a few blocks off, changing its mind often, a little up and down, staying
//!   around where it first fluttered; after a while it looks for a flower near it and lands on it, or on the ground.
//! - A kind drawn to light flutters around the nearest bright light within 10 blocks at its time, never landing there.
//! - A sprinting player close by makes it fly off at once, away from them. No fall damage (1 health).
//! - Settled (released from a jar on a flower, the library's territory), it only flies over the flowers of its patch
//!   and lands on them.
//!
//! The drawing needs to know which are flying (`flying`), which their render state doesn't say: a static map, single
//! player only (as the vulture's; arismoko/horizon#2740).
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::territory;
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Host, Mob, RandomSource};

/// How a kind of flier behaves.
#[derive(Debug)]
pub struct Kind {
    /// Out at night (moths, fireflies) rather than by day in fair weather (butterflies).
    pub night: bool,
    /// Flying speed, blocks a tick (a butterfly 0.16).
    pub speed: f64,
    /// How high above the ground it flies, at least and at most.
    pub height: (f64, f64),
    /// How long a flight and a rest last (ticks), at least and at most.
    pub flight: (i32, i32),
    pub rest: (i32, i32),
    /// How far it wanders off on a flight (it stays within 1.5 times this of where it first fluttered).
    pub wander: f64,
    /// Drawn to bright lights at its time (moths).
    pub drawn_to_light: bool,
    /// Lands on flowers (else on the ground).
    pub lands_on_flowers: bool,
}

/// How quickly it turns towards its target, and its flutter.
const STEER: f64 = 0.035;
const JITTER: f64 = 0.03;
/// How far it looks for a flower to land on.
const FLOWER_SEARCH: i32 = 6;
/// A sprinting player this close makes it flee.
const FLEE: f64 = 4.0;
/// How often a settled one checks its patch (ticks).
const CHECK_EVERY: i32 = 100;
/// A moth is drawn to light sources this bright (a torch is 14, a lantern 15, a campfire 15) within `LIGHT_SEARCH`
/// blocks, and flutters this close around one.
const LIGHT_MIN: u8 = 10;
const LIGHT_SEARCH: i32 = 10;
const LIGHT_ORBIT: f64 = 1.6;

/// The nearest bright light source around `at` (a block giving at least `LIGHT_MIN`), for a moth.
fn brightest_light(cx: &Cx, at: [f64; 3]) -> Option<[i32; 3]> {
    let c = block_of(at);
    let world = cx.w.world;
    let mut best: Option<([i32; 3], i32)> = None;
    for dy in -4..=4 {
        for dx in -LIGHT_SEARCH..=LIGHT_SEARCH {
            for dz in -LIGHT_SEARCH..=LIGHT_SEARCH {
                let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                let state = world.block(p[0], p[1], p[2]);
                let bright =
                    cx.w.facts
                        .states
                        .get(state.0 as usize)
                        .is_some_and(|f| f.emission >= LIGHT_MIN);
                let d = dx * dx + dy * dy + dz * dz;
                if bright && best.is_none_or(|b| d < b.1) {
                    best = Some((p, d));
                }
            }
        }
    }
    best.map(|b| b.0)
}

/// Where on a flower it sits, above the flower block's bottom.
const ON_FLOWER: f64 = 0.4;

static FLYING: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

/// Whether it is flying, for the drawing.
pub fn flying(id: u32) -> bool {
    FLYING.lock().unwrap().get(&id).copied().unwrap_or(false)
}

pub fn forget(id: u32) {
    FLYING.lock().unwrap().remove(&id);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Rest,
    Fly,
    Land,
}

#[derive(Clone, Debug)]
pub struct Flutter {
    kind: &'static Kind,
    mode: Mode,
    ticks: i32,
    target: Option<[f64; 3]>,
    /// Sitting on a flower (held up, no gravity).
    perched: bool,
    /// Its patch's flowers when settled, checked every `CHECK_EVERY` ticks.
    patch: Vec<[i32; 3]>,
    check: i32,
    /// Where a wild one first fluttered: it wanders around it rather than drifting off for good.
    home: Option<[f64; 3]>,
    /// A moth's light: the brightest light source near it, looked for every `CHECK_EVERY` ticks (mods/butterflies/SPECS.md §2.1).
    light: Option<[i32; 3]>,
}

impl Flutter {
    pub fn new(kind: &'static Kind) -> Flutter {
        Flutter {
            kind,
            mode: Mode::Rest,
            ticks: 40,
            target: None,
            perched: false,
            patch: Vec::new(),
            check: 0,
            home: None,
            light: None,
        }
    }

    fn awake(&self, cx: &Cx) -> bool {
        if self.kind.night {
            !cx.w.daylight
        } else {
            cx.w.daylight && !cx.w.raining
        }
    }
}

fn between(m: &mut Mob, range: (i32, i32)) -> i32 {
    range.0 + m.random.next_int_bound(range.1 - range.0).unwrap_or(0)
}

fn block_of(p: [f64; 3]) -> [i32; 3] {
    [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]
}

/// The top of the ground at `x`, `z` (the first air block above it).
fn ground(cx: &Cx, x: i32, z: i32) -> f64 {
    let world = cx.w.world;
    f64::from(world.world_surface(x, z, |s| world.is_air(s)))
}

impl Flutter {
    /// A point to flutter to: over its patch when settled, else a few blocks around, a little above the ground.
    fn wander(&self, m: &mut Mob, cx: &Cx) -> [f64; 3] {
        let r = |m: &mut Mob| f64::from(m.random.next_float()) * 2.0 - 1.0;
        // A moth flutters around a light it found, close and all around it, a little above and below.
        if let Some(l) = self.light {
            return [
                f64::from(l[0]) + 0.5 + r(m) * LIGHT_ORBIT,
                f64::from(l[1]) + 0.5 + r(m) * LIGHT_ORBIT,
                f64::from(l[2]) + 0.5 + r(m) * LIGHT_ORBIT,
            ];
        }
        let home = self.home.unwrap_or([m.position.x, m.position.y, m.position.z]);
        let (x, z) = match self.patch.len() {
            0 => (
                home[0] + r(m) * self.kind.wander * 1.5,
                home[2] + r(m) * self.kind.wander * 1.5,
            ),
            n => {
                let f = self.patch[m.random.next_int_bound(n as i32).unwrap_or(0) as usize];
                let reach = f64::from(territory::AROUND);
                (
                    f64::from(f[0]) + 0.5 + r(m) * reach,
                    f64::from(f[2]) + 0.5 + r(m) * reach,
                )
            }
        };
        let height = self.kind.height.0 + f64::from(m.random.next_float()) * (self.kind.height.1 - self.kind.height.0);
        [x, ground(cx, x.floor() as i32, z.floor() as i32) + height, z]
    }

    /// Where to land: a flower near it (one of its patch's when settled), else the ground below it.
    fn landing(&self, m: &Mob, cx: &Cx) -> ([f64; 3], bool) {
        let here = block_of([m.position.x, m.position.y, m.position.z]);
        let flowers = if !self.patch.is_empty() {
            self.patch.clone()
        } else if self.kind.lands_on_flowers {
            territory::flowers_near(cx, here, FLOWER_SEARCH)
        } else {
            Vec::new()
        };
        let distance = |f: &[i32; 3]| (0..3).map(|k| (f[k] - here[k]).pow(2)).sum::<i32>();
        match flowers.iter().min_by_key(|f| distance(f)) {
            Some(f) => (
                [
                    f64::from(f[0]) + 0.5,
                    f64::from(f[1]) + ON_FLOWER,
                    f64::from(f[2]) + 0.5,
                ],
                true,
            ),
            None => ([m.position.x, ground(cx, here[0], here[2]), m.position.z], false),
        }
    }

    fn take_off(&mut self, m: &mut Mob, cx: &Cx) {
        self.mode = Mode::Fly;
        self.ticks = between(m, self.kind.flight);
        self.target = Some(self.wander(m, cx));
        self.perched = false;
    }

    /// Steers towards `target` with a flutter; whether it is there.
    fn steer(&self, m: &mut Mob, target: [f64; 3], speed: f64) -> bool {
        let d = [
            target[0] - m.position.x,
            target[1] - m.position.y,
            target[2] - m.position.z,
        ];
        let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if len < 0.3 {
            return true;
        }
        let jitter = |m: &mut Mob| (f64::from(m.random.next_float()) * 2.0 - 1.0) * JITTER;
        let (jx, jy, jz) = (jitter(m), jitter(m), jitter(m));
        m.velocity.x = m.velocity.x * 0.85 + d[0] / len * STEER + jx;
        m.velocity.y = m.velocity.y * 0.85 + d[1] / len * STEER + jy;
        m.velocity.z = m.velocity.z * 0.85 + d[2] / len * STEER + jz;
        let v = (m.velocity.x.powi(2) + m.velocity.y.powi(2) + m.velocity.z.powi(2)).sqrt();
        if v > speed {
            let k = speed / v;
            m.velocity.x *= k;
            m.velocity.y *= k;
            m.velocity.z *= k;
        }
        let yaw = (-m.velocity.x).atan2(m.velocity.z).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        false
    }
}

impl Goal for Flutter {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        // No fall damage: with 1 health, a drop off a ledge would kill it.
        m.fall_distance = 0.0;
        if self.home.is_none() {
            self.home = Some([m.position.x, m.position.y, m.position.z]);
        }
        self.check -= 1;
        if self.check <= 0 {
            self.check = CHECK_EVERY;
            self.patch = territory::check(cx, m.id);
            if self.kind.drawn_to_light {
                self.light = brightest_light(cx, [m.position.x, m.position.y, m.position.z]);
            }
        }
        // A sprinting player close by: off it goes, away from them.
        if let Some(p) = cx.w.player.as_ref()
            && p.sprinting
            && p.alive
            && (p.position.x - m.position.x).hypot(p.position.z - m.position.z) < FLEE
            && self.mode != Mode::Fly
        {
            self.take_off(m, cx);
            let away = [m.position.x - p.position.x, m.position.z - p.position.z];
            let len = away[0].hypot(away[1]).max(0.1);
            let (x, z) = (
                m.position.x + away[0] / len * self.kind.wander,
                m.position.z + away[1] / len * self.kind.wander,
            );
            self.target = Some([
                x,
                ground(cx, x.floor() as i32, z.floor() as i32) + self.kind.height.1,
                z,
            ]);
        }
        let awake = self.awake(cx);
        self.ticks -= 1;
        match self.mode {
            Mode::Rest => {
                m.velocity.x = 0.0;
                m.velocity.z = 0.0;
                if self.perched {
                    m.velocity.y = 0.0;
                }
                m.no_gravity = self.perched;
                if self.ticks <= 0 {
                    if awake {
                        self.take_off(m, cx);
                    } else {
                        self.ticks = self.kind.rest.0;
                    }
                }
            }
            Mode::Fly => {
                m.no_gravity = true;
                let target = match self.target {
                    Some(t) => t,
                    None => self.wander(m, cx),
                };
                // There, or changing its mind: somewhere else.
                if self.steer(m, target, self.kind.speed) || m.random.next_int_bound(40).unwrap_or(1) == 0 {
                    self.target = Some(self.wander(m, cx));
                }
                // Around a light a moth keeps fluttering.
                if self.ticks <= 0 && awake && self.light.is_some() {
                    self.ticks = self.kind.flight.0;
                }
                if self.ticks <= 0 || !awake {
                    let (spot, on_flower) = self.landing(m, cx);
                    self.mode = Mode::Land;
                    self.target = Some(spot);
                    self.perched = on_flower;
                    self.ticks = 200;
                }
            }
            Mode::Land => {
                m.no_gravity = true;
                let target = self.target.unwrap_or([m.position.x, m.position.y, m.position.z]);
                let there = self.steer(m, target, self.kind.speed * 0.6);
                // Down, or too long trying: it rests where it is.
                if there || m.on_ground || self.ticks <= 0 {
                    m.velocity.x = 0.0;
                    m.velocity.y = 0.0;
                    m.velocity.z = 0.0;
                    self.perched = there && self.perched;
                    self.mode = Mode::Rest;
                    self.ticks = between(m, self.kind.rest);
                }
            }
        }
        FLYING.lock().unwrap().insert(m.id, self.mode != Mode::Rest);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// From the mod's `register_goals`, for one of its fliers: muted, its own tiny box (blocks: width, height, eye height),
/// its template's goals replaced by this flight.
pub fn register(mob: &mut Mob, size: (f64, f64, f64), kind: &'static Kind) {
    crate::mob::ours(mob, size.0, size.1, size.2);
    mob.ai.goals.add(1, Flutter::new(kind));
}

/// From the mod's `entity_tick_post`, for each of its fliers: its territory loaded and saved, and a wild one despawned
/// far from players (a settled one stays).
pub fn tick_post(host: &mut dyn Host, dimension: &str, entity: u32) {
    territory::tick(host, entity);
    let settled = territory::settled(entity);
    if crate::mob::despawn_far(host, dimension, entity, settled) {
        gone(entity);
    }
}

/// From the mod's `living_death` (and when one despawns): what the library kept about it is dropped.
pub fn gone(entity: u32) {
    crate::colours::forget(entity);
    territory::forget(entity);
    forget(entity);
}
