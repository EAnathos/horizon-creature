//! How a storm spirit flies (SPECS §2), by hand (the library's `body`): always in the air, 1 to 4 blocks above the
//! ground or the water, hovering and flickering, then zipping a few blocks in a quick dash (`Dash`). Hit, it dashes at
//! whoever hit it and shocks them, then keeps away and comes back to shock them again every few seconds, and calls
//! lightning down on them every few seconds too (sparks over their head first), until it loses them (`Shock`, its
//! target the library's `anger`).
use creature::body::{Behaviour, Body, Vec3, World};

use crate::{call_bolt, shock};

/// A dash's speed (blocks a tick), its length (blocks) and the wait between two (ticks).
const DASH_SPEED: f64 = 0.6;
const DASH_LENGTH: (f64, f64) = (2.0, 6.0);
const DASH_WAIT: (i32, i32) = (20, 60);
/// How high it hovers over the ground or the water (blocks).
const HEIGHT: (f64, f64) = (1.0, 4.0);
/// How far down it looks for the ground (blocks).
const DEPTH: i32 = 8;

/// Fighting: within `REACH` of its target it shocks it, every `EVERY` ticks (and up to `EVERY_MORE` more); in between
/// it keeps `KEEP` blocks away. It lets a target go out of `GIVE_UP` blocks.
const REACH: f64 = 1.6;
const EVERY: i32 = 60;
const EVERY_MORE: i32 = 30;
const KEEP: f64 = 5.0;
const GIVE_UP: f64 = 20.0;
const CHARGE_SPEED: f64 = 0.5;
const AWAY_SPEED: f64 = 0.25;
/// While it fights, it calls a lightning bolt down on its target every `BOLT_EVERY` ticks (and up to `BOLT_MORE`
/// more), the first `FIRST_BOLT` ticks after it was hit.
const BOLT_EVERY: i32 = 100;
const BOLT_MORE: i32 = 40;
const FIRST_BOLT: i32 = 40;

/// The buzz of a dash.
const BUZZ: &str = "storm_spirits:entity.storm_spirit.buzz";

/// Whether the block `id` lets it through.
fn open(id: &str) -> bool {
    matches!(
        id,
        "minecraft:air"
            | "minecraft:cave_air"
            | "minecraft:void_air"
            | "minecraft:short_grass"
            | "minecraft:tall_grass"
            | "minecraft:fern"
            | "minecraft:large_fern"
            | "minecraft:snow"
    ) || id.ends_with("_flower")
}

/// The height of the ground or the water under `at` (its top), or `DEPTH` blocks down with none.
fn surface(cx: &World<'_>, at: Vec3) -> f64 {
    let (x, y, z) = (at.x.floor() as i32, at.y.floor() as i32, at.z.floor() as i32);
    (0..=DEPTH)
        .map(|d| y - d)
        .find(|&by| !open(&cx.block([x, by, z])))
        .map_or(f64::from(y - DEPTH), |by| f64::from(by + 1))
}

/// Turns it to face along `d` (its yaw).
fn face(m: &mut Body, d: [f64; 3]) {
    if d[0].hypot(d[2]) < 1e-3 {
        return;
    }
    let yaw = (-d[0]).atan2(d[2]).to_degrees() as f32;
    m.yaw = yaw;
    m.body_yaw = yaw;
    m.head_yaw = yaw;
}

/// It flies: no gravity, no fall.
fn airborne(m: &mut Body) {
    m.no_gravity = true;
    m.fall_distance = 0.0;
}

/// Hovering at `height` over the ground: slowing down, rising or sinking to it, bobbing a little.
fn hover(m: &mut Body, cx: &World<'_>, height: f64) {
    let ground = surface(cx, m.position);
    let bob = 0.02 * (f64::from(m.age) * 0.3).sin();
    let rise = ((ground + height - m.position.y) * 0.1).clamp(-0.12, 0.12) + bob;
    m.velocity = Vec3 {
        x: m.velocity.x * 0.6,
        y: rise,
        z: m.velocity.z * 0.6,
    };
}

fn between(m: &mut Body, (low, high): (f64, f64)) -> f64 {
    low + f64::from(m.random.next_float()) * (high - low)
}

/// Its own flight: hovering, then a dash.
#[derive(Debug, Default)]
pub(crate) struct Dash {
    wait: i32,
    /// Ticks of the dash it is in, and its way (a unit vector).
    dash: i32,
    heading: [f64; 3],
    height: f64,
}

impl Behaviour for Dash {
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        airborne(m);
        if self.height == 0.0 {
            self.height = between(m, HEIGHT);
        }
        if self.dash > 0 {
            self.dash -= 1;
            let h = self.heading;
            m.velocity = Vec3::of(h.map(|k| k * DASH_SPEED));
            face(m, h);
            return;
        }
        hover(m, cx, self.height);
        self.wait -= 1;
        if self.wait > 0 {
            return;
        }
        self.wait = DASH_WAIT.0 + m.random.next_int_bound(DASH_WAIT.1 - DASH_WAIT.0).unwrap_or(0);
        self.height = between(m, HEIGHT);
        let angle = between(m, (0.0, std::f64::consts::TAU));
        let length = between(m, DASH_LENGTH);
        let rise = (surface(cx, m.position) + self.height - m.position.y).clamp(-2.0, 2.0);
        let d = [angle.cos() * length, rise, angle.sin() * length];
        let to = [m.position.x + d[0], m.position.y + d[1] + 0.5, m.position.z + d[2]];
        if !open(&cx.block(to.map(|k| k.floor() as i32))) {
            return;
        }
        let norm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        self.heading = d.map(|k| k / norm);
        self.dash = (norm / DASH_SPEED).ceil() as i32;
        let pitch = 1.4 + m.random.next_float() * 0.4;
        creature::sounds::queue(m.id, BUZZ, 0.5, pitch);
    }
}

/// Fighting back: a dash at its target and a shock, then keeping away until the next.
#[derive(Debug, Default)]
pub(crate) struct Shock {
    cooldown: i32,
    /// Ticks until its next bolt; 0 before its first.
    bolt: i32,
}

impl Behaviour for Shock {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        creature::anger::pursued(m, cx, GIVE_UP).is_some()
    }
    fn stop(&mut self, _: &mut Body, _: &World<'_>) {
        self.bolt = 0;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some((who, t)) = creature::anger::pursued(m, cx, GIVE_UP) else {
            return;
        };
        airborne(m);
        // At the middle of its target's body.
        let d = [t[0] - m.position.x, t[1] + 1.0 - m.position.y, t[2] - m.position.z];
        let reach = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(1e-3);
        let toward = d.map(|k| k / reach);
        face(m, toward);
        if self.bolt == 0 {
            self.bolt = FIRST_BOLT;
        }
        self.bolt -= 1;
        if self.bolt <= 0 {
            call_bolt(m.id, who);
            self.bolt = BOLT_EVERY + m.random.next_int_bound(BOLT_MORE).unwrap_or(0);
        }
        self.cooldown -= 1;
        if self.cooldown <= 0 {
            if reach < REACH {
                shock(m.id, who);
                self.cooldown = EVERY + m.random.next_int_bound(EVERY_MORE).unwrap_or(0);
                m.velocity = Vec3::of(toward.map(|k| -k * AWAY_SPEED));
            } else {
                m.velocity = Vec3::of(toward.map(|k| k * CHARGE_SPEED.min(reach)));
            }
            return;
        }
        if reach < KEEP {
            let ground = surface(cx, m.position);
            let rise = if m.position.y < ground + HEIGHT.0 { 0.1 } else { 0.0 };
            m.velocity = Vec3 {
                x: -toward[0] * AWAY_SPEED,
                y: rise,
                z: -toward[2] * AWAY_SPEED,
            };
        } else {
            hover(m, cx, 2.0);
        }
    }
}
