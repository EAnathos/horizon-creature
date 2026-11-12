//! How butterflies and moths move (SPECS §2, §2.1, §3): short erratic flights, landing on flowers, resting.
//!
//! - A butterfly is about by day in fair weather, a moth at night; otherwise each rests where it is.
//! - Resting, it sits a few seconds to half a minute (on a flower if it found one), then flies off again.
//! - Flying, it flutters towards a point a few blocks off, changing its mind often, a little up and down; after a few
//!   seconds it looks for a flower near it and lands on it, or on the ground.
//! - A sprinting player close by makes it fly off at once, away from them.
//! - Settled (released from a jar on a flower, the library's territory), it only flies over the flowers of its patch
//!   and lands on them.
//!
//! The drawing needs to know which are flying (`flying`), which their render state doesn't say: a static map, single
//! player only (as the vulture's; arismoko/horizon#2740).
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::territory;
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

/// Flying speed (blocks a tick), how quickly it turns towards its target, and its flutter.
const SPEED: f64 = 0.16;
const STEER: f64 = 0.035;
const JITTER: f64 = 0.03;
/// How long a flight and a rest last (ticks), at least and at most.
const FLIGHT: (i32, i32) = (80, 240);
const REST: (i32, i32) = (100, 600);
/// How far it wanders off on a flight (a wild one stays within 1.5 times this of where it first fluttered), and how high
/// above the ground it flies.
const WANDER: f64 = 6.0;
const HEIGHT: (f64, f64) = (1.0, 3.5);
/// How far it looks for a flower to land on.
const FLOWER_SEARCH: i32 = 6;
/// A sprinting player this close makes it flee.
const FLEE: f64 = 4.0;
/// How often a settled one checks its patch (ticks).
const CHECK_EVERY: i32 = 100;
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

#[derive(Clone, Debug, Default)]
pub struct Flutter {
    /// A moth (out at night) rather than a butterfly (out by day).
    moth: bool,
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
}

impl Flutter {
    pub fn new(moth: bool) -> Flutter {
        Flutter {
            moth,
            ticks: 40,
            ..Flutter::default()
        }
    }

    fn awake(&self, cx: &Cx) -> bool {
        if self.moth {
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
        let home = self.home.unwrap_or([m.position.x, m.position.y, m.position.z]);
        let (x, z) = match self.patch.len() {
            0 => (home[0] + r(m) * WANDER * 1.5, home[2] + r(m) * WANDER * 1.5),
            n => {
                let f = self.patch[m.random.next_int_bound(n as i32).unwrap_or(0) as usize];
                let reach = f64::from(territory::AROUND);
                (
                    f64::from(f[0]) + 0.5 + r(m) * reach,
                    f64::from(f[2]) + 0.5 + r(m) * reach,
                )
            }
        };
        let height = HEIGHT.0 + f64::from(m.random.next_float()) * (HEIGHT.1 - HEIGHT.0);
        [x, ground(cx, x.floor() as i32, z.floor() as i32) + height, z]
    }

    /// Where to land: a flower near it (one of its patch's when settled), else the ground below it.
    fn landing(&self, m: &Mob, cx: &Cx) -> ([f64; 3], bool) {
        let here = block_of([m.position.x, m.position.y, m.position.z]);
        let flowers = if self.patch.is_empty() {
            territory::flowers_near(cx, here, FLOWER_SEARCH)
        } else {
            self.patch.clone()
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
        self.ticks = between(m, FLIGHT);
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
                m.position.x + away[0] / len * WANDER,
                m.position.z + away[1] / len * WANDER,
            );
            self.target = Some([x, ground(cx, x.floor() as i32, z.floor() as i32) + HEIGHT.1, z]);
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
                        self.ticks = REST.0;
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
                if self.steer(m, target, SPEED) || m.random.next_int_bound(40).unwrap_or(1) == 0 {
                    self.target = Some(self.wander(m, cx));
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
                let there = self.steer(m, target, SPEED * 0.6);
                // Down, or too long trying: it rests where it is.
                if there || m.on_ground || self.ticks <= 0 {
                    m.velocity.x = 0.0;
                    m.velocity.y = 0.0;
                    m.velocity.z = 0.0;
                    self.perched = there && self.perched;
                    self.mode = Mode::Rest;
                    self.ticks = between(m, REST);
                }
            }
        }
        FLYING.lock().unwrap().insert(m.id, self.mode != Mode::Rest);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
