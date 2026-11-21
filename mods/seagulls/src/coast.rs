//! What a seagull does (SPECS §2, §3), by hand (the library's `body`), from the server tick. Its flight is the
//! library's (`creature::flight`); what a behaviour can't do itself (take from a hand, eat an item, give a fish,
//! befriend, teach a fact) it queues as an `Action`, which the mod's tick applies (`crate::act`).
//!
//! In priority order: `Carry` (off with stolen food), `Snatch` (the food thief), `Hunt` (a gliding flying fish),
//! `Dive` (fishing for its friend), `FollowBoat` (a wild one behind a boat), `LuckySpot` (circling over a lucky spot
//! for its friend), `FollowFriend`, `Stay`, `Eat` (food on the ground), the library's `Soar`, `Float` (bobbing on the
//! water), `Walk`; `Upkeep` watches every tick (no fall damage, its calls).
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::body::{Behaviour, Body, Vec3, World};
use creature::flight::{Controls, Flight, heading_of};

use crate::{CRY, LAUGH, SPLASH, WINGS};

/// What the mod's tick does for a behaviour.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Action {
    /// Takes one of what the player (entity id) holds in their main hand.
    Steal { gull: u32, player: u32 },
    /// Eats the stolen food: gone.
    Swallow { gull: u32 },
    /// Hit while carrying food: drops it.
    DropCarried { gull: u32 },
    /// Eats the item (entity id) on the ground; `befriends` a player (UUID) when it was a fish they threw.
    Eat {
        gull: u32,
        item: u32,
        befriends: Option<u128>,
    },
    /// Drops a fish at its friend's feet.
    GiveFish { gull: u32, at: [f64; 3] },
    /// A player (UUID) learns a fact.
    Learn { player: u128, fact: &'static str },
}

static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

fn queue(action: Action) {
    ACTIONS.lock().unwrap().push(action);
}

/// What the behaviours queued since the last call.
pub(crate) fn actions() -> Vec<Action> {
    std::mem::take(&mut *ACTIONS.lock().unwrap())
}

/// What the mod's tick finds around each seagull (every 10 ticks): the nearest food on the ground, the player in a
/// boat it may follow, its friend's bobber in the water, its friend in a boat.
#[derive(Clone, Debug, Default)]
pub(crate) struct Around {
    /// The item's entity id, where it is, whether it is a raw fish, and who threw it (an entity id).
    pub food: Option<(u32, [f64; 3], bool, Option<u32>)>,
    /// A player (entity id and UUID) in a boat on the water, and where they are.
    pub boater: Option<(u32, u128, [f64; 3])>,
    /// Its friend's bobber in the water.
    pub bobber: Option<[f64; 3]>,
    /// Its friend sits in a boat.
    pub friend_boating: bool,
}

static AROUND: Mutex<BTreeMap<u32, Around>> = Mutex::new(BTreeMap::new());

pub(crate) fn set_around(id: u32, around: Around) {
    AROUND.lock().unwrap().insert(id, around);
}

fn around(id: u32) -> Around {
    AROUND.lock().unwrap().get(&id).cloned().unwrap_or_default()
}

/// For its drawing (single player: the renderer reads them): diving with its wings folded back, floating.
static DIVING: Mutex<BTreeMap<u32, ()>> = Mutex::new(BTreeMap::new());
/// What each seagull carries (stolen food: its item id), for the hit that makes it drop it and the drawing.
static CARRIED: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
/// Each friend's lucky spot, by the seagull circling over it: (the spot, its friend's UUID).
static LUCKY: Mutex<BTreeMap<u32, ([f64; 3], u128)>> = Mutex::new(BTreeMap::new());

pub(crate) fn diving(id: u32) -> bool {
    DIVING.lock().unwrap().contains_key(&id)
}

pub(crate) fn carried(id: u32) -> Option<String> {
    CARRIED.lock().unwrap().get(&id).cloned()
}

pub(crate) fn set_carried(id: u32, item: Option<String>) {
    let mut all = CARRIED.lock().unwrap();
    match item {
        Some(i) => all.insert(id, i),
        None => all.remove(&id),
    };
}

/// The lucky spots of `player`'s seagulls.
pub(crate) fn lucky_spots(player: u128) -> Vec<[f64; 3]> {
    LUCKY
        .lock()
        .unwrap()
        .values()
        .filter(|(_, p)| *p == player)
        .map(|(at, _)| *at)
        .collect()
}

pub(crate) fn forget(id: u32) {
    AROUND.lock().unwrap().remove(&id);
    DIVING.lock().unwrap().remove(&id);
    CARRIED.lock().unwrap().remove(&id);
    LUCKY.lock().unwrap().remove(&id);
}

/// The behaviours, in priority order.
pub(crate) fn behaviours() -> Vec<Box<dyn Behaviour>> {
    vec![
        Box::new(Upkeep),
        Box::new(Carry::default()),
        Box::new(Snatch::default()),
        Box::new(Hunt::default()),
        Box::new(Dive::default()),
        Box::new(FollowBoat::default()),
        Box::new(LuckySpot::default()),
        Box::new(FollowFriend::default()),
        Box::new(Stay),
        Box::new(Eat::default()),
        Box::new(creature::flight::Soar::new(&crate::SOARING, &WINGS)),
        Box::new(Float::default()),
        Box::new(Walk::default()),
    ]
}

/// Its friend, if it has one and they are the nearest player: (their UUID, whether it stays, where they are).
fn friend(m: &Body, cx: &World<'_>) -> Option<(u128, bool, [f64; 3])> {
    let f = creature::friend::of(m.id)?;
    let p = cx.player.as_ref().filter(|p| f.is(p.uuid) && p.alive)?;
    Some((p.uuid, f.staying, p.position.array()))
}

fn wild(m: &Body) -> bool {
    creature::friend::of(m.id).is_none()
}

fn flat(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).hypot(a[2] - b[2])
}

fn chance(m: &mut Body, one_in: i32) -> bool {
    m.random.next_int_bound(one_in) == Some(0)
}

/// Flying towards `at` (its feet's place): heading for it, climbing or diving to its height, beating its wings when
/// below it or slow.
fn fly_to(flight: &mut Flight, m: &mut Body, at: [f64; 3]) {
    let (dx, dz) = (at[0] - m.position.x, at[2] - m.position.z);
    let below = at[1] - m.position.y;
    let across = dx.hypot(dz).max(0.5);
    let pitch = (-below).atan2(across).to_degrees().clamp(-45.0, 60.0) as f32;
    let controls = Controls {
        yaw: heading_of(dx, dz),
        pitch,
        flap: below > -1.0 || flight.speed < WINGS.cruise * 0.6,
        brake: across < 4.0 && flight.speed > WINGS.cruise,
        boost: m.on_ground || m.in_water,
        ridden: false,
    };
    flight.tick(m, &controls, &WINGS);
}

/// Every tick: no fall damage (it flies), its calls now and then by day, louder from a wild one.
#[derive(Debug)]
struct Upkeep;

impl Behaviour for Upkeep {
    fn claims(&self) -> bool {
        false
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        if !cx.daylight {
            return;
        }
        if chance(m, 400) {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            creature::sounds::queue(m.id, CRY, 1.0, pitch);
        } else if wild(m) && chance(m, 1500) {
            creature::sounds::queue(m.id, LAUGH, 1.0, 1.0);
        }
    }
}

/// The foods it snatches from a hand (SPECS §2): cooked fish and bread above all, then the rest.
pub(crate) const FOODS: &[&str] = &[
    "minecraft:cooked_cod",
    "minecraft:cooked_salmon",
    "minecraft:bread",
    "minecraft:cod",
    "minecraft:salmon",
    "minecraft:tropical_fish",
    "minecraft:cooked_chicken",
    "minecraft:cooked_beef",
    "minecraft:cooked_porkchop",
    "minecraft:cooked_mutton",
    "minecraft:cooked_rabbit",
    "minecraft:baked_potato",
    "minecraft:cookie",
    "minecraft:pumpkin_pie",
    "minecraft:apple",
    "minecraft:melon_slice",
    "minecraft:carrot",
    "minecraft:golden_carrot",
    "flying_fish:raw_flying_fish",
    "flying_fish:cooked_flying_fish",
    "seagulls:cooked_seagull",
];

/// A thief's try: one chance in this many ticks (about one in 4 a minute), then a rest; how near the player must be,
/// and how long it tries.
const SNATCH_ONE_IN: i32 = 4800;
const SNATCH_REST: i32 = 1200;
const SNATCH_RANGE: f64 = 12.0;
const SNATCH_FOR: i32 = 200;

/// The food thief (SPECS §2): a wild seagull near a player holding food in their main hand (not sneaking) swoops down,
/// hovers at their hand and takes one, then flies off with it (`Carry`).
#[derive(Debug, Default)]
struct Snatch {
    flight: Flight,
    ticks: i32,
    rest: i32,
    on: bool,
}

/// Where to take the food from: just in front of the player's eyes, a little below.
fn hand_of(cx: &World<'_>) -> Option<(u32, [f64; 3])> {
    let p = cx.player.as_ref()?;
    let at = [p.eye[0] + p.look[0] * 0.6, p.eye[1] - 0.7, p.eye[2] + p.look[2] * 0.6];
    Some((p.entity_id, at))
}

fn tempting(cx: &World<'_>) -> bool {
    cx.player.as_ref().is_some_and(|p| {
        p.alive && !p.spectator && !p.sneaking && p.main_hand.as_deref().is_some_and(|h| FOODS.contains(&h))
    })
}

impl Behaviour for Snatch {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        self.rest = (self.rest - 1).max(0);
        let near = cx
            .player
            .as_ref()
            .is_some_and(|p| creature::body::distance(p.position.array(), m.position.array()) < SNATCH_RANGE);
        let on = wild(m) && carried(m.id).is_none() && tempting(cx) && near;
        let on = if self.on {
            on && self.ticks < SNATCH_FOR
        } else {
            on && self.rest == 0 && chance(m, SNATCH_ONE_IN)
        };
        if on && !self.on {
            self.flight = Flight::default();
            self.ticks = 0;
        }
        if !on && self.on {
            self.rest = SNATCH_REST;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        self.ticks += 1;
        let Some((player, at)) = hand_of(cx) else {
            return;
        };
        if self.ticks == 1 {
            creature::sounds::queue(m.id, CRY, 1.2, 1.1);
        }
        let d = creature::body::distance(m.position.array(), at);
        if d < 1.2 {
            queue(Action::Steal { gull: m.id, player });
            self.ticks = SNATCH_FOR;
            return;
        }
        if flat(m.position.array(), at) < 4.0 {
            let yaw = heading_of(at[0] - m.position.x, at[2] - m.position.z);
            self.flight.hover(m, at, yaw, &WINGS);
        } else {
            fly_to(&mut self.flight, m, [at[0], at[1] + 1.0, at[2]]);
        }
    }
}

/// How long it flies off with stolen food before it eats it (ticks).
const CARRY_FOR: i32 = 100;

/// Off with stolen food (SPECS §2): up and away from the player, then it eats it. Hit, it drops it.
#[derive(Debug, Default)]
struct Carry {
    flight: Flight,
    ticks: i32,
    health: f32,
    away: f32,
}

impl Behaviour for Carry {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = carried(m.id).is_some();
        if on && self.ticks == 0 {
            self.flight = Flight::default();
            self.health = m.health;
            self.away = cx.player.as_ref().map_or(m.yaw, |p| {
                heading_of(m.position.x - p.position.x, m.position.z - p.position.z)
            });
        }
        if !on {
            self.ticks = 0;
        }
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.ticks = 0;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        self.ticks += 1;
        if m.health < self.health {
            queue(Action::DropCarried { gull: m.id });
            return;
        }
        if self.ticks >= CARRY_FOR {
            queue(Action::Swallow { gull: m.id });
            creature::sounds::queue(m.id, LAUGH, 1.0, 1.0);
            return;
        }
        let controls = Controls {
            yaw: self.away,
            pitch: -25.0,
            flap: true,
            brake: false,
            boost: m.on_ground || m.in_water,
            ridden: false,
        };
        self.flight.tick(m, &controls, &WINGS);
    }
}

/// Snatching a gliding flying fish in the air (SPECS §2), its prey from the library (`creature::prey::quarry`): it
/// flies at it while it is out of the water; the library's prey eats it once it reaches it.
#[derive(Debug, Default)]
struct Hunt {
    flight: Flight,
    on: bool,
}

/// Where its quarry is, if it glides (out of the water).
fn gliding_quarry(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
    let q = creature::prey::quarry(m.id)?;
    let at = cx.host().entity(q)?.position(cx.host())?;
    let cell = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    (cx.block(cell) != "minecraft:water").then_some(at)
}

impl Behaviour for Hunt {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = carried(m.id).is_none() && gliding_quarry(m, cx).is_some();
        if on && !self.on {
            self.flight = Flight::default();
            self.flight.speed = WINGS.cruise;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        if let Some(at) = gliding_quarry(m, cx) {
            fly_to(&mut self.flight, m, at);
        }
    }
}

/// Its dive for its friend (SPECS §3): about once a minute while they fish, how far from the bobber it dives, how
/// high it starts, how long it stays under.
const DIVE_EVERY: (i32, i32) = (900, 600);
const DIVE_AWAY: f64 = 4.0;
const DIVE_HEIGHT: f64 = 8.0;
const UNDER: i32 = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum DivePhase {
    #[default]
    Out,
    Plunge,
    Under(i32),
    Back,
}

/// Fishing for its friend (SPECS §3): out over the water near their bobber, a plunge-dive with its wings folded back,
/// a moment under, then back to drop the fish at their feet. It never misses.
#[derive(Debug, Default)]
struct Dive {
    flight: Flight,
    phase: DivePhase,
    spot: [f64; 3],
    wait: i32,
    on: bool,
}

impl Behaviour for Dive {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let following = friend(m, cx).is_some_and(|(_, staying, _)| !staying);
        let bobber = around(m.id).bobber;
        let on = if self.on {
            following && (bobber.is_some() || self.phase != DivePhase::Out)
        } else {
            self.wait -= 1;
            following && bobber.is_some() && self.wait <= 0 && carried(m.id).is_none()
        };
        if on && !self.on {
            let b = bobber.unwrap_or(m.position.array());
            let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
            self.spot = [b[0] + angle.cos() * DIVE_AWAY, b[1], b[2] + angle.sin() * DIVE_AWAY];
            self.phase = DivePhase::Out;
            self.flight = Flight::default();
        }
        if !on && self.on {
            self.wait = DIVE_EVERY.0 + m.random.next_int_bound(DIVE_EVERY.1).unwrap_or(0);
            DIVING.lock().unwrap().remove(&m.id);
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        DIVING.lock().unwrap().remove(&m.id);
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let spot = self.spot;
        match self.phase {
            DivePhase::Out => {
                let above = [spot[0], spot[1] + DIVE_HEIGHT, spot[2]];
                fly_to(&mut self.flight, m, above);
                if flat(m.position.array(), spot) < 2.5 && m.position.y > spot[1] + DIVE_HEIGHT * 0.5 {
                    self.phase = DivePhase::Plunge;
                    DIVING.lock().unwrap().insert(m.id, ());
                }
            }
            DivePhase::Plunge => {
                if m.in_water {
                    creature::sounds::queue(m.id, SPLASH, 1.0, 1.0);
                    self.phase = DivePhase::Under(UNDER);
                    return;
                }
                let (dx, dz) = (spot[0] - m.position.x, spot[2] - m.position.z);
                let controls = Controls {
                    yaw: heading_of(dx, dz),
                    pitch: 75.0,
                    flap: false,
                    brake: false,
                    boost: false,
                    ridden: false,
                };
                self.flight.tick(m, &controls, &WINGS);
                m.velocity.y = m.velocity.y.min(-0.6);
            }
            DivePhase::Under(left) => {
                m.no_gravity = true;
                m.velocity = Vec3 {
                    x: 0.0,
                    y: if left > UNDER / 2 { -0.05 } else { 0.08 },
                    z: 0.0,
                };
                self.phase = if left > 0 {
                    DivePhase::Under(left - 1)
                } else {
                    DIVING.lock().unwrap().remove(&m.id);
                    self.flight = Flight::default();
                    DivePhase::Back
                };
            }
            DivePhase::Back => {
                let Some((_, _, at)) = friend(m, cx) else {
                    return;
                };
                if flat(m.position.array(), at) < 2.5 {
                    queue(Action::GiveFish { gull: m.id, at });
                    self.phase = DivePhase::Out;
                    self.on = false;
                    self.wait = DIVE_EVERY.0 + m.random.next_int_bound(DIVE_EVERY.1).unwrap_or(0);
                    return;
                }
                fly_to(&mut self.flight, m, [at[0], at[1] + 3.0, at[2]]);
            }
        }
    }
}

/// A wild one behind a boat (SPECS §2): how long it follows (ticks), its rest after, how far behind and above.
const BOAT_FOR: (i32, i32) = (1200, 1200);
const BOAT_REST: i32 = 3600;
const BEHIND: f64 = 4.0;
const OVER: f64 = 4.0;

#[derive(Debug, Default)]
struct FollowBoat {
    flight: Flight,
    ticks: i32,
    rest: i32,
    on: bool,
}

impl Behaviour for FollowBoat {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        self.rest = (self.rest - 1).max(0);
        let boater = around(m.id).boater;
        let on = wild(m) && carried(m.id).is_none() && boater.is_some();
        let on = if self.on {
            on && self.ticks > 0
        } else {
            on && self.rest == 0 && chance(m, 200)
        };
        if on && !self.on {
            self.flight = Flight::default();
            self.ticks = BOAT_FOR.0 + m.random.next_int_bound(BOAT_FOR.1).unwrap_or(0);
            if let Some((_, player, _)) = boater {
                queue(Action::Learn {
                    player,
                    fact: crate::BOAT_FACT,
                });
            }
        }
        if !on && self.on {
            self.rest = BOAT_REST;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        self.ticks -= 1;
        let Some((id, _, at)) = around(m.id).boater else {
            return;
        };
        // Behind them, along the way they go (their look, as they steer by it).
        let look = cx
            .player
            .as_ref()
            .filter(|p| p.entity_id == id)
            .map_or([0.0, 0.0, 1.0], |p| p.look);
        let n = look[0].hypot(look[2]).max(0.01);
        let behind = [at[0] - look[0] / n * BEHIND, at[1] + OVER, at[2] - look[2] / n * BEHIND];
        if flat(m.position.array(), behind) < 3.0 {
            let yaw = heading_of(look[0], look[2]);
            self.flight.hover(m, behind, yaw, &WINGS);
        } else {
            fly_to(&mut self.flight, m, behind);
        }
        if chance(m, 160) {
            creature::sounds::queue(m.id, CRY, 1.0, 1.0);
        }
    }
}

/// Its lucky spot (SPECS §3): how far from its friend, how high it circles over it and how wide, how long it keeps
/// one (ticks).
const SPOT_RANGE: f64 = 12.0;
const SPOT_HEIGHT: f64 = 6.0;
const SPOT_RADIUS: f64 = 3.0;
const SPOT_FOR: (i32, i32) = (3600, 2400);

/// Circling over a lucky spot of water while its friend fishes or sits in a boat (SPECS §3), calling now and then:
/// fishing there is luckier (`crate::lucky`).
#[derive(Debug, Default)]
struct LuckySpot {
    flight: Flight,
    spot: Option<[f64; 3]>,
    ticks: i32,
    angle: f64,
}

impl LuckySpot {
    /// A spot of open water within `SPOT_RANGE` of `at`: the top of the water under a random place there.
    fn pick(m: &mut Body, cx: &World<'_>, at: [f64; 3]) -> Option<[f64; 3]> {
        for _ in 0..8 {
            let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
            let r = 4.0 + f64::from(m.random.next_float()) * (SPOT_RANGE - 4.0);
            let (x, z) = (
                (at[0] + angle.cos() * r).floor() as i32,
                (at[2] + angle.sin() * r).floor() as i32,
            );
            let top = (at[1].floor() as i32 - 6..at[1].floor() as i32 + 4)
                .rev()
                .find(|&y| cx.block([x, y, z]) != "minecraft:air")?;
            if cx.block([x, top, z]) == "minecraft:water" {
                return Some([f64::from(x) + 0.5, f64::from(top) + 1.0, f64::from(z) + 0.5]);
            }
        }
        None
    }
}

impl Behaviour for LuckySpot {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let a = around(m.id);
        let Some((player, staying, at)) = friend(m, cx) else {
            LUCKY.lock().unwrap().remove(&m.id);
            return false;
        };
        let on = !staying && (a.bobber.is_some() || a.friend_boating);
        if !on {
            LUCKY.lock().unwrap().remove(&m.id);
            self.spot = None;
            return false;
        }
        self.ticks -= 1;
        if self.spot.is_none() || self.ticks <= 0 || self.spot.is_some_and(|s| flat(s, at) > SPOT_RANGE * 1.5) {
            self.spot = Self::pick(m, cx, at);
            self.ticks = SPOT_FOR.0 + m.random.next_int_bound(SPOT_FOR.1).unwrap_or(0);
            if self.spot.is_some() {
                self.flight = Flight::default();
            }
        }
        match self.spot {
            Some(s) => {
                LUCKY.lock().unwrap().insert(m.id, (s, player));
                true
            }
            None => false,
        }
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        let Some(s) = self.spot else {
            return;
        };
        self.angle += 0.05;
        let at = [
            s[0] + self.angle.cos() * SPOT_RADIUS,
            s[1] + SPOT_HEIGHT,
            s[2] + self.angle.sin() * SPOT_RADIUS,
        ];
        fly_to(&mut self.flight, m, at);
        if chance(m, 200) {
            creature::sounds::queue(m.id, CRY, 1.0, 1.1);
        }
    }
}

/// Following its friend (SPECS §3): it flies to them past `FOLLOW` blocks, lands within `LAND`, comes to them past
/// `TELEPORT`.
const FOLLOW: f64 = 8.0;
const LAND: f64 = 4.0;
const TELEPORT: f64 = 32.0;

#[derive(Debug, Default)]
struct FollowFriend {
    flight: Flight,
    landing: bool,
    on: bool,
}

impl Behaviour for FollowFriend {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let at = friend(m, cx).filter(|(_, staying, _)| !staying).map(|(_, _, at)| at);
        let on = match at {
            Some(_) if self.on => !(self.landing && (m.on_ground || m.in_water)),
            Some(at) => flat(m.position.array(), at) > FOLLOW,
            None => false,
        };
        if on && !self.on {
            self.flight = Flight::default();
            self.landing = false;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some((_, _, at)) = friend(m, cx) else {
            return;
        };
        let d = flat(m.position.array(), at);
        if d > TELEPORT {
            m.position = Vec3 {
                x: at[0] + 2.0,
                y: at[1] + 2.0,
                z: at[2] + 2.0,
            };
            self.flight.speed = 0.0;
        }
        self.landing = d < LAND;
        if self.landing {
            let controls = Controls {
                yaw: heading_of(at[0] - m.position.x, at[2] - m.position.z),
                pitch: 25.0,
                flap: false,
                brake: true,
                boost: false,
                ridden: false,
            };
            self.flight.tick(m, &controls, &WINGS);
        } else {
            fly_to(&mut self.flight, m, [at[0], at[1] + 4.0, at[2]]);
        }
    }
}

/// Told to stay: it stays where it is.
#[derive(Debug)]
struct Stay;

impl Behaviour for Stay {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        creature::friend::of(m.id).is_some_and(|f| f.staying)
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
        if m.in_water {
            m.no_gravity = true;
            m.velocity.y = 0.0;
        }
    }
}

/// Food on the ground (SPECS §2, §3): it hovers down to it and takes it. A raw fish thrown by the player near it may
/// befriend it (`crate::act`).
#[derive(Debug, Default)]
struct Eat {
    flight: Flight,
    ticks: i32,
    on: bool,
}

impl Behaviour for Eat {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        let on = carried(m.id).is_none() && around(m.id).food.is_some() && self.ticks < 300;
        if on && !self.on {
            self.flight = Flight::default();
            self.ticks = 0;
        }
        if !on && !self.on {
            self.ticks = 0;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        self.ticks += 1;
        let Some((item, at, fish, thrower)) = around(m.id).food else {
            return;
        };
        if creature::body::distance(m.position.array(), at) < 1.3 {
            let player = cx.player.as_ref().filter(|p| thrower == Some(p.entity_id));
            let befriends = player.filter(|_| fish && wild(m)).map(|p| p.uuid);
            queue(Action::Eat {
                gull: m.id,
                item,
                befriends,
            });
            AROUND.lock().unwrap().entry(m.id).or_default().food = None;
            self.ticks = 0;
            return;
        }
        if flat(m.position.array(), at) < 4.0 {
            let yaw = heading_of(at[0] - m.position.x, at[2] - m.position.z);
            self.flight.hover(m, [at[0], at[1] + 0.3, at[2]], yaw, &WINGS);
        } else {
            fly_to(&mut self.flight, m, [at[0], at[1] + 2.0, at[2]]);
        }
    }
}

/// How deep it sits in the water (blocks of its feet under the surface), and how it drifts.
const FLOAT_DEPTH: f64 = 0.15;
const DRIFT: f64 = 0.01;

/// Bobbing on the waves (SPECS §2): it floats at the surface, drifting slowly, turning now and then.
#[derive(Debug, Default)]
struct Float {
    drift: [f64; 2],
    wait: i32,
}

impl Behaviour for Float {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        m.in_water
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.no_gravity = true;
        let (x, z) = (m.position.x.floor() as i32, m.position.z.floor() as i32);
        let y = m.position.y.floor() as i32;
        let top = (y..y + 4)
            .take_while(|&y| cx.block([x, y, z]) == "minecraft:water")
            .last()
            .unwrap_or(y);
        let surface = f64::from(top) + 0.9;
        let bob = (f64::from(m.age) * 0.1).sin() * 0.03;
        self.wait -= 1;
        if self.wait <= 0 {
            self.wait = 60 + m.random.next_int_bound(120).unwrap_or(0);
            let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
            self.drift = [angle.cos() * DRIFT, angle.sin() * DRIFT];
            m.yaw = heading_of(self.drift[0], self.drift[1]);
            m.body_yaw = m.yaw;
        }
        m.velocity = Vec3 {
            x: self.drift[0],
            y: ((surface - FLOAT_DEPTH + bob - m.position.y) * 0.3).clamp(-0.1, 0.1),
            z: self.drift[1],
        };
    }
}

/// On the ground: it walks about now and then.
#[derive(Debug, Default)]
struct Walk {
    wait: i32,
}

impl Behaviour for Walk {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        m.on_ground
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.no_gravity = false;
        self.wait -= 1;
        if self.wait > 0 {
            return;
        }
        self.wait = 100 + m.random.next_int_bound(160).unwrap_or(0);
        let angle = f64::from(m.random.next_float()) * std::f64::consts::TAU;
        let r = 2.0 + f64::from(m.random.next_float()) * 4.0;
        let to = [
            m.position.x + angle.cos() * r,
            m.position.y,
            m.position.z + angle.sin() * r,
        ];
        m.walk_to = Some((to, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_are_taken_once() {
        queue(Action::Swallow { gull: 3 });
        assert_eq!(actions(), vec![Action::Swallow { gull: 3 }]);
        assert!(actions().is_empty());
    }

    #[test]
    fn lucky_spots_are_their_friend_s() {
        LUCKY.lock().unwrap().insert(5, ([1.0, 2.0, 3.0], 77));
        assert_eq!(lucky_spots(77), vec![[1.0, 2.0, 3.0]]);
        assert!(lucky_spots(78).is_empty());
        forget(5);
        assert!(lucky_spots(77).is_empty());
    }
}
