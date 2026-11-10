//! The wild vulture (SPECS §4): soaring in slow circles high up, then gliding back down.
//!
//! `Soar` flies with the same physics as the ridden flight (`flight::Flight`), steered by its own controls: it takes
//! off now and then, climbs to 18 to 26 blocks above where it left the ground, circles there for 30 to 60 s, then
//! picks the highest point within 16 blocks (a peak, a tree top, a pillar, `landing_spot`) and glides down onto it. A vulture left in the air (its rider got off) glides down the same way. Fighting
//! (`DiveAttack`, below, its target from `HurtByTarget`) and riding come first.
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

use crate::flight::{CRUISE, Controls, Flight, MAX_CLIMB};

/// One take-off every 30 s on average, from the ground.
const TAKE_OFF_CHANCE: i32 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Climb,
    Circle,
    Descend,
}

#[derive(Clone, Debug)]
pub struct Soar {
    flight: Flight,
    phase: Phase,
    /// The circle's centre (x, z), radius and height.
    center: [f64; 2],
    radius: f64,
    cruise_y: f64,
    clockwise: bool,
    /// Ticks spent in the phase.
    ticks: i32,
    /// Ticks to circle before going down.
    circle_for: i32,
    /// Where it lands: the highest point near it when it starts down.
    landing: Option<[f64; 3]>,
}

/// Ticks of descent before it gives up its landing spot, and before it stops flying at all.
const ABANDON_SPOT: i32 = 400;
const GIVE_UP: i32 = 1200;

/// How far around it looks for a high point to land on, and in steps of how many blocks.
const LANDING_SEARCH: i32 = 16;
const LANDING_STEP: i32 = 2;

/// The highest ground within `LANDING_SEARCH` blocks (`Heightmap.Types.WORLD_SURFACE`), where it lands; None in
/// unloaded ground.
fn landing_spot(m: &Mob, cx: &Cx) -> Option<[f64; 3]> {
    let world = cx.w.world;
    let (cx0, cz0) = (m.position.x.floor() as i32, m.position.z.floor() as i32);
    let mut best: Option<(i32, i32, i32)> = None;
    for dx in (-LANDING_SEARCH..=LANDING_SEARCH).step_by(LANDING_STEP as usize) {
        for dz in (-LANDING_SEARCH..=LANDING_SEARCH).step_by(LANDING_STEP as usize) {
            let (x, z) = (cx0 + dx, cz0 + dz);
            let top = world.world_surface(x, z, |s| world.is_air(s));
            // Not in a tree top (it gets stuck in the leaves), nor on water or lava.
            let name =
                cx.w.registry
                    .state(world.block(x, top - 1, z))
                    .map_or("", |s| s.name.as_str());
            let perch =
                !cx.w.has_tag("minecraft:leaves", name) && !matches!(name, "minecraft:water" | "minecraft:lava");
            if perch && top > world.layout().min_y() && best.is_none_or(|b| top > b.1) {
                best = Some((x, top, z));
            }
        }
    }
    best.map(|(x, y, z)| [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5])
}

impl Soar {
    pub fn new() -> Self {
        Self {
            flight: Flight::default(),
            phase: Phase::Descend,
            center: [0.0; 2],
            radius: 16.0,
            cruise_y: 0.0,
            clockwise: true,
            ticks: 0,
            circle_for: 0,
            landing: None,
        }
    }

    /// The heading that follows the circle, steering back towards its radius.
    fn circle_yaw(&self, m: &Mob) -> f32 {
        let (dx, dz) = (m.position.x - self.center[0], m.position.z - self.center[1]);
        let dist = (dx * dx + dz * dz).sqrt().max(0.01);
        let (rx, rz) = (dx / dist, dz / dist);
        let (tx, tz) = if self.clockwise { (-rz, rx) } else { (rz, -rx) };
        // Inwards when too far, outwards when too close.
        let pull = ((dist - self.radius) / self.radius).clamp(-1.0, 1.0) * 0.8;
        heading_of(tx - rx * pull, tz - rz * pull)
    }
}

/// The yaw of a horizontal direction (`velocity.x = -sin(yaw)`, `velocity.z = cos(yaw)`).
fn heading_of(x: f64, z: f64) -> f32 {
    (-x).atan2(z).to_degrees() as f32
}

/// Only a wild vulture soars on its own; a tamed one follows its owner (`tame.rs`).
fn idle(m: &Mob) -> bool {
    !m.extra.tame && m.rider.is_none() && m.ai.target.is_none() && m.extra.leash.is_none()
}

impl Goal for Soar {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        idle(m) && (!m.on_ground || m.random.next_int_bound(TAKE_OFF_CHANCE).unwrap_or(1) == 0)
    }
    fn can_continue(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        // Down, or a minute spent trying: it lets itself drop (no fall damage).
        idle(m) && !(self.phase == Phase::Descend && (m.on_ground && self.ticks > 0 || self.ticks > GIVE_UP))
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, cx: &mut Cx) {
        self.flight = Flight::default();
        self.ticks = 0;
        self.landing = None;
        self.center = [m.position.x, m.position.z];
        self.radius = 12.0 + f64::from(m.random.next_int_bound(9).unwrap_or(4));
        self.clockwise = m.random.next_int_bound(2).unwrap_or(0) == 0;
        if m.on_ground {
            self.phase = Phase::Climb;
            self.cruise_y = m.position.y + 18.0 + f64::from(m.random.next_int_bound(9).unwrap_or(4));
            self.circle_for = 600 + m.random.next_int_bound(601).unwrap_or(300);
        } else {
            // Left in the air: glide down from where it is.
            self.phase = Phase::Descend;
            self.cruise_y = m.position.y;
            self.flight.speed = CRUISE * 0.6;
            self.landing = landing_spot(m, cx);
        }
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        self.ticks += 1;
        // A spot it can't reach in 20 s (behind a tree, a cliff): it spirals down where it is instead.
        if self.phase == Phase::Descend && self.ticks > ABANDON_SPOT {
            self.landing = None;
        }
        let yaw = self.circle_yaw(m);
        let below = self.cruise_y - m.position.y;
        let controls = match self.phase {
            Phase::Climb => {
                if below < 2.0 {
                    self.phase = Phase::Circle;
                    self.ticks = 0;
                }
                Controls {
                    yaw,
                    pitch: -MAX_CLIMB * 0.6,
                    flap: true,
                    brake: false,
                    boost: self.ticks < 10,
                    heavy: false,
                    ridden: false,
                }
            }
            Phase::Circle => {
                if self.ticks > self.circle_for {
                    self.phase = Phase::Descend;
                    self.ticks = 0;
                    self.landing = landing_spot(m, cx);
                }
                // Holds its height: noses up below it, down above it; flaps only to keep its speed or climb.
                let pitch = (-below * 3.0).clamp(-20.0, 20.0) as f32;
                Controls {
                    yaw,
                    pitch,
                    flap: below > 1.0 || self.flight.speed < CRUISE * 0.6,
                    brake: false,
                    boost: false,
                    heavy: false,
                    ridden: false,
                }
            }
            // Down onto its landing spot along a glide path (0.4 block down per block across), steeply over it; with no
            // spot found, spiralling down where it is.
            Phase::Descend => match self.landing {
                Some(at) => {
                    let (dx, dz) = (at[0] - m.position.x, at[2] - m.position.z);
                    let flat = (dx * dx + dz * dz).sqrt();
                    let path = at[1] + flat * 0.4 - m.position.y;
                    let (pitch, brake) = if flat < 3.0 {
                        (45.0, true)
                    } else {
                        ((-path * 3.0).clamp(-20.0, 35.0) as f32, self.flight.speed > CRUISE)
                    };
                    Controls {
                        yaw: heading_of(dx, dz),
                        pitch,
                        flap: path > 2.0,
                        brake,
                        boost: false,
                        heavy: false,
                        ridden: false,
                    }
                }
                None => Controls {
                    yaw,
                    pitch: 12.0,
                    flap: false,
                    brake: self.flight.speed > CRUISE,
                    boost: false,
                    heavy: false,
                    ridden: false,
                },
            },
        };
        self.flight.tick(m, &controls);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Above its target before diving, and how far from it it starts the dive, in blocks.
const DIVE_ABOVE: f64 = 6.0;
const DIVE_FROM: f64 = 10.0;
/// How close a strike lands, in blocks (from its feet to the target's eyes), and the ticks between strikes.
const STRIKE_REACH: f64 = 2.2;
const STRIKE_EVERY: i32 = 20;
/// Within this distance across it hovers over its target, its feet this high above the target's eyes (about a block
/// over its head).
const HOVER_FROM: f64 = 4.0;
const HOVER_ABOVE: f64 = 1.2;
/// After this many strikes from its hover it pulls away for `PULL_AWAY` ticks and dives again.
const STRIKES_PER_PASS: i32 = 3;
const PULL_AWAY: i32 = 30;

/// Fighting in the air (SPECS §4): with a target, it takes off, climbs above it, dives at it and hovers on beating
/// wings about a block over its head, striking with its beak (vanilla's `doHurtTarget`, its 3 damage); after a few
/// strikes it pulls away and comes back. It never lands during a fight (it used to dive to the target's eyes, touch
/// down and take off again). Replaces a ground melee attack, a vulture fights on the wing.
#[derive(Clone, Debug, Default)]
pub struct DiveAttack {
    flight: Flight,
    cooldown: i32,
    away: i32,
    strikes: i32,
}

impl Goal for DiveAttack {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.rider.is_none() && m.ai.target.and_then(|t| cx.w.target_info(t)).is_some_and(|t| t.alive)
    }
    fn start(&mut self, _: &mut Mob, _: &mut Cx) {
        self.flight = Flight::default();
        self.cooldown = 0;
        self.away = 0;
        self.strikes = 0;
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight.land(m);
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(who) = m.ai.target else {
            return;
        };
        let Some(t) = cx.w.target_info(who) else {
            return;
        };
        self.cooldown = (self.cooldown - 1).max(0);
        self.away = (self.away - 1).max(0);
        let (dx, dz) = (t.position.x - m.position.x, t.position.z - m.position.z);
        let flat = (dx * dx + dz * dz).sqrt();
        let reach = (flat * flat + (m.position.y - t.eye_y).powi(2)).sqrt();
        if reach < STRIKE_REACH && self.cooldown == 0 && self.away == 0 {
            m.swing();
            horizon_client::mobs::ai::do_hurt_target(m, who, cx);
            self.cooldown = STRIKE_EVERY;
            self.strikes += 1;
            if self.strikes >= STRIKES_PER_PASS {
                self.strikes = 0;
                self.away = PULL_AWAY;
            }
        }
        let to_target = heading_of(dx, dz);
        // Where it hovers: a block over the target's head.
        let hover_y = t.eye_y + HOVER_ABOVE;
        if self.away == 0 && flat < HOVER_FROM {
            self.flight.hover(m, [t.position.x, hover_y, t.position.z], to_target);
            return;
        }
        let dy = hover_y - m.position.y;
        let controls = if self.away > 0 {
            // Pulling away: up and past.
            Controls {
                yaw: m.yaw,
                pitch: -30.0,
                flap: true,
                brake: false,
                boost: false,
                heavy: false,
                ridden: false,
            }
        } else if flat > DIVE_FROM {
            // Closing in, above it.
            let below = t.position.y + DIVE_ABOVE - m.position.y;
            Controls {
                yaw: to_target,
                pitch: (-below * 3.0).clamp(-35.0, 35.0) as f32,
                flap: true,
                brake: false,
                boost: m.on_ground,
                heavy: false,
                ridden: false,
            }
        } else {
            // The dive, at the spot over its head (never below it: it would touch down).
            let pitch = (-dy).atan2(flat).to_degrees() as f32;
            let pitch = if dy > 0.0 { pitch.min(-10.0) } else { pitch };
            Controls {
                yaw: to_target,
                pitch,
                flap: false,
                brake: false,
                boost: m.on_ground,
                heavy: false,
                ridden: false,
            }
        };
        self.flight.tick(m, &controls);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
