//! Flight (SPECS §6.1): the flight of the series' big fliers, the vulture's and the seagull's. Moved from the vulture.
//!
//! - `Flight`: the physics, flown like a plane. It always moves along its heading, turns towards the wanted heading
//!   (banking, drawn by the mod's pose from the gap between its body and its head, which looks where it wants to go),
//!   noses up or down towards the wanted pitch, and trades speed for height like a glider. Steered each tick by
//!   `Controls` (a rider's keys, or the creature's own behaviour); a flier's speeds and sounds are its `Wings`.
//! - `Soar`: soaring on its own, as `Soaring` says: taking off now and then, climbing, circling, then gliding down onto
//!   the highest perch near it.
//!
//! Everything runs from the server tick (the library's `body`), which writes back the velocity, heading and pitch set.
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::body::{Behaviour, Body, World};
use crate::sounds;

/// The slowest it flies before it stalls, in blocks per tick.
pub const STALL: f64 = 0.18;
/// What a nose-down tick adds to the speed (× the sine of the pitch), a nose-up one takes away.
const GRAVITY: f64 = 0.03;
/// Wing beats close this share of the gap to the cruising speed every tick.
const FLAP: f64 = 0.06;
const GLIDE_DRAG: f64 = 0.004;
const BRAKE: f64 = 0.04;
const GLIDE_SINK: f64 = 0.04;
const TAKE_OFF: f64 = 0.4;
const MIN_DESCENT: f64 = 0.2;
/// From this pitch down a flight counts as a descent (smaller ones are a wild vulture holding its height).
const DESCENT_PITCH: f32 = 10.0;
/// The steepest climb and dive it takes, and how fast its pitch follows the wanted one, in degrees.
pub const MAX_CLIMB: f32 = 50.0;
const MAX_DIVE: f32 = 75.0;
const PITCH_RATE: f32 = 4.0;
/// Its turn: this share of the gap between its heading and the wanted one, at most `MAX_TURN` a tick.
const TURN: f32 = 0.08;
const MAX_TURN: f32 = 5.0;

/// What steers the flight for one tick.
pub struct Controls {
    /// Where it should head and how steeply (positive nose down), in degrees.
    pub yaw: f32,
    pub pitch: f32,
    /// Wing beats up to the cruising speed (W).
    pub flap: bool,
    /// Air brake (S).
    pub brake: bool,
    /// A strong flap that climbs, and takes off from the ground (Space).
    pub boost: bool,
    /// Ridden (the vulture): twice its own cruising and diving speeds, to travel far.
    pub ridden: bool,
}

/// What a rider's flight multiplies its cruising and diving speeds by.
const RIDDEN_SPEED: f64 = 2.0;

/// What differs between fliers: their speeds (blocks per tick, before air drag, ×0.91 a tick), their sounds, and
/// what multiplies their speeds for one of them (the vulture's grow with its kind and age; `|_| 1.0` for none).
pub struct Wings {
    pub cruise: f64,
    pub dive: f64,
    pub flap: &'static str,
    pub takeoff: &'static str,
    pub pace: fn(u32) -> f64,
}

/// Hovering: the share of the distance to its spot it wants to cover a tick, its fastest, and how quickly its velocity
/// follows.
const HOVER_PULL: f64 = 0.2;
const HOVER_SPEED: f64 = 0.3;
const HOVER_EASE: f64 = 0.25;

/// A flier's momentum.
#[derive(Clone, Debug, Default)]
pub struct Flight {
    /// Speed along the heading, blocks per tick.
    pub speed: f64,
    /// Its pitch, positive nose down (the mob's pitch, drawn by `pose`).
    pub pitch: f32,
    /// The speed across it set last tick, to tell a wall that stopped it.
    pub set_across: f64,
}

impl Flight {
    /// Back on the ground: no momentum, level.
    pub fn land(&mut self, m: &mut Body) {
        self.speed = 0.0;
        self.pitch = 0.0;
        m.pitch = 0.0;
        m.no_gravity = false;
        set_flapping(m.id, false);
    }

    /// Hovering on beating wings towards `at` (its feet's place), facing `yaw`: no momentum like a plane's, it eases its
    /// velocity towards the spot, at most `HOVER_SPEED` a tick. For a strike from just above its target.
    pub fn hover(&mut self, m: &mut Body, at: [f64; 3], yaw: f32, wings: &Wings) {
        m.no_gravity = true;
        let want = [at[0] - m.position.x, at[1] - m.position.y, at[2] - m.position.z];
        let len = (want[0] * want[0] + want[1] * want[1] + want[2] * want[2]).sqrt();
        let scale = if len > 0.0 {
            (len * HOVER_PULL).min(HOVER_SPEED) / len
        } else {
            0.0
        };
        m.velocity.x += (want[0] * scale - m.velocity.x) * HOVER_EASE;
        m.velocity.y += (want[1] * scale - m.velocity.y) * HOVER_EASE;
        m.velocity.z += (want[2] * scale - m.velocity.z) * HOVER_EASE;
        self.speed = m.velocity.x.hypot(m.velocity.z);
        // Body up, wings beating: the pitch eases back to level.
        self.pitch += (0.0 - self.pitch).clamp(-PITCH_RATE, PITCH_RATE);
        m.pitch = self.pitch;
        let gap = wrap(yaw - m.yaw);
        m.yaw += gap.clamp(-MAX_TURN * 2.0, MAX_TURN * 2.0);
        m.body_yaw = m.yaw;
        m.head_yaw = yaw;
        if m.age % 6 == 0 {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, wings.flap, 0.6, pitch);
        }
        set_flapping(m.id, true);
    }

    /// One tick in the air (or taking off): sets its heading, pitch and velocity.
    pub fn tick(&mut self, m: &mut Body, c: &Controls, wings: &Wings) {
        m.no_gravity = true;
        if m.on_ground {
            self.speed = self.speed.max(STALL);
            self.pitch = -MAX_CLIMB / 2.0;
            sounds::queue(m.id, wings.takeoff, 0.8, 1.0);
        }
        // Turn: a share of the gap, so it banks more and turns faster the further it has to go. The head looks
        // where it is going, which `pose` reads as the bank.
        let gap = wrap(c.yaw - m.yaw);
        let heading = m.yaw + (gap * TURN).clamp(-MAX_TURN, MAX_TURN);
        m.yaw = heading;
        m.body_yaw = heading;
        m.head_yaw = c.yaw;
        // Pitch: follows the wanted one, smoothly; a stall noses it down whatever is asked.
        let mut target = c.pitch.clamp(-MAX_CLIMB, MAX_DIVE);
        if self.speed < STALL && target < 0.0 {
            target = 20.0;
        }
        self.pitch += (target - self.pitch).clamp(-PITCH_RATE, PITCH_RATE);
        m.pitch = self.pitch;
        // Speed: gravity along the slope, then the wings.
        let (sin_pitch, cos_pitch) = f64::from(self.pitch).to_radians().sin_cos();
        self.speed += GRAVITY * sin_pitch;
        let pace = (wings.pace)(m.id) * if c.ridden { RIDDEN_SPEED } else { 1.0 };
        let cruise = wings.cruise * pace;
        if c.flap && self.speed < cruise {
            self.speed += (cruise - self.speed) * FLAP;
        } else if c.brake {
            self.speed -= BRAKE;
        } else {
            self.speed -= GLIDE_DRAG;
        }
        self.speed = self.speed.clamp(0.0, wings.dive * pace);
        let (s, c_yaw) = f64::from(heading).to_radians().sin_cos();
        let level = self.speed * cos_pitch;
        self.set_across = level;
        m.velocity.x = -s * level;
        m.velocity.z = c_yaw * level;
        m.velocity.y = -self.speed * sin_pitch;
        let flapping = c.flap || c.boost;
        // Bumping into something in the air (a tree, a cliff): it climbs over it rather than staying stuck. A wall is
        // the speed across it lost since last tick (the engine stopped it).
        let across = m.velocity.x.hypot(m.velocity.z);
        let blocked = !m.on_ground && self.set_across > 0.15 && across < self.set_across * 0.3;
        // Descending (pitch down), it sinks at least this fast even slow, so it comes down to land.
        if c.pitch >= DESCENT_PITCH {
            m.velocity.y = m.velocity.y.min(-MIN_DESCENT);
        }
        if c.boost || blocked {
            m.velocity.y = m.velocity.y.max(TAKE_OFF);
        } else if !flapping {
            // Gliding sinks gently, less the faster it goes.
            m.velocity.y -= GLIDE_SINK * (1.0 - (self.speed / cruise).min(1.0)) + GLIDE_SINK / 2.0;
        }
        // One wing beat every 8 ticks while it flaps.
        if flapping && m.age % 8 == 0 {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, wings.flap, 0.6, pitch);
        }
        set_flapping(m.id, flapping);
    }
}

/// `a` in degrees, wrapped to -180..180.
pub fn wrap(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// Whether each flier is beating its wings, for `pose` (the render state doesn't carry it). Single player:
/// the behaviours and the drawing share the mob's id.
static FLAPPING: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

fn set_flapping(id: u32, flapping: bool) {
    let mut map = FLAPPING.lock().unwrap();
    if flapping {
        map.insert(id, true);
    } else {
        map.remove(&id);
    }
}

pub fn flapping(id: u32) -> bool {
    FLAPPING.lock().unwrap().contains_key(&id)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Climb,
    Circle,
    Descend,
}

/// How a flier soars on its own: one take-off in `take_off` ticks on average from the ground, a climb to `height`
/// blocks (the first, plus up to the second more) above where it left the ground, a circle of `radius` blocks (the
/// same), circled for `circle_for` ticks (the same); when it may (`idle`), where it lands (`perch`, a block's id:
/// the vulture's no leaves, water nor lava), and whether reaching water ends its descent (the seagull floats).
pub struct Soaring {
    pub take_off: i32,
    pub height: (f64, i32),
    pub radius: (f64, i32),
    pub circle_for: (i32, i32),
    pub idle: fn(&Body, &World<'_>) -> bool,
    pub perch: fn(&str) -> bool,
    pub lands_on_water: bool,
}

/// Soaring (`Soaring` says how): it takes off now and then, climbs, circles for a while, then picks the highest
/// point it may perch on within 16 blocks (`landing_spot`) and glides down onto it. One left in the air glides down
/// the same way.
pub struct Soar {
    how: &'static Soaring,
    wings: &'static Wings,
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
    on: bool,
}

/// Ticks of descent before it gives up its landing spot, and before it stops flying at all.
const ABANDON_SPOT: i32 = 400;
/// Ticks of climbing before it gives up reaching its height.
const CLIMB_FOR: i32 = 200;
const GIVE_UP: i32 = 1200;

/// How far around it looks for a high point to land on, and in steps of how many blocks.
const LANDING_SEARCH: i32 = 16;
const LANDING_STEP: i32 = 2;

/// The highest ground within `LANDING_SEARCH` blocks (the first block from `SCAN_UP` above it down that isn't air),
/// where it lands; none where nothing is loaded.
pub fn landing_spot(m: &Body, cx: &World<'_>, perch: fn(&str) -> bool) -> Option<[f64; 3]> {
    let (cx0, cz0) = (m.position.x.floor() as i32, m.position.z.floor() as i32);
    let top0 = m.position.y.floor() as i32 + SCAN_UP;
    let mut best: Option<(i32, i32, i32)> = None;
    for dx in (-LANDING_SEARCH..=LANDING_SEARCH).step_by(LANDING_STEP as usize) {
        for dz in (-LANDING_SEARCH..=LANDING_SEARCH).step_by(LANDING_STEP as usize) {
            let (x, z) = (cx0 + dx, cz0 + dz);
            let Some((y, name)) = (top0 - SCAN_DEPTH..=top0)
                .rev()
                .map(|y| (y, cx.block([x, y, z])))
                .find(|(_, name)| !matches!(name.as_str(), "minecraft:air" | "minecraft:cave_air" | ""))
            else {
                continue;
            };
            let top = y + 1;
            if perch(&name) && best.is_none_or(|b| top > b.1) {
                best = Some((x, top, z));
            }
        }
    }
    best.map(|(x, y, z)| [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5])
}
/// How far above and below it the landing spot is looked for (blocks).
const SCAN_UP: i32 = 8;
const SCAN_DEPTH: i32 = 48;

impl std::fmt::Debug for Soar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Soar")
            .field("phase", &self.phase)
            .field("ticks", &self.ticks)
            .finish()
    }
}

impl Soar {
    pub fn new(how: &'static Soaring, wings: &'static Wings) -> Self {
        Self {
            how,
            wings,
            flight: Flight::default(),
            phase: Phase::Descend,
            center: [0.0; 2],
            radius: 16.0,
            cruise_y: 0.0,
            clockwise: true,
            ticks: 0,
            circle_for: 0,
            landing: None,
            on: false,
        }
    }

    /// The heading that follows the circle, steering back towards its radius.
    fn circle_yaw(&self, m: &Body) -> f32 {
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
pub fn heading_of(x: f64, z: f64) -> f32 {
    (-x).atan2(z).to_degrees() as f32
}

impl Behaviour for Soar {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = if self.on {
            // Down, or a minute spent trying: it lets itself drop (no fall damage).
            let down = m.on_ground || self.how.lands_on_water && m.in_water;
            (self.how.idle)(m, cx)
                && !(self.phase == Phase::Descend && (down && self.ticks > 0 || self.ticks > GIVE_UP))
        } else {
            let resting = m.on_ground || self.how.lands_on_water && m.in_water;
            (self.how.idle)(m, cx) && (!resting || m.random.next_int_bound(self.how.take_off).unwrap_or(1) == 0)
        };
        if on && !self.on {
            self.start(m, cx);
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        self.soar(m, cx);
    }
}

impl Soar {
    fn start(&mut self, m: &mut Body, cx: &World<'_>) {
        self.flight = Flight::default();
        self.ticks = 0;
        self.landing = None;
        self.center = [m.position.x, m.position.z];
        let (radius, more) = self.how.radius;
        self.radius = radius + f64::from(m.random.next_int_bound(more + 1).unwrap_or(more / 2));
        self.clockwise = m.random.next_int_bound(2).unwrap_or(0) == 0;
        if m.on_ground || self.how.lands_on_water && m.in_water {
            self.phase = Phase::Climb;
            let (height, more) = self.how.height;
            self.cruise_y = m.position.y + height + f64::from(m.random.next_int_bound(more + 1).unwrap_or(more / 2));
            let (circle, longer) = self.how.circle_for;
            self.circle_for = circle + m.random.next_int_bound(longer + 1).unwrap_or(longer / 2);
        } else {
            // Left in the air: glide down from where it is.
            self.phase = Phase::Descend;
            self.cruise_y = m.position.y;
            self.flight.speed = self.wings.cruise * 0.6;
            self.landing = landing_spot(m, cx, self.how.perch);
        }
    }
    fn soar(&mut self, m: &mut Body, cx: &World<'_>) {
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
                } else if self.ticks > CLIMB_FOR {
                    // Held down (under the leaves of a tree, say): it gives up and glides down to a perch.
                    self.phase = Phase::Descend;
                    self.ticks = 0;
                    self.landing = landing_spot(m, cx, self.how.perch);
                }
                Controls {
                    yaw,
                    pitch: -MAX_CLIMB * 0.6,
                    flap: true,
                    brake: false,
                    boost: self.ticks < 10,
                    ridden: false,
                }
            }
            Phase::Circle => {
                if self.ticks > self.circle_for {
                    self.phase = Phase::Descend;
                    self.ticks = 0;
                    self.landing = landing_spot(m, cx, self.how.perch);
                }
                // Holds its height: noses up below it, down above it; flaps only to keep its speed or climb.
                let pitch = (-below * 3.0).clamp(-20.0, 20.0) as f32;
                Controls {
                    yaw,
                    pitch,
                    flap: below > 1.0 || self.flight.speed < self.wings.cruise * 0.6,
                    brake: false,
                    boost: false,
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
                        (
                            (-path * 3.0).clamp(-20.0, 35.0) as f32,
                            self.flight.speed > self.wings.cruise,
                        )
                    };
                    Controls {
                        yaw: heading_of(dx, dz),
                        pitch,
                        flap: path > 2.0,
                        brake,
                        boost: false,
                        ridden: false,
                    }
                }
                None => Controls {
                    yaw,
                    pitch: 12.0,
                    flap: false,
                    brake: self.flight.speed > self.wings.cruise,
                    boost: false,
                    ridden: false,
                },
            },
        };
        self.flight.tick(m, &controls, self.wings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_wrap_to_a_half_turn_each_way() {
        assert_eq!(wrap(190.0), -170.0);
        assert_eq!(wrap(-190.0), 170.0);
        assert_eq!(wrap(720.0 + 45.0), 45.0);
        assert_eq!(wrap(-180.0), -180.0);
    }

    #[test]
    fn a_heading_points_where_the_velocity_goes() {
        assert!(heading_of(0.0, 1.0).abs() < 1e-6);
        assert!((heading_of(-1.0, 0.0) - 90.0).abs() < 1e-4);
    }
}
