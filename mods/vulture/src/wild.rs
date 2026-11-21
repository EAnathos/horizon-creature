//! The wild vulture (SPECS §4): soaring in slow circles high up, then gliding back down (the library's
//! `creature::flight::Soar`, as `SOARING` says), and fighting in the air (`DiveAttack`, below, its target from
//! vanilla's `HurtByTarget`, added at join). Fighting and riding come first. Both run from the server tick (the
//! library's `body`).
use creature::body::{Behaviour, Body, World};
use creature::flight::{Controls, Flight, Soaring, heading_of};

use crate::WINGS;

/// How a wild vulture soars: one take-off every 30 s on average from the ground, a climb to 18 to 26 blocks above
/// where it left the ground, a circle of 12 to 20 blocks for 30 to 60 s, then down onto a high perch: not a tree top
/// (it gets stuck in the leaves), nor water or lava.
pub static SOARING: Soaring = Soaring {
    take_off: 600,
    height: (18.0, 8),
    radius: (12.0, 8),
    circle_for: (600, 600),
    idle,
    perch: |name| !name.ends_with("leaves") && !matches!(name, "minecraft:water" | "minecraft:lava"),
    lands_on_water: false,
};

/// Only a wild vulture soars on its own; a tamed one follows its owner (`tame.rs`).
fn idle(m: &Body, cx: &World<'_>) -> bool {
    crate::state::saved(m.id).owner.is_none()
        && crate::ride::of(m.id).is_none()
        && crate::target(cx, m.id).is_none()
        && !crate::state::leashed(m.id)
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
#[derive(Debug, Default)]
pub struct DiveAttack {
    flight: Flight,
    cooldown: i32,
    away: i32,
    strikes: i32,
    on: bool,
}

impl Behaviour for DiveAttack {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = crate::ride::of(m.id).is_none() && crate::target(cx, m.id).is_some();
        if on && !self.on {
            self.flight = Flight::default();
            self.cooldown = 0;
            self.away = 0;
            self.strikes = 0;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(t) = crate::target(cx, m.id) else {
            return;
        };
        self.cooldown = (self.cooldown - 1).max(0);
        self.away = (self.away - 1).max(0);
        let (dx, dz) = (t.at[0] - m.position.x, t.at[2] - m.position.z);
        let flat = (dx * dx + dz * dz).sqrt();
        let reach = (flat * flat + (m.position.y - t.eye_y).powi(2)).sqrt();
        if reach < STRIKE_REACH && self.cooldown == 0 && self.away == 0 {
            crate::strike(m.id, t.id);
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
            self.flight.hover(m, [t.at[0], hover_y, t.at[2]], to_target, &WINGS);
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
                ridden: false,
            }
        } else if flat > DIVE_FROM {
            // Closing in, above it.
            let below = t.at[1] + DIVE_ABOVE - m.position.y;
            Controls {
                yaw: to_target,
                pitch: (-below * 3.0).clamp(-35.0, 35.0) as f32,
                flap: true,
                brake: false,
                boost: m.on_ground,
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
                ridden: false,
            }
        };
        self.flight.tick(m, &controls, &WINGS);
    }
}
