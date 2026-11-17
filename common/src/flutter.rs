//! Small fliers (SPECS §3.5): short erratic flights, landing (on flowers), resting; butterflies, moths, fireflies and
//! fairies.
//!
//! A mod gives each of its creatures a `Kind` (out by day or at night, speed, height, how long it flies and rests,
//! drawn to light, landing on flowers) and adds `Flutter::new(&KIND)` to its goals.
//!
//! - It is about by day in fair weather, at night, or at any hour but in the rain (its `Hours`); otherwise it rests
//!   where it is.
//! - Resting, it sits a while (on a flower if it found one and its kind lands on flowers), then flies off again.
//! - Flying, it flutters towards a point a few blocks off, changing its mind often, a little up and down, staying
//!   around where it first fluttered; after a while it looks for a flower near it and lands on it, or on the ground.
//! - A kind drawn to light flutters around the nearest bright light within 10 blocks at its time, never landing there.
//! - A sprinting player close by makes it fly off at once, away from them. No fall damage (1 health).
//! - Settled (released from a jar on a flower, the library's territory), it only flies over the flowers of its patch
//!   and lands on them.
//!
//! **Spawning** (`spawns`): small fliers come out near players, as vanilla's ambient bats do, rather than through
//! Horizon's natural spawning alone, whose creature cap the farm animals fill (it adds creatures only every 400
//! ticks): every `SPAWN_EVERY` ticks, for each player and each kind registered, one chance in its `chance` while fewer
//! than its `cap` are within `SPAWN_NEAR` blocks, a group comes out `SPAWN_MIN` to `SPAWN_MAX` blocks away, over the
//! ground, in one of its biomes, at its hours. Wild ones far from players despawn (`tick_post`), so they come and go.
//!
//! The flight runs from the server tick (`body`: it sets the flier's velocity itself). The drawing needs to know which
//! are flying (`flying`), which their render state doesn't say: a static map, single player only (as the vulture's).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{Host, LevelRef, ServerHost, SoundSource};

use crate::body::{Behaviour, Body, World};
use crate::territory::{self, Ground};

/// When a kind of flier is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hours {
    /// By day in fair weather (butterflies).
    Day,
    /// At night (moths, fireflies).
    Night,
    /// Day and night, but not in the rain (fairies).
    Always,
}

/// How a kind of flier behaves.
#[derive(Debug)]
pub struct Kind {
    /// When it is about.
    pub hours: Hours,
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
    /// The sound of its wings as it takes off (the mod's own event), or none.
    pub takeoff: Option<&'static str>,
}

/// How loud the wings are at take-off.
const TAKEOFF_VOLUME: f32 = 0.5;

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
const LIGHT_MIN: u8 = 14;
const LIGHT_SEARCH: i32 = 10;
const LIGHT_ORBIT: f64 = 1.6;

/// The nearest bright light source around `at` (a block giving at least `LIGHT_MIN`), for a moth.
fn brightest_light(world: &World<'_>, at: [f64; 3]) -> Option<[i32; 3]> {
    let c = block_of(at);
    let mut best: Option<([i32; 3], i32)> = None;
    for dy in -4..=4 {
        for dx in -LIGHT_SEARCH..=LIGHT_SEARCH {
            for dz in -LIGHT_SEARCH..=LIGHT_SEARCH {
                let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                let bright = world.light_emission(p) >= LIGHT_MIN;
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

/// Where small fliers come out (`spawns`): a kind, its flight (for its hours), its biomes, its groups, how many at most
/// near a player, and one chance in how many each try.
pub struct Spawning {
    pub kind: &'static str,
    pub flight: &'static Kind,
    pub biomes: &'static [&'static str],
    pub group: (u32, u32),
    pub cap: usize,
    pub chance: u64,
}

/// How often a try is made (ticks), how near a player the cap counts (blocks), and how far from them a group comes out.
const SPAWN_EVERY: u32 = 200;
const SPAWN_NEAR: f64 = 48.0;
const SPAWN_MIN: f64 = 16.0;
const SPAWN_MAX: f64 = 40.0;
/// A group spreads this far around its spot (blocks), this high over the ground.
const GROUP_SPREAD: f64 = 3.0;
const GROUP_HEIGHT: (f64, f64) = (0.5, 2.5);

static SPAWNING: Mutex<Vec<&'static Spawning>> = Mutex::new(Vec::new());
static SPAWN_TICKS: Mutex<u32> = Mutex::new(0);
/// Each flier's kind (from `register`) and where it is (from `tick_post`), for the caps.
static KINDS: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
static AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());

/// From a mod's `hooks()`: its kind comes out near players.
pub fn spawns(spawning: &'static Spawning) {
    let mut all = SPAWNING.lock().unwrap();
    if !all.iter().any(|s| s.kind == spawning.kind) {
        all.push(spawning);
    }
}

/// Whether a kind out at `hours` is about now in `level` (from a hook: its day time and rain).
fn about(host: &dyn Host, level: LevelRef, hours: Hours) -> bool {
    let day = level.day_time(host).unwrap_or(0).rem_euclid(24_000) < 12_500;
    let raining = level.weather(host).is_some_and(|(rain, _)| rain > 0.2);
    match hours {
        Hours::Day => day && !raining,
        Hours::Night => !day,
        Hours::Always => !raining,
    }
}

/// From the library's `level_tick_post`, in the overworld: groups of small fliers come out near players.
pub fn spawn(host: &mut dyn ServerHost, level: LevelRef) {
    if level.dimension() != mod_api::DimensionKey::OVERWORLD {
        return;
    }
    {
        let mut t = SPAWN_TICKS.lock().unwrap();
        *t += 1;
        if *t < SPAWN_EVERY {
            return;
        }
        *t = 0;
    }
    let spawning: Vec<&'static Spawning> = SPAWNING.lock().unwrap().clone();
    let mut random = crate::colours::Random::new(SPAWN_EVERY);
    let mut float = || (random.next() % 10_000) as f64 / 10_000.0;
    let players = host.players().server_players();
    for player in players {
        if player.level(host) != Some(level) {
            continue;
        }
        let Some(p) = player.entity().position(host) else {
            continue;
        };
        for s in &spawning {
            if !random_chance(&mut float, s.chance) || !about(host, level, s.flight.hours) {
                continue;
            }
            let near = {
                let kinds = KINDS.lock().unwrap();
                AT.lock()
                    .unwrap()
                    .iter()
                    .filter(|(id, at)| {
                        kinds.get(id).is_some_and(|k| k == s.kind) && (at[0] - p[0]).hypot(at[2] - p[2]) < SPAWN_NEAR
                    })
                    .count()
            };
            if near >= s.cap {
                continue;
            }
            let angle = float() * std::f64::consts::TAU;
            let distance = SPAWN_MIN + float() * (SPAWN_MAX - SPAWN_MIN);
            let (x, z) = (p[0] + angle.cos() * distance, p[2] + angle.sin() * distance);
            let Some(ground) = surface(
                Ground::new(&*host, level),
                [x.floor() as i32, p[1] as i32, z.floor() as i32],
            ) else {
                continue;
            };
            if !level
                .biome_at(host, [x.floor() as i32, ground, z.floor() as i32])
                .is_some_and(|b| s.biomes.contains(&b.as_str()))
            {
                continue;
            }
            let count = s.group.0 + (float() * f64::from(s.group.1 - s.group.0 + 1)) as u32;
            for _ in 0..count.min(s.group.1) {
                let at = [
                    x + (float() * 2.0 - 1.0) * GROUP_SPREAD,
                    f64::from(ground) + GROUP_HEIGHT.0 + float() * (GROUP_HEIGHT.1 - GROUP_HEIGHT.0),
                    z + (float() * 2.0 - 1.0) * GROUP_SPREAD,
                ];
                let dimension = level.dimension().id(host).unwrap_or_default();
                host.summon(&dimension, s.kind, at, mod_api::Compound::new());
            }
        }
    }
}

/// One chance in `n`, from `float` (0 to 1).
fn random_chance(float: &mut impl FnMut() -> f64, n: u64) -> bool {
    float() * (n as f64) < 1.0
}

/// The first open block over the ground at `at`'s column, from 16 blocks above `at` down to 16 below (not over water).
pub fn surface(ground: Ground<'_>, at: [i32; 3]) -> Option<i32> {
    let open = |name: &str| name == "minecraft:air" || name == "minecraft:cave_air";
    (at[1] - 16..=at[1] + 16).rev().find(|&y| {
        let below = ground.block([at[0], y - 1, at[2]]);
        open(&ground.block([at[0], y, at[2]])) && !open(&below) && below != "minecraft:water"
    })
}

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

#[derive(Debug)]
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

    fn awake(&self, world: &World<'_>) -> bool {
        match self.kind.hours {
            Hours::Day => world.daylight && !world.raining,
            Hours::Night => !world.daylight,
            Hours::Always => !world.raining,
        }
    }
}

fn between(m: &mut Body, range: (i32, i32)) -> i32 {
    range.0 + m.random.next_int_bound(range.1 - range.0).unwrap_or(0)
}

fn block_of(p: [f64; 3]) -> [i32; 3] {
    [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32]
}

/// The top of the ground at `x`, `z` near height `y` (the first air block above it), `y` where none is found.
fn ground(world: &World<'_>, x: i32, y: f64, z: i32) -> f64 {
    surface(world.ground, [x, y.floor() as i32, z]).map_or(y, f64::from)
}

impl Flutter {
    /// A point to flutter to: over its patch when settled, else a few blocks around, a little above the ground.
    fn wander(&self, m: &mut Body, cx: &World<'_>) -> [f64; 3] {
        let r = |m: &mut Body| f64::from(m.random.next_float()) * 2.0 - 1.0;
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
        [
            x,
            ground(cx, x.floor() as i32, m.position.y, z.floor() as i32) + height,
            z,
        ]
    }

    /// Where to land: a flower near it (one of its patch's when settled), else the ground below it.
    fn landing(&self, m: &Body, cx: &World<'_>) -> ([f64; 3], bool) {
        let here = block_of([m.position.x, m.position.y, m.position.z]);
        let flowers = if !self.patch.is_empty() {
            self.patch.clone()
        } else if self.kind.lands_on_flowers {
            territory::flowers_near(cx.ground, here, FLOWER_SEARCH)
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
            None => (
                [m.position.x, ground(cx, here[0], m.position.y, here[2]), m.position.z],
                false,
            ),
        }
    }

    fn take_off(&mut self, m: &mut Body, cx: &World<'_>) {
        if let Some(sound) = self.kind.takeoff {
            crate::sounds::queue(m.id, sound, TAKEOFF_VOLUME, 0.9 + m.random.next_float() * 0.2);
        }
        self.mode = Mode::Fly;
        self.ticks = between(m, self.kind.flight);
        self.target = Some(self.wander(m, cx));
        self.perched = false;
    }

    fn steer(&self, m: &mut Body, target: [f64; 3], speed: f64) -> bool {
        steer(m, target, speed)
    }
}

/// Steers a flier towards `target` with a flutter, at most `speed` blocks a tick, facing where it goes; whether it is
/// there (also used by a mod's own flights: a fairy following its friend).
pub fn steer(m: &mut Body, target: [f64; 3], speed: f64) -> bool {
    let d = [
        target[0] - m.position.x,
        target[1] - m.position.y,
        target[2] - m.position.z,
    ];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len < 0.3 {
        return true;
    }
    let jitter = |m: &mut Body| (f64::from(m.random.next_float()) * 2.0 - 1.0) * JITTER;
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

impl Behaviour for Flutter {
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        // No fall damage: with 1 health, a drop off a ledge would kill it.
        m.fall_distance = 0.0;
        if self.home.is_none() {
            self.home = Some([m.position.x, m.position.y, m.position.z]);
        }
        self.check -= 1;
        if self.check <= 0 {
            self.check = CHECK_EVERY;
            self.patch = territory::check(cx.host(), m.id);
            if self.kind.drawn_to_light {
                self.light = brightest_light(cx, [m.position.x, m.position.y, m.position.z]);
            }
        }
        // A sprinting player close by: off it goes, away from them.
        if let Some(p) = cx.player.as_ref()
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
                ground(cx, x.floor() as i32, m.position.y, z.floor() as i32) + self.kind.height.1,
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
}

/// From the mod's `register`, for one of its fliers (`butterflies:butterfly`): its own tiny box, its template's goals
/// taken off (it flies by `tick_post`).
pub fn register(kind: &'static str, size: crate::mob::Size) {
    crate::mob::ours(kind, size);
}

/// From the mod's `entity_tick_post`, for each of its fliers (of entity type `kind`): its flight (`flight` gives its
/// behaviours the first time: its `Flutter`, and a mod's own before it, a fairy's following), its territory loaded
/// and saved, and a wild one despawned far from players (a settled or befriended one stays; the mod ticks `friend`
/// first).
pub fn tick_post(
    host: &mut dyn ServerHost,
    dimension: &str,
    kind: &str,
    entity: u32,
    flight: impl FnOnce() -> Vec<Box<dyn Behaviour>>,
) {
    crate::body::tick(host, entity, flight);
    crate::sounds::play_queued(host, entity, SoundSource::Neutral);
    territory::tick(host, entity);
    KINDS.lock().unwrap().insert(entity, kind.to_string());
    if let Some(at) = host.entity(entity).and_then(|e| e.position(host)) {
        AT.lock().unwrap().insert(entity, at);
    }
    // A settled or befriended one stays.
    let kept = territory::settled(entity) || crate::friend::of(entity).is_some();
    if crate::mob::despawn_far(host, dimension, entity, kept) {
        gone(entity);
    }
}

/// From the mod's `living_death` (and when one despawns): what the library kept about it is dropped.
pub fn gone(entity: u32) {
    crate::body::forget(entity);
    crate::colours::forget(entity);
    territory::forget(entity);
    forget(entity);
    KINDS.lock().unwrap().remove(&entity);
    AT.lock().unwrap().remove(&entity);
}
