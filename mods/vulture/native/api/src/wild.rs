//! The wild vulture (SPECS §4): soaring in slow circles high up, then gliding back down.
//!
//! `Soar` flies with the same physics as the ridden flight (`flight::Flight`), steered by its own controls: it takes
//! off now and then, climbs to 18 to 26 blocks above where it left the ground, circles there for 30 to 60 s and
//! spirals down until it lands. A vulture left in the air (its rider got off) glides down the same way. Fighting
//! (`HurtByTarget` and `MeleeAttack`, set in `lib.rs`) and riding come first.
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
    !m.extra.tame && m.rider.is_none() && m.ai.target.is_none()
}

impl Goal for Soar {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        idle(m) && (!m.on_ground || m.random.next_int_bound(TAKE_OFF_CHANCE).unwrap_or(1) == 0)
    }
    fn can_continue(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        idle(m) && !(m.on_ground && self.phase == Phase::Descend && self.ticks > 0)
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight = Flight::default();
        self.ticks = 0;
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
        }
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        self.ticks += 1;
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
                }
            }
            Phase::Circle => {
                if self.ticks > self.circle_for {
                    self.phase = Phase::Descend;
                    self.ticks = 0;
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
                }
            }
            Phase::Descend => Controls {
                yaw,
                pitch: 12.0,
                flap: false,
                brake: self.flight.speed > CRUISE,
                boost: false,
                heavy: false,
            },
        };
        self.flight.tick(m, &controls);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
